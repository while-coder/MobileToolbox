import { computed, ref } from "vue"
import { confirm, toast } from "@qingfeng346/ui-kit"
import { localizedLabel, type AndroidDevice, type PackageInfo } from "@mobiletoolbox/protocol"
import { native } from "../platform/native"
import { baseName, errorText } from "../utils/errors"

export const devices = ref<AndroidDevice[]>([])
export const selectedDeviceId = ref<string | null>(null)
export const loading = ref(false)
export const connecting = ref(false)
export const busyText = ref("")

export const selectedDevice = computed(
  () => devices.value.find((device) => device.id === selectedDeviceId.value) ?? null,
)
export const onlineCount = computed(() => devices.value.filter((device) => device.state === "device").length)

export function selectDevice(id: string): void {
  selectedDeviceId.value = id
}

function requireOnline(deviceId: string): AndroidDevice | null {
  const device = devices.value.find((item) => item.id === deviceId)
  if (!device) {
    toast.show("warning", "设备已断开，请刷新设备列表。")
    return null
  }
  if (device.state !== "device") {
    toast.show("warning", "该设备当前不在线，请检查 USB 调试或模拟器状态。")
    return null
  }
  return device
}

export async function refreshDevices(quiet = false): Promise<void> {
  if (!native.isTauri) return
  loading.value = true
  try {
    const list = await native.adbDevices()
    devices.value = list
    const keep = selectedDeviceId.value
    if (!keep || !list.some((device) => device.id === keep)) {
      selectedDeviceId.value = (list.find((device) => device.state === "device") ?? list[0])?.id ?? null
    }
  } catch (error) {
    if (!quiet) toast.show("error", `读取设备列表失败：${errorText(error)}`, 7000)
  } finally {
    loading.value = false
  }
}

export async function connectDevice(address: string): Promise<void> {
  if (!native.isTauri) return
  if (connecting.value) return
  connecting.value = true
  try {
    const output = await native.adbConnect(address)
    toast.show("success", output.trim() || `已连接 ${address}`)
    await refreshDevices(true)
  } catch (error) {
    toast.show("error", `连接失败：${errorText(error)}`, 8000)
  } finally {
    connecting.value = false
  }
}

export async function disconnectDevice(address: string): Promise<void> {
  try {
    const output = await native.adbDisconnect(address)
    toast.show("info", output.trim() || `已断开 ${address}`)
    await refreshDevices(true)
  } catch (error) {
    toast.show("error", `断开失败：${errorText(error)}`, 7000)
  }
}

async function runTask<T>(text: string, task: () => Promise<T>): Promise<T | null> {
  busyText.value = text
  try {
    return await task()
  } catch (error) {
    toast.show("error", errorText(error), 9000)
    return null
  } finally {
    busyText.value = ""
  }
}

/** 安装单个 APK，成功返回 true。 */
export async function installApk(deviceId: string, apkPath: string): Promise<boolean> {
  if (!requireOnline(deviceId)) return false
  const output = await runTask(`正在安装 ${baseName(apkPath)}…`, () =>
    native.adbInstall(deviceId, apkPath),
  )
  if (output === null) return false
  toast.show("success", output.trim() || "安装完成", 5000)
  return true
}

/** 拖拽多个 APK 时按顺序安装，失败不中断后续文件。 */
export async function installPaths(deviceId: string, paths: string[]): Promise<void> {
  if (!requireOnline(deviceId)) return
  let succeeded = 0
  for (const path of paths) {
    const output = await runTask(`正在安装 ${baseName(path)}…`, () =>
      native.adbInstall(deviceId, path),
    )
    if (output !== null) succeeded += 1
  }
  if (succeeded > 0) {
    toast.show("success", `已安装 ${succeeded}/${paths.length} 个安装包`)
  }
}

export async function installPackage(deviceId: string, pkg: PackageInfo): Promise<void> {
  if (!requireOnline(deviceId)) return
  const label = localizedLabel(pkg, typeof navigator === "undefined" ? "zh-CN" : navigator.language)
  const output = await runTask(`正在安装「${label}」…`, () =>
    native.adbInstallPackage(deviceId, pkg.id),
  )
  if (output === null) return
  toast.show("success", output.trim() || `「${label}」安装完成`, 5000)
}

export async function pickAndInstall(deviceId: string): Promise<void> {
  if (!requireOnline(deviceId)) return
  try {
    const paths = await native.pickApkFile()
    if (paths[0]) await installApk(deviceId, paths[0])
  } catch (error) {
    toast.show("error", `打开文件选择器失败：${errorText(error)}`)
  }
}

export async function pushObb(deviceId: string, filePath: string): Promise<void> {
  if (!requireOnline(deviceId)) return
  const output = await runTask(`正在推送 ${baseName(filePath)}…`, () =>
    native.adbPushObb(deviceId, filePath),
  )
  if (output === null) return
  toast.show("success", output.trim() || "OBB 已推送到设备", 5000)
}

export async function pickAndPushObb(deviceId: string): Promise<void> {
  if (!requireOnline(deviceId)) return
  try {
    const paths = await native.pickObbFile()
    if (paths[0]) await pushObb(deviceId, paths[0])
  } catch (error) {
    toast.show("error", `打开文件选择器失败：${errorText(error)}`)
  }
}

export async function uninstallPackage(deviceId: string, packageName: string): Promise<void> {
  if (!requireOnline(deviceId)) return
  const confirmed = await confirm.show({
    title: "卸载应用",
    content: `确定从设备上卸载「${packageName}」吗？应用数据会一并清除。`,
    confirmText: "卸载",
    error: true,
  })
  if (!confirmed) return
  const output = await runTask(`正在卸载 ${packageName}…`, () =>
    native.adbUninstall(deviceId, packageName),
  )
  if (output === null) return
  toast.show("success", output.trim() || "已卸载", 5000)
}