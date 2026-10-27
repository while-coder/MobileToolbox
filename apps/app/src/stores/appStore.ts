import { computed, reactive, ref } from "vue"
import type { EnvironmentInfo, ProgressEvent } from "@mobiletoolbox/protocol"
import type { ViewId } from "@mobiletoolbox/ui/types"
import { native } from "../platform/native"
import { errorText } from "../utils/errors"
import { log } from "../log"

/** 当前页面。保持模块级单例，视图切换不丢失状态。 */
export const activeView = ref<ViewId>("packages")

export const environment = ref<EnvironmentInfo | null>(null)
export const environmentLoading = ref(false)
export const environmentError = ref("")

/** 正在运行的后台任务，key 为包 id / 文件名。 */
const running = reactive(new Map<string, ProgressEvent>())

/** 当前运行中的任务，供导入和安装包进度展示。 */
export const runningList = computed(() => [...running.values()])

export const runningTasks = computed(() => runningList.value.length)

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
    log.error("环境", environmentError.value)
  } finally {
    environmentLoading.value = false
  }
}
