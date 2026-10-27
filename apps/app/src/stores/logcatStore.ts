import { ref, watch } from "vue"
import { native } from "../platform/native"
import { errorText } from "../utils/errors"
import { selectedDevice } from "./devicesStore"
import { confirm, toast } from "@qingfeng346/ui-kit"

export const lines = ref<string[]>([])
export const running = ref(false)
export const starting = ref(false)
export const stopping = ref(false)
export const error = ref("")
export const savedPath = ref("")
export const exporting = ref(false)
export const clearing = ref(false)
let generation = 0
let sessionId = ""
let pending: Promise<string> = Promise.resolve("")

export async function start(): Promise<void> {
  const deviceId = selectedDevice.value?.state === "device" ? selectedDevice.value.id : null
  if (!deviceId || starting.value || stopping.value || running.value || clearing.value) return
  if (!native.isTauri) { error.value = "请在桌面应用中查看 logcat。"; return }
  const token = ++generation
  error.value = ""
  starting.value = true
  try {
    sessionId = crypto.randomUUID()
    lines.value = []
    savedPath.value = ""
    running.value = true
    pending = native.startLogcat(deviceId, sessionId, event => {
      if (token !== generation) return
      if (event.lines.length) lines.value = [...lines.value, ...event.lines].slice(-5000)
      if (event.error) error.value = event.error
      if (event.finished) running.value = false
    })
    const path = await pending
    if (token === generation) savedPath.value = path
  } catch (cause) {
    if (token === generation) { error.value = errorText(cause); running.value = false }
  } finally {
    if (token === generation) starting.value = false
  }
}

export async function stop(): Promise<void> {
  if (stopping.value) return
  // Keep accepting the final buffered output until the native process has drained.
  const token = generation
  const stoppingSession = sessionId
  starting.value = false
  if (!running.value || !native.isTauri) { ++generation; return }
  stopping.value = true
  try {
    await pending.catch(() => undefined)
    await native.stopLogcat(stoppingSession)
  } catch (cause) { error.value = errorText(cause) }
  finally {
    if (token === generation) { ++generation; running.value = false }
    stopping.value = false
  }
}

export async function exportDisplayed(displayed: string[]): Promise<void> {
  if (exporting.value || clearing.value || !displayed.length) return
  const text = displayed.join("\n") + "\n"
  exporting.value = true
  try {
    const path = await native.pickLogcatFile()
    if (!path) return
    await native.exportLogcat(path, text)
    toast.show("success", "已导出当前筛选结果")
  } catch (cause) { error.value = errorText(cause) }
  finally { exporting.value = false }
}

export async function exportComplete(): Promise<void> {
  if (exporting.value || clearing.value || !savedPath.value) return
  const sourceSession = sessionId
  exporting.value = true
  try {
    const path = await native.pickLogcatFile()
    if (!path) return
    await native.exportLogcatSession(sourceSession, path)
    toast.show("success", "已导出完整日志（截至导出时的已采集内容）")
  } catch (cause) { error.value = errorText(cause) }
  finally { exporting.value = false }
}

export async function openCache(): Promise<void> {
  try { await native.openLogcatCache() }
  catch (cause) { error.value = errorText(cause) }
}

export async function clearCache(): Promise<void> {
  if (running.value || starting.value || stopping.value || exporting.value || clearing.value) return
  clearing.value = true
  try {
    if (!await confirm.show({ title: "清理日志缓存", content: "删除全部采集日志缓存？未导出的日志也会删除。", confirmText: "清理", error: true })) return
    await native.clearLogcatCache()
    savedPath.value = ""
    toast.show("success", "日志缓存已清理")
  } catch (cause) { error.value = errorText(cause) }
  finally { clearing.value = false }
}

watch(() => selectedDevice.value?.state === "device" ? selectedDevice.value.id : null,
  () => { void stop() })
