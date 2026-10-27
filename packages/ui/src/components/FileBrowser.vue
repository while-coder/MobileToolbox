<script setup lang="ts">
import ResizeHandle from "./ResizeHandle.vue"
import { computed, onBeforeUnmount, onMounted, ref } from "vue"
import {
  AlertCircle,
  ChevronRight,
  File,
  FileArchive,
  FileCode2,
  FileText,
  Folder,
  FolderOpen,
  FolderTree,
  Image as ImageIcon,
  RefreshCw,
  Search,
} from "lucide-vue-next"
import {
  formatBytes,
  type BrowserRoot,
  type FileEntry,
  type LoadDirectory,
  type LoadImage,
  type LoadText,
  type TextPreview,
} from "@mobiletoolbox/protocol"
import { toast } from "@qingfeng346/ui-kit"

const props = defineProps<{
  root: BrowserRoot
  loadDirectory: LoadDirectory
  loadText: LoadText
  loadImage: LoadImage
}>()

const emit = defineEmits<{ "open-folder": [] }>()

interface Row {
  entry: FileEntry
  depth: number
}

const rootEntries = ref<FileEntry[]>([])
const children = ref<Record<string, FileEntry[]>>({})
const expanded = ref<Set<string>>(new Set())
const loadingDirs = ref<Set<string>>(new Set())
const rootLoading = ref(true)
const rootError = ref("")
const rootTruncated = ref(false)
const query = ref("")
const searchTerm = computed(() => query.value.trim().toLowerCase())
let directoryGeneration = 0

const activeFile = ref<FileEntry | null>(null)
const previewState = ref<"empty" | "loading" | "text" | "image" | "binary" | "error">("empty")
const previewText = ref<TextPreview | null>(null)
const previewImage = ref("")
const previewError = ref("")
let previewToken = 0

const rootLabel = computed(() => (props.root === "java" ? "Java 源码" : "解包资源"))

function errorText(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error ?? "未知错误")
}

function flatten(entries: FileEntry[], depth: number, output: Row[]): void {
  for (const entry of entries) {
    const nested: Row[] = []
    const loaded = children.value[entry.relativePath]
    if (entry.directory && loaded && expanded.value.has(entry.relativePath)) {
      flatten(loaded, depth + 1, nested)
    }
    if (searchTerm.value && !entry.relativePath.toLowerCase().includes(searchTerm.value) && nested.length === 0) continue
    output.push({ entry, depth })
    for (const row of nested) output.push(row)
  }
}

const rows = computed(() => {
  const output: Row[] = []
  flatten(rootEntries.value, 0, output)
  return output
})

async function loadRoot(): Promise<void> {
  rootLoading.value = true
  rootError.value = ""
  try {
    const listing = await props.loadDirectory(props.root, "")
    rootEntries.value = listing.entries
    rootTruncated.value = listing.truncated
  } catch (error) {
    rootError.value = errorText(error)
  } finally {
    rootLoading.value = false
  }
}

async function loadDirectory(relativePath: string): Promise<void> {
  const generation = directoryGeneration
  const pending = new Set(loadingDirs.value)
  pending.add(relativePath)
  loadingDirs.value = pending
  try {
    const listing = await props.loadDirectory(props.root, relativePath)
    if (generation !== directoryGeneration) return
    children.value = { ...children.value, [relativePath]: listing.entries }
    if (listing.truncated) toast.show("warning", "目录条目过多，仅显示前 2000 项")
  } catch (error) {
    if (generation !== directoryGeneration) return
    toast.show("error", `读取目录失败：${errorText(error)}`)
  } finally {
    if (generation === directoryGeneration) {
      const next = new Set(loadingDirs.value)
      next.delete(relativePath)
      loadingDirs.value = next
    }
  }
}

async function toggleDirectory(entry: FileEntry): Promise<void> {
  const path = entry.relativePath
  const next = new Set(expanded.value)
  if (next.has(path)) {
    next.delete(path)
    expanded.value = next
    return
  }
  next.add(path)
  expanded.value = next
  if (!children.value[path]) await loadDirectory(path)
}

async function selectFile(entry: FileEntry): Promise<void> {
  activeFile.value = entry
  previewText.value = null
  previewImage.value = ""
  previewError.value = ""
  const token = ++previewToken

  if (entry.kind === "binary") {
    previewState.value = "binary"
    return
  }

  previewState.value = "loading"
  try {
    if (entry.kind === "image") {
      const url = await props.loadImage(props.root, entry.relativePath)
      if (token !== previewToken) return
      previewImage.value = url
      previewState.value = "image"
    } else {
      const text = await props.loadText(props.root, entry.relativePath)
      if (token !== previewToken) return
      previewText.value = text
      previewState.value = "text"
    }
  } catch (error) {
    if (token !== previewToken) return
    previewError.value = errorText(error)
    previewState.value = "error"
  }
}

function onEntryClick(entry: FileEntry): void {
  if (entry.directory) void toggleDirectory(entry)
  else void selectFile(entry)
}

function isExpanded(entry: FileEntry): boolean {
  return expanded.value.has(entry.relativePath)
}

function isSelected(entry: FileEntry): boolean {
  return activeFile.value?.relativePath === entry.relativePath
}

function iconFor(entry: FileEntry | null) {
  if (!entry) return File
  if (entry.directory) return isExpanded(entry) ? FolderOpen : Folder
  switch (entry.extension) {
    case "java":
    case "kt":
    case "kts":
    case "smali":
    case "xml":
    case "json":
    case "yml":
    case "yaml":
    case "properties":
    case "gradle":
    case "pro":
    case "js":
    case "ts":
    case "vue":
    case "html":
    case "css":
      return FileCode2
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "bmp":
    case "ico":
    case "svg":
      return ImageIcon
    case "zip":
    case "apk":
    case "jar":
    case "dex":
    case "so":
      return FileArchive
    case "txt":
    case "md":
    case "log":
      return FileText
    default:
      return File
  }
}

async function refresh(): Promise<void> {
  ++directoryGeneration
  loadingDirs.value = new Set()
  children.value = {}
  expanded.value = new Set()
  activeFile.value = null
  previewState.value = "empty"
  previewText.value = null
  previewImage.value = ""
  await loadRoot()
}

onMounted(() => {
  void loadRoot()
})
onBeforeUnmount(() => { ++directoryGeneration })
</script>

<template>
  <section class="file-browser">
    <header class="fb-toolbar">
      <div class="root-label">
        <FolderTree :size="14" />
        <span>{{ rootLabel }}</span>
        <SIconButton size="24" variant="ghost" title="刷新目录" @click="refresh">
          <RefreshCw :size="13" />
        </SIconButton>
      </div>
      <span class="spacer" />
      <SButton size="small" @click="emit('open-folder')">
        <FolderOpen :size="14" />
        打开目录
      </SButton>
    </header>

    <div class="fb-body">
      <div class="tree-column">
        <ResizeHandle label="调整文件目录宽度" :min="160" :remaining="200" />
        <div class="tree-search">
          <Search :size="14" aria-hidden="true" />
          <SInput v-model:value="query" size="small" clearable class="tree-search-input"
            aria-label="过滤文件名或路径" placeholder="过滤文件名或路径…" />
        </div>
        <div class="tree-pane">
          <div v-if="rootLoading" class="tree-state">
            <span class="apk-spinner" />
            正在读取目录…
          </div>
          <div v-else-if="rootError" class="tree-state error">
            <AlertCircle :size="18" />
            <span>{{ rootError }}</span>
            <SButton size="small" @click="loadRoot">重试</SButton>
          </div>
          <div v-else-if="rows.length === 0" class="tree-state">
            {{ searchTerm ? '没有匹配的文件或目录' : '目录为空' }}
          </div>
          <template v-else>
            <div v-if="rootTruncated" class="tree-notice">条目过多，仅显示前 2000 项</div>
            <button
              v-for="row in rows"
              :key="row.entry.relativePath"
              type="button"
              class="tree-row"
              :class="{ selected: isSelected(row.entry), directory: row.entry.directory }"
              :style="{ paddingLeft: `${8 + row.depth * 14}px` }"
              :title="row.entry.relativePath"
              @click="onEntryClick(row.entry)"
            >
              <span class="chevron" :class="{ open: row.entry.directory && isExpanded(row.entry) }">
                <ChevronRight v-if="row.entry.directory" :size="12" />
              </span>
              <component :is="iconFor(row.entry)" :size="14" class="entry-icon" />
              <span class="entry-name">{{ row.entry.name }}</span>
              <span v-if="loadingDirs.has(row.entry.relativePath)" class="apk-spinner tiny" />
              <span v-else-if="!row.entry.directory" class="entry-size">{{ formatBytes(row.entry.size) }}</span>
            </button>
          </template>
        </div>

      </div>
      <div class="preview-pane">
        <header v-if="activeFile" class="preview-head">
          <component :is="iconFor(activeFile)" :size="15" class="entry-icon" />
          <div class="preview-title">
            <strong>{{ activeFile.name }}</strong>
            <span class="apk-mono">{{ activeFile.relativePath }}</span>
          </div>
          <STag size="small" :bordered="false">{{ formatBytes(activeFile.size) }}</STag>
        </header>

        <div class="preview-body">
          <div v-if="previewState === 'empty'" class="preview-empty">
            <div class="preview-empty-icon">
              <FileText :size="26" :stroke-width="1.4" />
            </div>
            <strong>选择文件预览</strong>
            <p>支持文本、XML、Smali、图片等常见格式</p>
          </div>

          <div v-else-if="previewState === 'loading'" class="preview-empty">
            <span class="apk-spinner" />
            <span>正在读取…</span>
          </div>

          <div v-else-if="previewState === 'error'" class="preview-empty error">
            <AlertCircle :size="22" />
            <strong>预览失败</strong>
            <p>{{ previewError }}</p>
          </div>

          <div v-else-if="previewState === 'image'" class="image-view">
            <img :src="previewImage" :alt="activeFile?.name" />
          </div>

          <div v-else-if="previewState === 'binary'" class="preview-empty">
            <FileArchive :size="26" :stroke-width="1.4" />
            <strong>二进制文件</strong>
            <p>暂不支持预览 {{ activeFile?.extension ? `.${activeFile.extension}` : "该类型" }} 文件，可在文件夹中打开。</p>
          </div>

          <div v-else-if="previewState === 'text'" class="text-view">
            <pre class="code-view">{{ previewText?.content }}</pre>
            <div v-if="previewText?.truncated" class="truncate-note">
              文件较大，仅显示部分内容（共 {{ formatBytes(previewText.size) }}）
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.file-browser {
  display: grid;
  height: 100%;
  min-height: 0;
  grid-template-rows: auto minmax(0, 1fr);
  background: rgba(8, 15, 27, 0.35);
}
.fb-toolbar {
  display: flex;
  min-height: 42px;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--apk-border);
}
.root-label {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--apk-text-soft);
  font-size: 12px;
  font-weight: 600;
}
.spacer {
  flex: 1;
}
.fb-body {
  --pane-size: initial;
  display: grid;
  min-height: 0;
  grid-template-columns: var(--pane-size, minmax(220px, 320px)) minmax(0, 1fr);
}
.tree-column {
  position: relative;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
}
.tree-search {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px;
  border-right: 1px solid var(--apk-border);
  border-bottom: 1px solid var(--apk-border);
  color: var(--apk-muted);
}
.tree-search > svg { flex: 0 0 auto; }
.tree-search-input { min-width: 0; flex: 1; }
.tree-pane {
  height: 100%;
  position: relative;
  min-height: 0;
  overflow: auto;
  padding: 6px 4px;
  border-right: 1px solid var(--apk-border);
}
.tree-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 26px 12px;
  color: var(--apk-muted);
  font-size: 12px;
  text-align: center;
}
.tree-state.error {
  flex-direction: column;
  color: var(--apk-red);
}
.tree-notice {
  margin: 4px 8px 6px;
  padding: 5px 8px;
  border: 1px solid rgba(251, 191, 36, 0.24);
  border-radius: 6px;
  background: rgba(251, 191, 36, 0.08);
  color: var(--apk-amber);
  font-size: 10.5px;
}
.tree-row {
  display: flex;
  width: 100%;
  height: 26px;
  align-items: center;
  gap: 5px;
  padding-right: 7px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--apk-text-soft);
  cursor: pointer;
  font-size: 12px;
  text-align: left;
}
.tree-row:hover {
  background: rgba(148, 163, 184, 0.08);
}
.tree-row.selected {
  background: rgba(56, 189, 248, 0.13);
  color: #bae6fd;
}
.tree-row.directory {
  color: #d7e3f0;
}
.tree-row:focus-visible {
  outline: 1px solid var(--apk-accent);
  outline-offset: -1px;
}
.chevron {
  display: inline-flex;
  width: 12px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  color: var(--apk-faint);
  transition: transform 0.14s ease;
}
.chevron.open {
  transform: rotate(90deg);
}
.entry-icon {
  flex: 0 0 auto;
  color: var(--apk-muted);
}
.tree-row.selected .entry-icon {
  color: var(--apk-accent);
}
.entry-name {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.entry-size {
  flex: 0 0 auto;
  color: var(--apk-faint);
  font-family: var(--apk-mono);
  font-size: 9.5px;
}
.apk-spinner.tiny {
  width: 10px;
  height: 10px;
  color: var(--apk-accent);
}
.preview-pane {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: auto minmax(0, 1fr);
}
.preview-head {
  display: flex;
  min-height: 42px;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--apk-border);
}
.preview-title {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 1px;
}
.preview-title strong {
  overflow: hidden;
  color: var(--apk-text);
  font-size: 12px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.preview-title span {
  overflow: hidden;
  color: var(--apk-faint);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.preview-body {
  min-height: 0;
  overflow: auto;
  background: #081120;
}
.preview-empty {
  display: flex;
  height: 100%;
  min-height: 220px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 28px;
  color: var(--apk-muted);
  text-align: center;
}
.preview-empty strong {
  color: var(--apk-text-soft);
  font-size: 12.5px;
}
.preview-empty p {
  max-width: 360px;
  margin: 0;
  color: var(--apk-faint);
  font-size: 11.5px;
  line-height: 1.6;
}
.preview-empty.error {
  color: var(--apk-red);
}
.preview-empty-icon {
  display: grid;
  width: 56px;
  height: 56px;
  place-items: center;
  border: 1px dashed var(--apk-border-strong);
  border-radius: 16px;
  color: var(--apk-faint);
}
.image-view {
  display: flex;
  min-height: 100%;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background-color: #0b1526;
  background-image:
    linear-gradient(45deg, rgba(148, 163, 184, 0.035) 25%, transparent 25%),
    linear-gradient(-45deg, rgba(148, 163, 184, 0.035) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgba(148, 163, 184, 0.035) 75%),
    linear-gradient(-45deg, transparent 75%, rgba(148, 163, 184, 0.035) 75%);
  background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  background-size: 16px 16px;
}
.image-view img {
  max-width: 100%;
  max-height: calc(100vh - 320px);
  object-fit: contain;
  image-rendering: auto;
}
.text-view {
  min-height: 100%;
}
.code-view {
  margin: 0;
  padding: 14px 16px 20px;
  color: #c7d5e6;
  font-family: var(--apk-mono);
  font-size: 11.5px;
  line-height: 1.62;
  tab-size: 4;
  white-space: pre-wrap;
  word-break: break-word;
}
.truncate-note {
  margin: 0 14px 14px;
  padding: 7px 10px;
  border: 1px solid rgba(251, 191, 36, 0.22);
  border-radius: 6px;
  background: rgba(251, 191, 36, 0.07);
  color: var(--apk-amber);
  font-size: 10.5px;
}
@media (max-width: 900px) {
  .fb-body {
    grid-template-columns: var(--pane-size, minmax(180px, 44%)) minmax(0, 1fr);
  }
}
</style>
