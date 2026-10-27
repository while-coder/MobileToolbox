import type { AndroidDevice, InstalledApp } from "@mobiletoolbox/protocol"
import { native, type CommandOptions, type CommandOutput } from "./native"

function combined(output: CommandOutput): string {
  return [output.stdout.trim(), output.stderr.trim()].filter(Boolean).join("\n")
}
function deviceArgs(deviceId: string, args: string[]): string[] {
  if (!/^[a-z\d._:-]{1,128}$/i.test(deviceId)) throw new Error("非法的设备标识")
  return ["-s", deviceId, ...args]
}
function command(deviceId: string, args: string[], options?: CommandOptions): Promise<CommandOutput> {
  return native.adbCommand(deviceArgs(deviceId, args), options)
}
function validPackage(name: string): boolean {
  return name.length <= 255 && /^[a-z\d_]+(?:\.[a-z\d_]+)+$/i.test(name)
}
function address(value: string): void {
  if (!value || value.length > 128 || !value.includes(":") || value.includes("..") || !/^[a-z\d.:_\[\]-]+$/i.test(value)) {
    throw new Error("连接地址格式不正确，应为 host:port")
  }
}
export function parsePackages(output: string): string[] {
  const packages = output.split(/\r?\n/).map(line => line.trim()).filter(Boolean).map(line => {
    const name = line.startsWith("package:") ? line.slice(8).trim() : ""
    if (name !== "android" && !validPackage(name)) throw new Error(`无法读取设备应用列表：${line}`)
    return name
  })
  return [...new Set(packages)].sort()
}
export function parseDevices(output: string): AndroidDevice[] {
  return output.split(/\r?\n/).map(line => line.trim()).filter(line => line && !line.startsWith("List of devices") && !line.startsWith("*")).flatMap(line => {
    const [id, state, ...details] = line.split(/\s+/)
    if (!id || !state || !/^[a-z\d._:-]{1,128}$/i.test(id)) return []
    const fields = Object.fromEntries(details.map(field => {
      const index = field.indexOf(":")
      return [field.slice(0, index), field.slice(index + 1)]
    }))
    return [{ id, state, model: fields.model ?? null, manufacturer: fields.manufacturer ?? null,
      androidRelease: null, sdkInt: null, product: fields.product ?? null }]
  })
}
async function devices(): Promise<AndroidDevice[]> {
  const devices = parseDevices((await native.adbCommand(["devices", "-l"])).stdout)
  for (const device of devices) {
    if (device.state !== "device") continue
    try {
      const output = await command(device.id, ["shell", "getprop"])
      const props = Object.fromEntries(output.stdout.split(/\r?\n/).flatMap(line => {
        const match = line.match(/^\[([^\]]+)\]: \[(.*)\]$/)
        return match ? [[match[1], match[2]]] : []
      }))
      device.model = props["ro.product.model"] ?? device.model
      device.manufacturer = props["ro.product.manufacturer"] ?? props["ro.product.brand"] ?? device.manufacturer
      device.androidRelease = props["ro.build.version.release"] ?? null
      device.sdkInt = props["ro.build.version.sdk"] ?? null
      device.product = props["ro.product.name"] ?? device.product
    } catch { /* Keep the device entry when property reads fail. */ }
  }
  return devices
}
async function applicationLabels(deviceId: string): Promise<Record<string, string>> {
  const { toolsDir } = await native.runtimePaths()
  const remote = `/data/local/tmp/mobiletoolbox-app-labels-${crypto.randomUUID()}.dex`
  try {
    await command(deviceId, ["push", `${toolsDir}/app-info/app-labels.dex`, remote])
    const output = await command(deviceId, ["shell", `CLASSPATH=${remote}`, "app_process", "/", "com.mobiletoolbox.tools.AppLabels"])
    const parsed: unknown = JSON.parse(output.stdout)
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {}
    return Object.fromEntries(Object.entries(parsed).filter((entry): entry is [string, string] => typeof entry[1] === "string" && Boolean(entry[1].trim())))
  } finally {
    await command(deviceId, ["shell", "rm", "-f", remote]).catch(() => undefined)
  }
}
async function installedApps(deviceId: string): Promise<InstalledApp[]> {
  const packages = parsePackages((await command(deviceId, ["shell", "pm", "list", "packages"])).stdout)
  const system = new Set(parsePackages((await command(deviceId, ["shell", "pm", "list", "packages", "-s"])).stdout))
  const labels = await applicationLabels(deviceId).catch(() => ({} as Record<string, string>))
  return packages.map(packageName => ({ packageName, appName: labels[packageName] ?? packageName, system: system.has(packageName) }))
}
async function install(deviceId: string, path: string): Promise<string> {
  const output = combined(await command(deviceId, ["install", "-r", path], { stream: true, source: "adb install" }))
  if (/failure|error/i.test(output) || !/success/i.test(output)) throw new Error(`安装结果未确认成功：${output}`)
  return output
}
export const adb = {
  devices,
  installedApps,
  install,
  async connect(value: string): Promise<string> {
    address(value)
    const output = combined(await native.adbCommand(["connect", value]))
    if (/failed|cannot|unable/i.test(output)) throw new Error(output || "adb connect 失败")
    return output
  },
  async disconnect(value: string): Promise<string> {
    address(value)
    return combined(await native.adbCommand(["disconnect", value]))
  },
  async installPackage(deviceId: string, id: string): Promise<string> {
    const { workDir } = await native.runtimePaths()
    if (!await native.packageFile(id, "isFile", "original.apk")) throw new Error("原始 APK 不存在")
    return install(deviceId, `${workDir}/${id}/original.apk`)
  },
  async uninstall(deviceId: string, packageName: string): Promise<string> {
    if (!validPackage(packageName)) throw new Error("包名格式不正确")
    const output = combined(await command(deviceId, ["uninstall", packageName], { stream: true, source: "adb uninstall" }))
    if (/failure|error/i.test(output) || !/success/i.test(output)) throw new Error(`卸载结果未确认成功：${output}`)
    return output
  },
  async pushObb(deviceId: string, path: string): Promise<string> {
    const filename = path.split(/[\\/]/).pop() ?? ""
    const match = filename.match(/^(?:main|patch)\.\d+\.(.+)\.obb$/)
    if (!match || !validPackage(match[1])) throw new Error("OBB 文件名应为 main.<版本号>.<包名>.obb 或 patch.<版本号>.<包名>.obb")
    const directory = `/sdcard/Android/obb/${match[1]}`
    await command(deviceId, ["shell", "mkdir", "-p", directory])
    const remote = `${directory}/${filename}`
    const output = combined(await command(deviceId, ["push", path, remote], { stream: true, source: "adb push" }))
    if (!/pushed|skipped/i.test(output)) throw new Error(`推送结果未确认成功：${output}`)
    return `${filename} → ${remote}`
  },
}
