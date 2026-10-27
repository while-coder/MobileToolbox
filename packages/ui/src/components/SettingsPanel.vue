<script setup lang="ts">
import { computed } from "vue"
import {
  Boxes,
  Coffee,
  FolderCog,
  FolderOpen,
  HardDrive,
  Info,
  RefreshCw,
  ShieldAlert,
  Smartphone,
  Terminal,
  Wrench,
} from "lucide-vue-next"
import type { EnvironmentInfo, ToolStatus } from "@mobiletoolbox/protocol"

const props = withDefaults(defineProps<{
  environment: EnvironmentInfo | null
  packagesCount: number
  loading: boolean
  updateStatus: "idle" | "checking" | "latest" | "available" | "downloading" | "installed" | "error"
  updateStatusText: string
  updaterSupported: boolean
}>(), {
  loading: false,
  updateStatus: "idle",
  updateStatusText: "",
  updaterSupported: false,
})

const emit = defineEmits<{
  refresh: []
  "open-workdir": []
  "clear-all": []
  "check-update": []
}>()

interface ToolRow {
  key: string
  name: string
  description: string
  status: ToolStatus | null
}

const tools = computed<ToolRow[]>(() => {
  const env = props.environment
  return [
    { key: "java", name: "Java 运行时", description: "apktool / dex2jar / CFR 的运行依赖", status: env?.java ?? null },
    { key: "adb", name: "adb", description: "安装 APK、推送 OBB、设备管理", status: env?.adb ?? null },
    { key: "apktool", name: "apktool", description: "解包 APK 资源与 Smali", status: env?.apktool ?? null },
    { key: "dex2jar", name: "dex2jar", description: "DEX 转 JAR 字节码", status: env?.dex2jar ?? null },
    { key: "cfr", name: "CFR", description: "JAR 转 Java 源码", status: env?.cfr ?? null },
  ]
})

const readyCount = computed(() => tools.value.filter((tool) => tool.status?.available).length)
const javaReady = computed(() => Boolean(props.environment?.java.available))
const adbDescription = computed(() => {
  const adb = props.environment?.adb
  if (!adb) return "检测中"
  return adb.available ? adb.path ?? "可用" : "缺失"
})

const updateButtonLabel = computed(() => {
  if (!props.updaterSupported) return "当前平台不支持"
  if (props.updateStatus === "checking") return "检查中…"
  if (props.updateStatus === "downloading") return "下载中…"
  if (props.updateStatus === "installed") return "重启生效"
  return "检查更新"
})
</script>

<template>
  <div class="settings-page">
    <div class="settings-inner">
      <section class="settings-hero">
        <div class="hero-mark"><Wrench :size="22" :stroke-width="1.5" /></div>
        <div>
          <h2>环境与工作目录</h2>
          <p>MobileToolbox 调用本地 Java、Android 构建工具和 adb 完成解析、解包、反编译与安装。</p>
        </div>
        <SButton size="small" :loading="loading" @click="emit('refresh')">
          <RefreshCw v-if="!loading" :size="13" />
          重新检测
        </SButton>
      </section>

      <section class="settings-card">
        <header class="card-head">
          <div class="card-title"><FolderCog :size="15" /> 工作目录</div>
          <SButton size="small" @click="emit('open-workdir')">
            <FolderOpen :size="13" />
            打开目录
          </SButton>
        </header>
        <div class="path-box apk-mono">{{ environment?.workDir || "检测中…" }}</div>
        <p class="card-note">每个安装包按「文件名-Hash」独立存放，包含原始 APK、解包资源、source.jar 与 Java 源码。</p>
      </section>

      <section class="settings-card">
        <header class="card-head">
          <div class="card-title"><Terminal :size="15" /> 工具链</div>
          <span class="ready-count">{{ readyCount }} / {{ tools.length }} 就绪</span>
        </header>

        <div v-if="!environment" class="tool-loading">
          <span class="apk-spinner" />
          正在检测工具链…
        </div>

        <div v-else class="tool-list">
          <div v-for="tool in tools" :key="tool.key" class="tool-row">
            <div class="tool-icon" :class="{ missing: !tool.status?.available }">
              <Coffee v-if="tool.key === 'java'" :size="14" />
              <Boxes v-else-if="tool.key === 'dex2jar' || tool.key === 'cfr'" :size="14" />
              <Smartphone v-else-if="tool.key === 'adb'" :size="14" />
              <Wrench v-else :size="14" />
            </div>
            <div class="tool-main">
              <div class="tool-name">
                {{ tool.name }}
                <span v-if="tool.status?.version" class="tool-version apk-mono">{{ tool.status.version }}</span>
              </div>
              <div class="tool-desc">{{ tool.description }}</div>
              <div class="tool-path apk-mono" :title="tool.status?.path || undefined">
                {{ tool.status?.available ? tool.status.path : tool.status?.detail || "未找到" }}
              </div>
            </div>
            <STag :type="tool.status?.available ? 'success' : 'error'" size="small" :bordered="false">
              {{ tool.status?.available ? "可用" : "缺失" }}
            </STag>
          </div>
        </div>

        <div v-if="environment && !javaReady" class="alert-line">
          <ShieldAlert :size="14" />
          <span>未检测到 Java。请安装 JDK 11 或更高版本，并确保 <span class="apk-mono">java</span> 在 PATH 中；否则无法解包和反编译。</span>
        </div>
        <div v-else-if="environment" class="hint-line">
          内置工具位于 <span class="apk-mono">{{ environment.toolsDir }}</span>；当前使用的 adb：<span class="apk-mono">{{ adbDescription }}</span>
        </div>
      </section>

      <section class="settings-card danger-card">
        <header class="card-head">
          <div class="card-title"><HardDrive :size="15" /> 数据管理</div>
          <STag size="small" :bordered="false">{{ packagesCount }} 个安装包</STag>
        </header>
        <p class="card-note">清空会删除安装包库索引及工作目录中的全部 APK、解包资源、JAR 和源码，操作不可撤销。</p>
        <div class="danger-actions">
          <SButton type="error" size="small" tertiary :disabled="packagesCount === 0" @click="emit('clear-all')">
            清空全部安装包
          </SButton>
        </div>
      </section>

      <section class="settings-card">
        <header class="card-head">
          <div class="card-title"><RefreshCw :size="15" /> 应用更新</div>
          <SButton
            size="small"
            :loading="updateStatus === 'checking'"
            :disabled="!updaterSupported || updateStatus === 'checking' || updateStatus === 'downloading'"
            @click="emit('check-update')"
          >
            <RefreshCw v-if="updateStatus !== 'checking'" :size="13" />
            {{ updateButtonLabel }}
          </SButton>
        </header>
        <p
          class="card-note update-note"
          :class="{ 'is-error': updateStatus === 'error' }"
          role="status"
          aria-live="polite"
        >
          {{ updateStatusText }}
        </p>
      </section>

      <section class="settings-card about-card">
        <header class="card-head">
          <div class="card-title"><Info :size="15" /> 关于</div>
          <STag size="small" :bordered="false">v{{ environment?.appVersion || "0.1.0" }}</STag>
        </header>
        <div class="about-grid">
          <div><span class="k">应用</span><span class="v">MobileToolbox · 移动应用与设备工具箱</span></div>
          <div><span class="k">平台</span><span class="v apk-mono">{{ environment?.platform || "-" }}</span></div>
          <div><span class="k">解析</span><span class="v">内置 APK 解析器 / apktool / dex2jar / CFR</span></div>
          <div><span class="k">设备</span><span class="v">系统 adb（缺失时使用内置版本）</span></div>
        </div>
        <p class="card-note about-note">仅用于分析你有权处理的安装包。所有处理都在本机完成，不会上传任何文件。</p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  height: 100%;
  overflow-y: auto;
  padding: 18px 20px 28px;
}
.settings-inner {
  display: flex;
  max-width: 880px;
  margin: 0 auto;
  flex-direction: column;
  gap: 13px;
}
.settings-hero {
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 16px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background:
    radial-gradient(circle at 0 0, rgba(56, 189, 248, 0.1), transparent 42%),
    linear-gradient(180deg, rgba(20, 34, 56, 0.5), rgba(12, 22, 38, 0.45));
}
.hero-mark {
  display: grid;
  width: 46px;
  height: 46px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid rgba(56, 189, 248, 0.24);
  border-radius: 13px;
  background: rgba(56, 189, 248, 0.08);
  color: var(--apk-accent);
}
.settings-hero > div:nth-child(2) {
  min-width: 0;
  flex: 1;
}
.settings-hero h2 {
  margin: 0 0 2px;
  color: var(--apk-text);
  font-size: 15px;
  font-weight: 650;
}
.settings-hero p {
  margin: 0;
  color: var(--apk-muted);
  font-size: 11px;
  line-height: 1.55;
}
.settings-card {
  padding: 14px 16px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: rgba(12, 22, 38, 0.5);
}
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}
.card-title {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--apk-text-soft);
  font-size: 12.5px;
  font-weight: 650;
}
.card-title svg {
  color: var(--apk-accent);
}
.ready-count {
  color: var(--apk-muted);
  font-family: var(--apk-mono);
  font-size: 10.5px;
}
.path-box {
  padding: 9px 11px;
  overflow-x: auto;
  border: 1px solid var(--apk-border);
  border-radius: 7px;
  background: #081120;
  color: #93c5fd;
  font-size: 11px;
  white-space: nowrap;
}
.card-note {
  margin: 9px 0 0;
  color: var(--apk-faint);
  font-size: 10.5px;
  line-height: 1.65;
}
.update-note {
  margin-top: 0;
}
.update-note.is-error {
  color: #fda4af;
}
.tool-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.tool-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 6px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.07);
}
.tool-row:last-child {
  border-bottom: 0;
}
.tool-icon {
  display: grid;
  width: 30px;
  height: 30px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 8px;
  background: rgba(56, 189, 248, 0.09);
  color: var(--apk-accent);
}
.tool-icon.missing {
  background: rgba(251, 113, 133, 0.09);
  color: var(--apk-red);
}
.tool-main {
  min-width: 0;
  flex: 1;
}
.tool-name {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--apk-text-soft);
  font-size: 11.5px;
  font-weight: 600;
}
.tool-version {
  color: var(--apk-faint);
  font-size: 9.5px;
  font-weight: 400;
}
.tool-desc {
  color: var(--apk-faint);
  font-size: 10px;
}
.tool-path {
  overflow: hidden;
  color: var(--apk-muted);
  font-size: 9.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tool-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 18px 6px;
  color: var(--apk-muted);
  font-size: 11.5px;
}
.alert-line,
.hint-line {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin-top: 10px;
  padding: 8px 10px;
  border-radius: 7px;
  font-size: 10.5px;
  line-height: 1.6;
}
.alert-line {
  border: 1px solid rgba(251, 113, 133, 0.25);
  background: rgba(251, 113, 133, 0.07);
  color: #fda4af;
}
.alert-line svg {
  flex: 0 0 auto;
  margin-top: 1px;
}
.hint-line {
  border: 1px solid var(--apk-border);
  background: rgba(148, 163, 184, 0.035);
  color: var(--apk-muted);
}
.alert-line .apk-mono,
.hint-line .apk-mono {
  color: inherit;
}
.danger-card .card-title svg {
  color: var(--apk-red);
}
.danger-actions {
  margin-top: 10px;
}
.about-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 8px 20px;
}
.about-grid > div {
  display: flex;
  gap: 10px;
  font-size: 11px;
}
.about-grid .k {
  width: 48px;
  flex: 0 0 auto;
  color: var(--apk-faint);
}
.about-grid .v {
  color: var(--apk-text-soft);
}
.about-note {
  padding-top: 9px;
  border-top: 1px solid rgba(148, 163, 184, 0.07);
}
</style>