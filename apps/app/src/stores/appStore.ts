import { computed, reactive, ref } from "vue"
import type { EnvironmentInfo, LogEvent, ProgressEvent } from "@mobiletoolbox/protocol"
import type { ViewId } from "@mobiletoolbox/ui/types"
import { native } from "../platform/native"
import { errorText } from "../utils/errors"

/** 当前页面。保持模块级单例，视图切换不丢失状态。 */
export const activeView = ref<ViewId>("packages")

export const environment = ref<EnvironmentInfo | null>(null)
export const environmentLoading = ref(false)
export const environmentError = ref("")

export const logs = ref<LogEvent[]>([])
const MAX_LOG_LINES = 800

/** 正在运行的后台任务，key 为包 id / 文件名。 */
const running = reactive(new Map<string, ProgressEvent>())

/** 供日志面板展示的运行中任务列表。 */
export const runningList = computed(() => [...running.values()])

export const runningTasks = computed(() => runningList.value.length)

export function logLocal(source: string, text: string, stream: LogEvent["stream"] = "info"): void {
  logs.value.push({ time: Date.now(), source, stream, text })
  if (logs.value.length > MAX_LOG_LINES) logs.value.splice(0, logs.value.length - MAX_LOG_LINES)
}

export function pushLog(event: LogEvent): void {
  logs.value.push(event)
  if (logs.value.length > MAX_LOG_LINES) logs.value.splice(0, logs.value.length - MAX_LOG_LINES)
}

export function clearLogs(): void {
  logs.value = []
}

export function handleProgress(event: ProgressEvent): void {
  const key = event.id ?? event.file ?? event.stage
  if (event.state === "running") {
    running.set(key, event)
  } else {
    running.delete(key)
  }
}

/** 读取环境信息；失败时保留上一次结果，便于界面继续展示。 */
export async function loadEnvironment(): Promise<void> {
  if (!native.isTauri) {
    environmentError.value = "当前为浏览器预览模式，未连接本地工具链。"
    return
  }
  environmentLoading.value = true
  environmentError.value = ""
  try {
    environment.value = await native.environment()
  } catch (error) {
    environmentError.value = errorText(error)
    logLocal("环境", environmentError.value, "error")
  } finally {
    environmentLoading.value = false
  }
}