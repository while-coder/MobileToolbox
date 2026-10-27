<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue"
import {
  ArrowDownToLine,
  ChevronDown,
  ChevronUp,
  Copy,
  Eraser,
  Maximize2,
  Minimize2,
  Search,
  Terminal,
} from "lucide-vue-next"
import { toast } from "@qingfeng346/ui-kit"
import { formatTime, type LogEvent, type ProgressEvent } from "@mobiletoolbox/protocol"

const props = withDefaults(defineProps<{
  lines: LogEvent[]
  /** 当前运行中的任务，来自 mobiletoolbox://progress */
  tasks?: ProgressEvent[]
}>(), { tasks: () => [] })

const emit = defineEmits<{ clear: [] }>()

type Filter = "all" | "info" | "warn" | "error"

const filters: { id: Filter; label: string }[] = [
  { id: "all", label: "全部" },
  { id: "info", label: "信息" },
  { id: "warn", label: "警告" },
  { id: "error", label: "错误" },
]

const expanded = ref(true)
const maximized = ref(false)
const filter = ref<Filter>("all")
const keyword = ref("")
const body = ref<HTMLElement | null>(null)
const stickToBottom = ref(true)

/** stderr 多为工具链的警告输出，归到「警告」；error 单独归类。 */
function levelOf(stream: LogEvent["stream"]): Filter {
  if (stream === "error") return "error"
  if (stream === "stderr") return "warn"
  return "info"
}

const filtered = computed(() => {
  const term = keyword.value.trim().toLowerCase()
  return props.lines.filter((line) => {
    if (filter.value !== "all" && levelOf(line.stream) !== filter.value) return false
    if (!term) return true
    return (
      line.text.toLowerCase().includes(term) ||
      line.source.toLowerCase().includes(term)
    )
  })
})

const taskRows = computed(() =>
  props.tasks.map((task) => {
    const ratio =
      typeof task.ratio === "number" && Number.isFinite(task.ratio) ? task.ratio : null
    return {
      key: task.id ?? task.file ?? task.stage,
      message: task.message,
      percent: ratio === null ? null : Math.max(0, Math.min(100, Math.round(ratio * 100))),
    }
  }),
)

const latest = computed(() => props.lines[props.lines.length - 1] ?? null)

function toggle(): void {
  expanded.value = !expanded.value
  if (!expanded.value) maximized.value = false
}

function toggleMaximize(): void {
  maximized.value = !maximized.value
  if (maximized.value) expanded.value = true
}

function onScroll(): void {
  const element = body.value
  if (!element) return
  stickToBottom.value = element.scrollHeight - element.scrollTop - element.clientHeight < 60
}

async function scrollToBottom(): Promise<void> {
  stickToBottom.value = true
  await nextTick()
  const element = body.value
  if (element) element.scrollTop = element.scrollHeight
}

watch(
  () => filtered.value.length,
  () => {
    if (expanded.value && stickToBottom.value) void scrollToBottom()
  },
)

watch(expanded, (value) => {
  if (value) void scrollToBottom()
})

watch(filter, () => {
  stickToBottom.value = true
  void scrollToBottom()
})

onMounted(() => {
  if (expanded.value) void scrollToBottom()
})

function legacyCopy(text: string): void {
  const area = document.createElement("textarea")
  area.value = text
  area.style.position = "fixed"
  area.style.opacity = "0"
  document.body.appendChild(area)
  area.select()
  document.execCommand("copy")
  document.body.removeChild(area)
}

async function copyLogs(): Promise<void> {
  if (filtered.value.length === 0) return
  const text = filtered.value
    .map((line) => `[${formatTime(line.time)}] [${line.source}] ${line.text}`)
    .join("\n")
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
    } else {
      legacyCopy(text)
    }
    toast.show("success", `已复制 ${filtered.value.length} 条日志`)
  } catch {
    toast.show("error", "复制日志失败")
  }
}
</script>

<template>
  <section class="log-console" :class="{ expanded, maximized }">
    <header class="log-bar" :title="expanded ? '收起运行日志' : '展开运行日志'" @click="toggle">
      <div class="bar-title">
        <Terminal :size="13" />
        <span class="bar-label">运行日志</span>
        <span v-if="lines.length" class="log-count">{{ lines.length }}</span>
        <span v-if="tasks.length" class="running">
          <span class="apk-spinner" />
          {{ tasks.length }} 个任务
        </span>
      </div>

      <div v-if="!expanded" class="bar-preview">
        <span v-if="tasks.length" class="preview-task">{{ tasks[0].message }}</span>
        <span v-else-if="latest" class="preview-line">
          <span class="preview-source">{{ latest.source }}</span>
          <span class="preview-text">{{ latest.text }}</span>
        </span>
        <span v-else class="preview-empty">暂无日志</span>
      </div>

      <div class="bar-actions" @click.stop>
        <SIconButton
          size="24"
          variant="ghost"
          title="复制当前筛选结果"
          :disabled="filtered.length === 0"
          @click="copyLogs"
        >
          <Copy :size="13" />
        </SIconButton>
        <SIconButton
          size="24"
          variant="ghost"
          title="清空日志"
          :disabled="lines.length === 0"
          @click="emit('clear')"
        >
          <Eraser :size="13" />
        </SIconButton>
        <SIconButton
          v-if="expanded"
          size="24"
          variant="ghost"
          :title="maximized ? '还原日志面板' : '最大化日志面板'"
          @click="toggleMaximize"
        >
          <Minimize2 v-if="maximized" :size="13" />
          <Maximize2 v-else :size="13" />
        </SIconButton>
        <SIconButton
          size="24"
          variant="ghost"
          :title="expanded ? '收起日志' : '展开日志'"
          @click="toggle"
        >
          <ChevronDown v-if="expanded" :size="14" />
          <ChevronUp v-else :size="14" />
        </SIconButton>
      </div>
    </header>

    <div v-show="expanded" class="log-panel">
      <div class="log-toolbar">
        <div class="filters" role="tablist" aria-label="日志级别">
          <button
            v-for="item in filters"
            :key="item.id"
            type="button"
            class="filter"
            :class="{ active: filter === item.id }"
            role="tab"
            :aria-selected="filter === item.id"
            @click="filter = item.id"
          >
            {{ item.label }}
          </button>
        </div>

        <div class="search-wrap">
          <Search class="search-icon" :size="12" />
          <SInput
            v-model:value="keyword"
            size="small"
            clearable
            aria-label="筛选日志"
            placeholder="筛选日志…"
            spellcheck="false"
          />
        </div>

        <span class="filtered-count">{{ filtered.length }} / {{ lines.length }}</span>

        <SIconButton size="22" variant="ghost" title="回到底部" @click="scrollToBottom">
          <ArrowDownToLine :size="13" />
        </SIconButton>
      </div>

      <div v-if="taskRows.length" class="log-tasks">
        <div v-for="task in taskRows" :key="task.key" class="task-row">
          <div class="task-line">
            <span class="apk-spinner" />
            <span class="task-text" :title="task.message">{{ task.message }}</span>
            <span class="task-percent">{{ task.percent === null ? "" : `${task.percent}%` }}</span>
          </div>
          <div class="task-track">
            <span
              class="task-fill"
              :class="{ indeterminate: task.percent === null }"
              :style="task.percent === null ? undefined : { width: `${task.percent}%` }"
            />
          </div>
        </div>
      </div>

      <div ref="body" class="log-body" @scroll="onScroll">
        <div v-if="lines.length === 0" class="log-empty">
          暂无日志。导入、解包、反编译或设备操作时，命令行输出会显示在这里。
        </div>
        <div v-else-if="filtered.length === 0" class="log-empty">没有匹配的日志。</div>
        <div
          v-for="(line, index) in filtered"
          :key="index"
          class="log-line"
          :class="levelOf(line.stream)"
        >
          <span class="log-time">{{ formatTime(line.time) }}</span>
          <span class="log-source" :title="line.source">{{ line.source }}</span>
          <span class="log-text">{{ line.text }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/*
 * 折叠时只占 34px 状态条；展开时以绝对定位向上覆盖内容区，
 * 由 AppShell 的 footer-dock 提供定位上下文，因此不会挤压主内容。
 */
.log-console {
  position: absolute;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 20;
  display: flex;
  height: 34px;
  flex-direction: column;
  border-top: 1px solid var(--apk-border);
  background: #07101d;
  box-shadow: 0 -14px 34px rgba(2, 8, 18, 0.36);
  transition: height 0.16s ease;
}
.log-console.expanded {
  height: 320px;
}
.log-console.maximized {
  height: 72vh;
}

.log-bar {
  display: flex;
  height: 34px;
  flex: 0 0 auto;
  align-items: center;
  gap: 12px;
  padding: 0 10px 0 14px;
  border-bottom: 1px solid transparent;
  cursor: pointer;
}
.log-console.expanded .log-bar {
  border-bottom-color: var(--apk-border);
}
.bar-title {
  display: flex;
  min-width: 0;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
  color: var(--apk-muted);
  font-size: 11px;
  font-weight: 600;
}
.bar-title > svg {
  color: var(--apk-accent);
}
.bar-label {
  white-space: nowrap;
}
.log-count {
  padding: 0 5px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
  color: var(--apk-faint);
  font-family: var(--apk-mono);
  font-size: 9.5px;
  line-height: 15px;
}
.running {
  display: flex;
  align-items: center;
  gap: 5px;
  color: #7dd3fc;
  font-size: 10px;
  font-weight: 400;
  white-space: nowrap;
}
.running .apk-spinner {
  width: 9px;
  height: 9px;
}
.bar-preview {
  display: flex;
  min-width: 0;
  flex: 1;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  color: var(--apk-faint);
  font-family: var(--apk-mono);
  font-size: 10px;
  white-space: nowrap;
}
.preview-task {
  overflow: hidden;
  color: #7dd3fc;
  text-overflow: ellipsis;
}
.preview-line {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 6px;
}
.preview-source {
  flex: 0 0 auto;
  color: #6b819c;
}
.preview-text {
  overflow: hidden;
  text-overflow: ellipsis;
}
.preview-empty {
  opacity: 0.7;
}
.bar-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 1px;
  margin-left: auto;
}

.log-panel {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
}

.log-toolbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  padding: 6px 10px 6px 14px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
}
.filters {
  display: flex;
  flex: 0 0 auto;
  gap: 2px;
  padding: 2px;
  border-radius: 7px;
  background: rgba(148, 163, 184, 0.07);
}
.filter {
  padding: 2px 8px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--apk-muted);
  cursor: pointer;
  font-size: 10.5px;
  line-height: 16px;
  transition: background 0.14s ease, color 0.14s ease;
}
.filter:hover {
  color: var(--apk-text);
}
.filter.active {
  background: rgba(56, 189, 248, 0.16);
  color: #7dd3fc;
}
.filter:focus-visible {
  outline: 1px solid var(--apk-accent);
  outline-offset: -1px;
}
.search-wrap {
  display: flex;
  width: min(220px, 32%);
  align-items: center;
  gap: 5px;
}
.search-icon {
  flex: 0 0 auto;
  color: var(--apk-faint);
}
.filtered-count {
  margin-left: auto;
  color: var(--apk-faint);
  font-family: var(--apk-mono);
  font-size: 9.5px;
  white-space: nowrap;
}

.log-tasks {
  max-height: 84px;
  flex: 0 0 auto;
  overflow-y: auto;
  padding: 5px 14px 6px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
}
.task-row + .task-row {
  margin-top: 5px;
}
.task-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10.5px;
}
.task-line .apk-spinner {
  width: 9px;
  height: 9px;
  flex: 0 0 auto;
}
.task-text {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--apk-text-soft);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.task-percent {
  flex: 0 0 auto;
  color: #7dd3fc;
  font-family: var(--apk-mono);
  font-size: 9.5px;
  font-variant-numeric: tabular-nums;
}
.task-track {
  height: 3px;
  margin-top: 3px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(56, 189, 248, 0.12);
}
.task-fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #38bdf8, #818cf8);
  transition: width 0.18s ease;
}
.task-fill.indeterminate {
  width: 34%;
  animation: task-slide 1.1s ease-in-out infinite;
}
@keyframes task-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(320%); }
}

.log-body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 4px 0 8px;
  font-family: var(--apk-mono);
  font-size: 10.5px;
  line-height: 1.55;
}
.log-empty {
  padding: 16px 14px;
  color: var(--apk-faint);
  font-family: var(--apk-font);
  font-size: 10.5px;
}
.log-line {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 1px 14px;
  color: #a9bace;
}
.log-line:hover {
  background: rgba(148, 163, 184, 0.04);
}
.log-line.warn {
  color: #fbbf24;
}
.log-line.error {
  background: rgba(251, 113, 133, 0.05);
  color: #fda4af;
}
.log-line.info {
  color: #7dd3fc;
}
.log-time {
  flex: 0 0 auto;
  color: #52657d;
  font-variant-numeric: tabular-nums;
}
.log-source {
  width: 54px;
  flex: 0 0 auto;
  overflow: hidden;
  color: #6b819c;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.log-text {
  min-width: 0;
  flex: 1;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
