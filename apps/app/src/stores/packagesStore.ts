import { computed, ref } from "vue"
import { confirm, toast } from "@qingfeng346/ui-kit"
import type { PackageInfo, PackageStage } from "@mobiletoolbox/protocol"
import { native } from "../platform/native"
import * as packageCommands from "../platform/packageCommands"
import { errorText } from "../utils/errors"
import { activeView, handleProgress } from "./appStore"
import { log } from "../log"

export const packages = ref<PackageInfo[]>([])
export const selectedId = ref<string | null>(null)
export const icons = ref<Record<string, string>>({})
export const busy = ref<Record<string, PackageStage>>({})
export const query = ref("")
export const loading = ref(false)
export const importing = ref(false)

export const selectedPackage = computed(
  () => packages.value.find((pkg) => pkg.id === selectedId.value) ?? null,
)
export const busyStage = computed<PackageStage>(
  () => (selectedId.value ? busy.value[selectedId.value] ?? "idle" : "idle"),
)

function sortPackages(): void {
  packages.value.sort((left, right) => right.importedAt - left.importedAt)
}

function upsert(pkg: PackageInfo): void {
  const index = packages.value.findIndex((item) => item.id === pkg.id)
  if (index >= 0) packages.value[index] = pkg
  else packages.value.push(pkg)
}

export function iconFor(id: string | null): string {
  return id ? icons.value[id] ?? "" : ""
}

export function setBusy(id: string, stage: PackageStage | null): void {
  const next = { ...busy.value }
  if (stage && stage !== "idle") next[id] = stage
  else delete next[id]
  busy.value = next
}

export function selectPackage(id: string): void {
  selectedId.value = id
}

/** 图标按需解码成 data URL，限制并发避免一次导入大量 APK 时卡住。 */
async function hydrateIcons(ids: string[]): Promise<void> {
  const queue = ids.filter((id) => id && !icons.value[id])
  if (queue.length === 0) return
  const workers = Array.from({ length: Math.min(4, queue.length) }, async () => {
    for (;;) {
      const id = queue.shift()
      if (!id) return
      try {
        const url = await native.getPackageIcon(id)
        if (url) icons.value = { ...icons.value, [id]: url }
      } catch {
        // 图标只是装饰，读取失败时交给字母头像兜底
      }
    }
  })
  await Promise.all(workers)
}

export async function loadPackages(): Promise<void> {
  if (!native.isTauri) return
  loading.value = true
  try {
    const list = await native.listPackages()
    packages.value = list
    sortPackages()
    if (!selectedId.value || !packages.value.some((pkg) => pkg.id === selectedId.value)) {
      selectedId.value = packages.value[0]?.id ?? null
    }
    void hydrateIcons(packages.value.map((pkg) => pkg.id))
  } catch (error) {
    toast.show("error", `读取安装包库失败：${errorText(error)}`)
  } finally {
    loading.value = false
  }
}

/** 返回成功导入的数量，调用方据此决定是否切换页面。 */
export async function importFromPaths(paths: string[]): Promise<number> {
  if (!native.isTauri) {
    toast.show("warning", "浏览器预览模式不支持导入，请运行桌面端。")
    return 0
  }
  const apks = paths.filter((path) => path.toLowerCase().endsWith(".apk"))
  if (apks.length === 0) {
    toast.show("warning", "请选择 .apk 文件。")
    return 0
  }
  if (importing.value) return 0

  importing.value = true
  try {
    const outcome = await native.importPackages(apks)
    for (const pkg of outcome.packages) upsert(pkg)
    sortPackages()

    if (outcome.packages.length > 0) {
      selectedId.value = outcome.packages[0].id
      activeView.value = "packages"
      void hydrateIcons(outcome.packages.map((pkg) => pkg.id))
      toast.show(
        "success",
        outcome.packages.length === 1
          ? `已导入「${outcome.packages[0].appName}」`
          : `已导入 ${outcome.packages.length} 个安装包`,
      )
    }

    for (const failure of outcome.errors) {
      toast.show("error", `${failure.file}：${failure.message}`, 7000)
    }
    const writeLog = outcome.errors.length > 0 ? log.error : log.info
    writeLog(
      "导入",
      `完成：成功 ${outcome.packages.length} 个，失败 ${outcome.errors.length} 个`,
    )
    return outcome.packages.length
  } catch (error) {
    toast.show("error", `导入失败：${errorText(error)}`, 7000)
    return 0
  } finally {
    importing.value = false
  }
}

export async function pickAndImport(): Promise<void> {
  try {
    const paths = await native.pickApkFiles()
    if (paths.length > 0) await importFromPaths(paths)
  } catch (error) {
    toast.show("error", `打开文件选择器失败：${errorText(error)}`)
  }
}

export async function decodePackage(pkg: PackageInfo): Promise<void> {
  if (busy.value[pkg.id]) return
  setBusy(pkg.id, "decoding")
  try {
    const updated = await packageCommands.decodePackage(pkg.id, handleProgress)
    upsert(updated)
    toast.show("success", `「${updated.appName}」资源已解包`)
  } catch (error) {
    toast.show("error", `解包失败：${errorText(error)}`, 8000)
  } finally {
    setBusy(pkg.id, null)
  }
}

export async function buildJar(pkg: PackageInfo): Promise<void> {
  if (busy.value[pkg.id]) return
  setBusy(pkg.id, "jar")
  try {
    const updated = await packageCommands.buildJar(pkg.id, handleProgress)
    upsert(updated)
    toast.show("success", `已生成 source.jar`)
  } catch (error) {
    toast.show("error", `生成 JAR 失败：${errorText(error)}`, 8000)
  } finally {
    setBusy(pkg.id, null)
  }
}

export async function decompilePackage(pkg: PackageInfo): Promise<void> {
  if (busy.value[pkg.id]) return
  setBusy(pkg.id, "decompiling")
  try {
    const updated = await packageCommands.decompilePackage(pkg.id, handleProgress)
    upsert(updated)
    toast.show("success", `「${updated.appName}」Java 源码已就绪`)
  } catch (error) {
    toast.show("error", `反编译失败：${errorText(error)}`, 8000)
  } finally {
    setBusy(pkg.id, null)
  }
}

export async function removePackage(pkg: PackageInfo): Promise<void> {
  const confirmed = await confirm.show({
    title: "删除安装包",
    content: `确定从工作目录中删除「${pkg.appName}」吗？解包结果与反编译源码会一并删除，且不可恢复。`,
    confirmText: "删除",
    error: true,
  })
  if (!confirmed) return

  setBusy(pkg.id, "removing")
  try {
    await native.removePackage(pkg.id)
    packages.value = packages.value.filter((item) => item.id !== pkg.id)
    if (icons.value[pkg.id]) {
      const next = { ...icons.value }
      delete next[pkg.id]
      icons.value = next
    }
    if (selectedId.value === pkg.id) selectedId.value = packages.value[0]?.id ?? null
    toast.show("success", "已删除")
  } catch (error) {
    toast.show("error", `删除失败：${errorText(error)}`, 7000)
  } finally {
    setBusy(pkg.id, null)
  }
}

export async function clearAll(): Promise<void> {
  if (packages.value.length === 0) {
    toast.show("info", "安装包库已经是空的。")
    return
  }
  const confirmed = await confirm.show({
    title: "清空安装包库",
    content: `将删除全部 ${packages.value.length} 个安装包及其解包/反编译结果，此操作不可恢复。`,
    confirmText: "全部删除",
    error: true,
  })
  if (!confirmed) return

  try {
    await native.clearPackages()
    packages.value = []
    icons.value = {}
    selectedId.value = null
    toast.show("success", "安装包库已清空")
  } catch (error) {
    toast.show("error", `清空失败：${errorText(error)}`, 7000)
  }
}
