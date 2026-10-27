import type { PackageInfo, ProgressEvent } from "@mobiletoolbox/protocol"
import { native } from "./native"

type Progress = (event: ProgressEvent) => void

async function runStage(id: string, stage: string, message: string, done: string, progress: Progress, task: () => Promise<void>): Promise<PackageInfo> {
  const notify = (state: ProgressEvent["state"], message: string) => progress({ id, file: null, stage, message, state, ratio: state === "done" ? 1 : null })
  notify("running", message)
  try {
    await task()
    const info = await native.getPackageInfo(id)
    notify("done", done)
    return info
  } catch (error) {
    notify("error", String(error))
    throw error
  }
}

async function replaceOutput(id: string, temporary: string, target: string): Promise<void> {
  await native.packageFile(id, "remove", target)
  if (!await native.packageFile(id, "rename", temporary, target)) throw new Error(`未生成 ${temporary}`)
}

export function decodePackage(id: string, progress: Progress): Promise<PackageInfo> {
  return runStage(id, "decode", "正在解包资源（apktool）", "资源解包完成", progress, async () => {
    const { workDir, toolsDir } = await native.runtimePaths()
    if (!await native.packageFile(id, "isFile", "original.apk")) throw new Error("原始 APK 不存在，无法解包")
    const dir = `${workDir}/${id}`
    await native.packageFile(id, "remove", "decoded.tmp")
    try {
      await native.javaCommand(["-jar", `${toolsDir}/apktool/apktool.jar`, "d", "-f", "-o", `${dir}/decoded.tmp`, `${dir}/original.apk`], { stream: true, source: "apktool" })
      if (!await native.packageFile(id, "isFile", "decoded.tmp/AndroidManifest.xml")) throw new Error("apktool 已结束，但没有生成 AndroidManifest.xml")
      await replaceOutput(id, "decoded.tmp", "decoded")
    } finally {
      await native.packageFile(id, "remove", "decoded.tmp").catch(() => undefined)
    }
  })
}

export function buildJar(id: string, progress: Progress): Promise<PackageInfo> {
  return runStage(id, "jar", "正在生成 source.jar（dex2jar）", "source.jar 生成完成", progress, async () => {
    const { workDir, toolsDir } = await native.runtimePaths()
    if (!await native.packageFile(id, "isFile", "original.apk")) throw new Error("原始 APK 不存在")
    const dir = `${workDir}/${id}`
    await native.packageFile(id, "remove", "source.tmp.jar")
    try {
      await native.javaCommand(["-Xms512m", "-Xmx2048m", "-cp", "lib/*", "com.googlecode.dex2jar.tools.Dex2jarCmd", "-f", `${dir}/original.apk`, "-o", `${dir}/source.tmp.jar`], { stream: true, source: "dex2jar", cwd: `${toolsDir}/dex-tools` })
      if (!await native.packageFile(id, "isFile", "source.tmp.jar")) throw new Error("dex2jar 已结束，但没有生成 source.jar")
      await replaceOutput(id, "source.tmp.jar", "source.jar")
    } finally {
      await native.packageFile(id, "remove", "source.tmp.jar").catch(() => undefined)
    }
  })
}

export async function decompilePackage(id: string, progress: Progress): Promise<PackageInfo> {
  if (!await native.packageFile(id, "isFile", "source.jar")) await buildJar(id, progress)
  return runStage(id, "decompile", "正在反编译 Java 源码（CFR）", "Java 源码已就绪", progress, async () => {
    const { workDir, toolsDir } = await native.runtimePaths()
    const dir = `${workDir}/${id}`
    await native.packageFile(id, "remove", "java.tmp")
    try {
      await native.javaCommand(["-jar", `${toolsDir}/cfr.jar`, `${dir}/source.jar`, "--outputdir", `${dir}/java.tmp`, "--silent", "true"], { stream: true, source: "cfr" })
      if (!await native.packageFile(id, "hasEntries", "java.tmp")) throw new Error("CFR 已结束，但没有生成任何 Java 文件")
      await replaceOutput(id, "java.tmp", "java")
    } finally {
      await native.packageFile(id, "remove", "java.tmp").catch(() => undefined)
    }
  })
}
