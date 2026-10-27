<script setup lang="ts">
import { computed } from "vue"
import { FolderOpen, PackageOpen, Search, Trash2, Upload } from "lucide-vue-next"
import {
  formatBytes,
  localizedLabel,
  matchesQuery,
  packageStatus,
  type PackageInfo,
  type PackageStage,
  type ProgressEvent,
} from "@mobiletoolbox/protocol"
import PackageIcon from "./PackageIcon.vue"

const props = withDefaults(defineProps<{
  packages: PackageInfo[]
  selectedId: string | null
  icons: Record<string, string>
  busy: Record<string, PackageStage>
  query: string
  importing?: boolean
  /** 当前导入任务的实时进度 */
  importTask?: ProgressEvent | null
}>(), { importing: false, importTask: null })

const emit = defineEmits<{
  select: [id: string]
  import: []
  delete: [pkg: PackageInfo]
  reveal: [pkg: PackageInfo]
  "update:query": [value: string]
}>()

const filtered = computed(() => props.packages.filter((pkg) => matchesQuery(pkg, props.query)))

const importPercent = computed(() => {
  const ratio = props.importTask?.ratio
  if (typeof ratio !== "number" || !Number.isFinite(ratio)) return null
  return Math.max(0, Math.min(100, Math.round(ratio * 100)))
})
const importText = computed(() =>
  importPercent.value === null ? "解析中" : `解析中 ${importPercent.value}%`,
)
const importNote = computed(() => props.importTask?.message ?? "准备导入…")

const locale = typeof navigator === "undefined" ? "zh-CN" : navigator.language

const stageText: Record<string, string> = {
  importing: "解析中",
  decoding: "解包中",
  jar: "生成 JAR",
  decompiling: "反编译中",
  removing: "删除中",
}

function busyText(pkg: PackageInfo): string {
  const stage = props.busy[pkg.id]
  return stage ? stageText[stage] ?? "处理中" : ""
}

function onSelect(pkg: PackageInfo): void {
  emit("select", pkg.id)
}
</script>

<template>
  <section class="package-list">
    <header class="list-head">
      <div class="head-line">
        <div class="head-title">
          <PackageOpen :size="16" :stroke-width="1.8" />
          <span>安装包库</span>
          <span class="count">{{ packages.length }}</span>
        </div>
        <SButton type="primary" size="small" :loading="importing" @click="emit('import')">
          <Upload v-if="!importing" :size="14" />
          {{ importing ? importText : "导入 APK" }}
        </SButton>
      </div>
      <div v-if="importing" class="import-status">
        <div class="import-track">
          <span
            class="import-fill"
            :class="{ indeterminate: importPercent === null }"
            :style="importPercent === null ? undefined : { width: `${importPercent}%` }"
          />
        </div>
        <span class="import-note" :title="importNote">{{ importNote }}</span>
      </div>
      <div class="search-wrap">
        <Search class="search-icon" :size="14" />
        <SInput
          :value="query"
          size="small"
          clearable
          placeholder="搜索应用、包名、版本"
          class="search-input"
          @update:value="emit('update:query', String($event ?? ''))"
        />
      </div>
    </header>

    <div class="list-body">
      <div v-if="filtered.length === 0" class="empty-wrap">
        <div v-if="packages.length === 0" class="empty">
          <div class="empty-orbit">
            <PackageOpen :size="30" :stroke-width="1.4" />
          </div>
          <strong>还没有安装包</strong>
          <p>点击「导入 APK」，或把 APK 文件直接拖进窗口</p>
          <SButton type="primary" size="small" @click="emit('import')">
            <Upload :size="14" />
            导入 APK
          </SButton>
        </div>
        <div v-else class="empty compact">
          <Search :size="24" :stroke-width="1.4" />
          <strong>没有匹配的安装包</strong>
          <p>换个关键词试试</p>
        </div>
      </div>

      <button
        v-for="pkg in filtered"
        v-else
        :key="pkg.id"
        type="button"
        class="pkg-row"
        :class="{ selected: pkg.id === selectedId, busy: Boolean(busy[pkg.id]) }"
        @click="onSelect(pkg)"
      >
        <PackageIcon :name="localizedLabel(pkg, locale)" :url="icons[pkg.id]" :size="42" />

        <span class="row-main">
          <span class="row-title">
            <span class="app-name">{{ localizedLabel(pkg, locale) }}</span>
            <STag :type="packageStatus(pkg).tone" size="small" :bordered="false">
              {{ packageStatus(pkg).label }}
            </STag>
          </span>
          <span class="row-package apk-mono">{{ pkg.packageName || pkg.fileName }}</span>
          <span class="row-meta">
            <span>v{{ pkg.versionName || "-" }}</span>
            <span class="sep">·</span>
            <span>{{ formatBytes(pkg.sizeBytes) }}</span>
            <span class="sep">·</span>
            <span>minSdk {{ pkg.minSdk || "-" }}</span>
          </span>
        </span>

        <span class="row-side">
          <span v-if="busy[pkg.id]" class="busy-line">
            <span class="apk-spinner" />
            {{ busyText(pkg) }}
          </span>
          <span class="row-actions">
            <SIconButton size="26" variant="ghost" title="在资源管理器中显示" @click.stop="emit('reveal', pkg)">
              <FolderOpen :size="14" />
            </SIconButton>
            <SIconButton size="26" variant="ghost" type="error" title="删除安装包" @click.stop="emit('delete', pkg)">
              <Trash2 :size="14" />
            </SIconButton>
          </span>
        </span>
      </button>
    </div>

    <footer v-if="filtered.length > 0" class="list-foot">
      <span>{{ query ? `${filtered.length} / ${packages.length}` : `${packages.length} 个安装包` }}</span>
      <span class="hint">拖入 APK 可快速导入</span>
    </footer>
  </section>
</template>

<style scoped>
.package-list {
  display: flex;
  height: 100%;
  min-height: 0;
  flex-direction: column;
  border-right: 1px solid var(--apk-border);
  background: rgba(9, 17, 30, 0.55);
}
.list-head {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  gap: 10px;
  padding: 14px 14px 10px;
  border-bottom: 1px solid var(--apk-border);
}
.head-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.import-status {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.import-track {
  position: relative;
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(56, 189, 248, 0.12);
}
.import-fill {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #38bdf8, #818cf8);
  transition: width 0.18s ease;
}
.import-fill.indeterminate {
  width: 38%;
  animation: import-slide 1.1s ease-in-out infinite;
}
@keyframes import-slide {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(320%); }
}
.import-note {
  overflow: hidden;
  color: var(--apk-muted);
  font-size: 10.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.head-title {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 7px;
  color: var(--apk-text-soft);
  font-size: 12.5px;
  font-weight: 600;
}
.head-title .count {
  padding: 0 6px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
  color: var(--apk-muted);
  font-family: var(--apk-mono);
  font-size: 10px;
  font-weight: 500;
  line-height: 17px;
}
.search-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.search-icon {
  position: absolute;
  left: 9px;
  z-index: 1;
  color: var(--apk-faint);
  pointer-events: none;
}
.search-input {
  width: 100%;
}
.search-input :deep(.s-input) {
  padding-left: 28px;
  font-size: 12px;
}
.list-body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.empty-wrap {
  display: grid;
  height: 100%;
  place-items: center;
}
.empty {
  display: flex;
  max-width: 260px;
  flex-direction: column;
  align-items: center;
  gap: 9px;
  padding: 26px 12px;
  color: var(--apk-muted);
  text-align: center;
}
.empty strong {
  color: var(--apk-text-soft);
  font-size: 13px;
  font-weight: 600;
}
.empty p {
  margin: 0;
  color: var(--apk-faint);
  font-size: 11.5px;
  line-height: 1.6;
}
.empty.compact {
  gap: 7px;
}
.empty-orbit {
  display: grid;
  width: 64px;
  height: 64px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px dashed rgba(56, 189, 248, 0.34);
  border-radius: 20px;
  background: radial-gradient(circle, rgba(56, 189, 248, 0.12), transparent 68%);
  color: var(--apk-accent);
}
.pkg-row {
  position: relative;
  display: flex;
  width: 100%;
  align-items: center;
  gap: 10px;
  padding: 9px 8px 9px 9px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
  transition: background 0.14s ease, border-color 0.14s ease;
}
.pkg-row + .pkg-row {
  margin-top: 2px;
}
.pkg-row:hover {
  background: rgba(148, 163, 184, 0.07);
}
.pkg-row.selected {
  border-color: rgba(56, 189, 248, 0.26);
  background: linear-gradient(90deg, rgba(56, 189, 248, 0.12), rgba(56, 189, 248, 0.035));
}
.pkg-row.selected::before {
  position: absolute;
  top: 10px;
  bottom: 10px;
  left: -1px;
  width: 2px;
  border-radius: 2px;
  background: var(--apk-accent);
  content: "";
  box-shadow: 0 0 10px rgba(56, 189, 248, 0.6);
}
.pkg-row:focus-visible {
  outline: 2px solid var(--apk-accent);
  outline-offset: -2px;
}
.pkg-row.busy {
  cursor: progress;
}
.row-main {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 2px;
}
.row-title {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 7px;
}
.app-name {
  overflow: hidden;
  color: var(--apk-text);
  font-size: 13px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-package {
  overflow: hidden;
  color: var(--apk-muted);
  font-size: 10.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--apk-faint);
  font-size: 10.5px;
}
.row-meta .sep {
  opacity: 0.6;
}
.row-side {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  align-items: flex-end;
  gap: 3px;
}
.busy-line {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--apk-accent);
  font-size: 10.5px;
}
.busy-line .apk-spinner {
  width: 10px;
  height: 10px;
}
.row-actions {
  display: flex;
  align-items: center;
  gap: 1px;
  opacity: 0;
  transition: opacity 0.14s ease;
}
.pkg-row:hover .row-actions,
.pkg-row.selected .row-actions,
.pkg-row:focus-within .row-actions {
  opacity: 1;
}
.list-foot {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 7px 14px;
  border-top: 1px solid var(--apk-border);
  color: var(--apk-faint);
  font-size: 10.5px;
}
.list-foot .hint {
  opacity: 0.75;
}
</style>