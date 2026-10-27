<script setup lang="ts">
import { computed, ref, watch } from "vue"
import {
  Boxes,
  Code2,
  Copy,
  FileSearch,
  FolderOpen,
  HardDriveDownload,
  Info,
  PackageOpen,
  ShieldCheck,
  Smartphone,
  Trash2,
  Wrench,
} from "lucide-vue-next"
import {
  androidLabel,
  androidName,
  formatBytes,
  formatDateTime,
  localizedLabel,
  packageStatus,
  type LoadDirectory,
  type LoadImage,
  type LoadText,
  type PackageInfo,
  type PackageStage,
} from "@mobiletoolbox/protocol"
import { toast } from "@qingfeng346/ui-kit"
import PackageIcon from "./PackageIcon.vue"
import FileBrowser from "./FileBrowser.vue"
import { permissionDescription } from "../androidPermissions"

const props = withDefaults(defineProps<{
  pkg: PackageInfo | null
  iconUrl: string
  busyStage?: PackageStage
  deviceCount?: number
  loadDirectory: LoadDirectory
  loadText: LoadText
  loadImage: LoadImage
}>(), { busyStage: "idle", deviceCount: 0 })

const emit = defineEmits<{
  decode: [pkg: PackageInfo]
  "build-jar": [pkg: PackageInfo]
  decompile: [pkg: PackageInfo]
  "open-folder": [pkg: PackageInfo, kind: "root" | "source" | "java"]
  reveal: [pkg: PackageInfo]
  install: [pkg: PackageInfo]
  delete: [pkg: PackageInfo]
}>()

const locale = typeof navigator === "undefined" ? "zh-CN" : navigator.language
const tab = ref("overview")

watch(
  () => props.pkg?.id,
  () => {
    tab.value = "overview"
  },
)

const label = computed(() => (props.pkg ? localizedLabel(props.pkg, locale) : ""))
const status = computed(() => (props.pkg ? packageStatus(props.pkg) : null))

const stageText = computed(() => {
  switch (props.busyStage) {
    case "importing": return "正在解析安装包…"
    case "decoding": return "正在解包资源（apktool）…"
    case "jar": return "正在生成 source.jar（dex2jar）…"
    case "decompiling": return "正在反编译 Java 源码（CFR）…"
    default: return ""
  }
})

const busy = computed(() => Boolean(stageText.value))
function fallbackCopy(text: string): boolean {
  try {
    const area = document.createElement("textarea")
    area.value = text
    area.style.position = "fixed"
    area.style.opacity = "0"
    document.body.appendChild(area)
    area.select()
    const ok = document.execCommand("copy")
    document.body.removeChild(area)
    return ok
  } catch {
    return false
  }
}

async function copyValue(value: string, name: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(value)
    toast.show("success", `${name}已复制`)
  } catch {
    if (fallbackCopy(value)) toast.show("success", `${name}已复制`)
    else toast.show("error", "复制失败，请手动选择文本")
  }
}

function requestInstall(): void {
  if (!props.pkg) return
  if (props.deviceCount <= 0) {
    toast.show("warning", "请先在「设备」页选择一台在线设备")
    return
  }
  emit("install", props.pkg)
}
</script>

<template>
  <section class="package-details">
    <div v-if="!pkg" class="detail-empty">
      <div class="empty-visual">
        <PackageOpen :size="40" :stroke-width="1.2" />
      </div>
      <h2>选择一个安装包</h2>
      <p>从左侧列表选择，或导入新的 APK 开始分析</p>
      <div class="empty-features">
        <span><FileSearch :size="13" /> 解析清单</span>
        <span><PackageOpen :size="13" /> 解包资源</span>
        <span><Code2 :size="13" /> 反编译源码</span>
        <span><Smartphone :size="13" /> 安装到设备</span>
      </div>
    </div>

    <template v-else>
      <header class="detail-head">
        <PackageIcon :name="label" :url="iconUrl" :size="62" :radius="16" />
        <div class="head-text">
          <div class="head-name">
            <h2>{{ label }}</h2>
            <STag v-if="status" :type="status.tone" size="small" :bordered="false">{{ status.label }}</STag>
            <STag v-if="pkg.debuggable" type="warning" size="small" :bordered="false">debuggable</STag>
          </div>
          <div class="head-package apk-mono">{{ pkg.packageName || pkg.fileName }}</div>
          <div class="head-meta">
            <span>v{{ pkg.versionName || "-" }} ({{ pkg.versionCode || "-" }})</span>
            <span class="dot" />
            <span>{{ androidName(pkg.minSdk) }}</span>
            <span class="dot" />
            <span>{{ formatBytes(pkg.sizeBytes) }}</span>
            <span class="dot" />
            <span>{{ pkg.fileName }}</span>
          </div>
        </div>

        <div class="head-actions">
          <SButton
            v-if="!pkg.decoded"
            type="primary"
            size="small"
            :loading="busyStage === 'decoding'"
            :disabled="busy"
            @click="emit('decode', pkg)"
          >
            <PackageOpen v-if="busyStage !== 'decoding'" :size="14" />
            解包资源
          </SButton>
          <SButton
            v-else
            type="primary"
            size="small"
            :loading="busyStage === 'decompiling' || busyStage === 'jar'"
            :disabled="busy"
            @click="emit('decompile', pkg)"
          >
            <Code2 v-if="busyStage !== 'decompiling' && busyStage !== 'jar'" :size="14" />
            {{ pkg.decompiled ? "重新反编译" : "反编译源码" }}
          </SButton>
          <SButton size="small" :disabled="busy" @click="emit('open-folder', pkg, 'root')">
            <FolderOpen :size="14" />
            打开目录
          </SButton>
        </div>
      </header>

      <div v-if="busy" class="stage-bar">
        <span class="apk-spinner" />
        <span>{{ stageText }}</span>
        <span class="stage-hint">正在后台处理</span>
      </div>

      <STabs v-model:value="tab" class="detail-tabs">
        <STabPane name="overview" tab="概览">
          <div class="tab-body">
            <div class="info-grid">
              <div class="info-card">
                <div class="card-title"><Info :size="14" /> 基本信息</div>
                <SInfoTable class="info-table" label-width="104px">
                  <SInfoRow label="应用名称">{{ label }}</SInfoRow>
                  <SInfoRow label="包名" mono>{{ pkg.packageName || "-" }}</SInfoRow>
                  <SInfoRow label="版本名称" mono>{{ pkg.versionName || "-" }}</SInfoRow>
                  <SInfoRow label="版本号" mono>{{ pkg.versionCode || "-" }}</SInfoRow>
                  <SInfoRow label="最低支持">{{ androidLabel(pkg.minSdk) }}</SInfoRow>
                  <SInfoRow label="目标 SDK">{{ androidLabel(pkg.targetSdk) }}</SInfoRow>
                  <SInfoRow label="编译 SDK">{{ androidLabel(pkg.compileSdk) }}</SInfoRow>
                  <SInfoRow label="启动 Activity" mono>{{ pkg.launchableActivity || "-" }}</SInfoRow>
                  <SInfoRow label="安装位置">{{ pkg.installLocation || "-" }}</SInfoRow>
                  <SInfoRow label="原生架构">{{ pkg.nativeCode.length ? pkg.nativeCode.join(" · ") : "-" }}</SInfoRow>
                </SInfoTable>
              </div>

              <div class="info-card">
                <div class="card-title"><HardDriveDownload :size="14" /> 文件与指纹</div>
                <SInfoTable class="info-table" label-width="104px">
                  <SInfoRow label="文件">{{ pkg.fileName }}</SInfoRow>
                  <SInfoRow label="大小">{{ formatBytes(pkg.sizeBytes) }}</SInfoRow>
                  <SInfoRow label="导入时间">{{ formatDateTime(pkg.importedAt) }}</SInfoRow>
                  <SInfoRow label="SHA-256" mono>
                    <div class="hash">
                      <span>{{ pkg.sha256 }}</span>
                      <SIconButton size="22" variant="ghost" title="复制 SHA-256" @click="copyValue(pkg.sha256, 'SHA-256 ')">
                        <Copy :size="12" />
                      </SIconButton>
                    </div>
                  </SInfoRow>
                </SInfoTable>
              </div>

              <div class="info-card actions-card">
                <div class="card-title"><Wrench :size="14" /> 分析操作</div>
                <div class="action-list">
                  <button type="button" class="action-row" :disabled="busy" @click="emit('decode', pkg)">
                    <PackageOpen :size="15" />
                    <span class="action-text">
                      <strong>{{ pkg.decoded ? "重新解包资源" : "解包资源" }}</strong>
                      <small>apktool 还原 Manifest、资源与 Smali</small>
                    </span>
                  </button>
                  <button type="button" class="action-row" :disabled="busy" @click="emit('build-jar', pkg)">
                    <Boxes :size="15" />
                    <span class="action-text">
                      <strong>{{ pkg.hasJar ? "重新生成 source.jar" : "生成 source.jar" }}</strong>
                      <small>dex2jar 将 DEX 转换为 JAR</small>
                    </span>
                  </button>
                  <button type="button" class="action-row" :disabled="busy" @click="emit('decompile', pkg)">
                    <Code2 :size="15" />
                    <span class="action-text">
                      <strong>{{ pkg.decompiled ? "重新反编译源码" : "反编译 Java 源码" }}</strong>
                      <small>source.jar → CFR，可在「Java 源码」中浏览</small>
                    </span>
                  </button>
                  <button type="button" class="action-row" @click="emit('open-folder', pkg, 'root')">
                    <FolderOpen :size="15" />
                    <span class="action-text">
                      <strong>打开工作目录</strong>
                      <small>APK、解包资源与源码都保存在这里</small>
                    </span>
                  </button>
                  <button type="button" class="action-row" @click="emit('reveal', pkg)">
                    <FileSearch :size="15" />
                    <span class="action-text">
                      <strong>定位 APK 文件</strong>
                      <small>在系统资源管理器中显示原始文件</small>
                    </span>
                  </button>
                </div>
                <div class="danger-row">
                  <SButton type="error" size="small" tertiary :disabled="busy" @click="emit('delete', pkg)">
                    <Trash2 :size="13" />
                    删除安装包
                  </SButton>
                  <SButton size="small" :disabled="busy || deviceCount === 0" @click="requestInstall">
                    <Smartphone :size="13" />
                    {{ deviceCount === 0 ? "无可用设备" : "安装到设备" }}
                  </SButton>
                </div>
              </div>
            </div>

            <div class="permissions-card">
              <div class="card-title">
                <ShieldCheck :size="14" />
                权限
                <STag size="small" :bordered="false">{{ pkg.permissions.length }}</STag>
                <span class="permissions-note">来自 AndroidManifest.xml</span>
              </div>
              <div v-if="pkg.permissions.length === 0" class="permissions-empty">未声明额外权限</div>
              <dl v-else class="permission-list" aria-label="安装包声明的权限及中文说明">
                <div v-for="permission in pkg.permissions" :key="permission" class="permission-row">
                  <dt class="permission-name apk-mono">{{ permission }}</dt>
                  <dd class="permission-description">{{ permissionDescription(permission) }}</dd>
                </div>
              </dl>
            </div>
          </div>
        </STabPane>

        <STabPane name="files" tab="文件资源">
          <FileBrowser
            v-if="pkg.decoded"
            :key="`${pkg.id}-source`"
            root="source"
            :load-directory="loadDirectory"
            :load-text="loadText"
            :load-image="loadImage"
            @open-folder="emit('open-folder', pkg, 'source')"
          />
          <div v-else class="locked-pane">
            <div class="locked-icon"><PackageOpen :size="30" :stroke-width="1.3" /></div>
            <h3>尚未解包资源</h3>
            <p>解包后可以浏览 AndroidManifest.xml、资源图片、布局 XML 和 Smali 代码。</p>
            <SButton type="primary" :loading="busyStage === 'decoding'" :disabled="busy" @click="emit('decode', pkg)">
              <PackageOpen :size="14" />
              解包资源
            </SButton>
          </div>
        </STabPane>

        <STabPane name="java" tab="Java 源码">
          <FileBrowser
            v-if="pkg.decompiled"
            :key="`${pkg.id}-java`"
            root="java"
            :load-directory="loadDirectory"
            :load-text="loadText"
            :load-image="loadImage"
            @open-folder="emit('open-folder', pkg, 'java')"
          />
          <div v-else class="locked-pane">
            <div class="locked-icon violet"><Code2 :size="30" :stroke-width="1.3" /></div>
            <h3>{{ pkg.hasJar ? "已生成 JAR，等待反编译" : "尚未反编译源码" }}</h3>
            <p>将依次执行 dex2jar 与 CFR，生成可在应用内浏览的 Java 源码；大型安装包可能需要数分钟。</p>
            <SButton type="primary" :loading="busyStage === 'decompiling' || busyStage === 'jar'" :disabled="busy" @click="emit('decompile', pkg)">
              <Code2 :size="14" />
              反编译 Java 源码
            </SButton>
          </div>
        </STabPane>
      </STabs>
    </template>
  </section>
</template>

<style scoped>
.package-details {
  display: flex;
  height: 100%;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  background: rgba(8, 15, 27, 0.3);
}

.detail-empty {
  display: flex;
  height: 100%;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 32px;
  color: var(--apk-muted);
  text-align: center;
}
.empty-visual {
  display: grid;
  width: 96px;
  height: 96px;
  margin-bottom: 6px;
  place-items: center;
  border: 1px dashed rgba(56, 189, 248, 0.28);
  border-radius: 28px;
  background:
    radial-gradient(circle at 50% 40%, rgba(56, 189, 248, 0.16), transparent 66%),
    rgba(56, 189, 248, 0.03);
  color: var(--apk-accent);
}
.detail-empty h2 {
  margin: 0;
  color: var(--apk-text);
  font-size: 17px;
  font-weight: 650;
}
.detail-empty p {
  margin: 0;
  color: var(--apk-faint);
  font-size: 12px;
}
.empty-features {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  margin-top: 12px;
}
.empty-features span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 9px;
  border: 1px solid var(--apk-border);
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.04);
  color: var(--apk-muted);
  font-size: 10.5px;
}

.detail-head {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 14px;
  padding: 16px 20px 14px;
  border-bottom: 1px solid var(--apk-border);
}
.head-text {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 3px;
}
.head-name {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 7px;
}
.head-name h2 {
  overflow: hidden;
  margin: 0;
  color: var(--apk-text);
  font-size: 18px;
  font-weight: 680;
  letter-spacing: -0.015em;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.head-package {
  overflow: hidden;
  color: var(--apk-accent);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.head-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  color: var(--apk-faint);
  font-size: 10.5px;
}
.head-meta .dot {
  width: 2px;
  height: 2px;
  border-radius: 50%;
  background: var(--apk-faint);
}
.head-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 7px;
}
.stage-bar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  padding: 7px 20px;
  border-bottom: 1px solid rgba(56, 189, 248, 0.18);
  background: linear-gradient(90deg, rgba(56, 189, 248, 0.1), rgba(56, 189, 248, 0.02));
  color: #7dd3fc;
  font-size: 11.5px;
}
.stage-bar .apk-spinner {
  color: var(--apk-accent);
}
.stage-hint {
  margin-left: auto;
  color: var(--apk-faint);
  font-size: 10px;
}

.detail-tabs {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.detail-tabs :deep(.s-tab-list) {
  padding: 0 12px;
  background: rgba(7, 13, 24, 0.32);
}
.detail-tabs :deep(.s-tab-button) {
  font-size: 12px;
}
.detail-tabs :deep(.s-tab-content) {
  min-height: 0;
}
.detail-tabs :deep(.s-tab-pane) {
  overflow-y: auto;
  scrollbar-gutter: stable;
}
.tab-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px 20px 24px;
}
.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
  align-items: start;
}
.info-card,
.permissions-card {
  padding: 14px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: linear-gradient(180deg, rgba(20, 34, 56, 0.6), rgba(12, 22, 38, 0.5));
}
.card-title {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 10px;
  color: var(--apk-text-soft);
  font-size: 12px;
  font-weight: 650;
}
.card-title svg {
  color: var(--apk-accent);
}
.permissions-note {
  margin-left: auto;
  color: var(--apk-faint);
  font-size: 10px;
  font-weight: 400;
}
.info-table {
  table-layout: fixed;
  font-size: 11.5px;
}
.info-table :deep(.s-info-row-value) {
  overflow-wrap: anywhere;
  color: var(--apk-text-soft);
}
.hash {
  display: flex;
  align-items: center;
  gap: 4px;
}
.hash span {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  font-size: 10.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.action-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.action-row {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--apk-text-soft);
  cursor: pointer;
  text-align: left;
  transition: background 0.14s ease, border-color 0.14s ease;
}
.action-row:hover:not(:disabled) {
  border-color: rgba(56, 189, 248, 0.2);
  background: rgba(56, 189, 248, 0.07);
}
.action-row:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}
.action-row > svg {
  flex: 0 0 auto;
  color: var(--apk-accent);
}
.action-text {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}
.action-text strong {
  font-size: 11.5px;
  font-weight: 600;
}
.action-text small {
  color: var(--apk-faint);
  font-size: 10px;
}
.danger-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid rgba(148, 163, 184, 0.09);
}
.permission-list {
  margin: 0;
}
.permission-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 6px 24px;
  padding: 10px 0;
  border-top: 1px solid var(--apk-border);
  font-size: 12px;
  line-height: 1.6;
}
.permission-name {
  color: var(--apk-text-soft);
  overflow-wrap: anywhere;
}
.permission-description {
  margin: 0;
  color: var(--apk-muted);
  overflow-wrap: anywhere;
}
.permissions-empty {
  padding: 4px 0;
  color: var(--apk-faint);
  font-size: 11.5px;
}
@media (max-width: 1100px) {
  .permission-row {
    grid-template-columns: minmax(0, 1fr);
  }
}

.locked-pane {
  display: flex;
  height: 100%;
  min-height: 300px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 40px;
  text-align: center;
}
.locked-icon {
  display: grid;
  width: 78px;
  height: 78px;
  margin-bottom: 2px;
  place-items: center;
  border: 1px dashed rgba(56, 189, 248, 0.3);
  border-radius: 24px;
  background: rgba(56, 189, 248, 0.05);
  color: var(--apk-accent);
}
.locked-icon.violet {
  border-color: rgba(167, 139, 250, 0.3);
  background: rgba(167, 139, 250, 0.06);
  color: var(--apk-violet);
}
.locked-pane h3 {
  margin: 0;
  color: var(--apk-text);
  font-size: 14px;
  font-weight: 650;
}
.locked-pane p {
  max-width: 440px;
  margin: 0 0 8px;
  color: var(--apk-faint);
  font-size: 11.5px;
  line-height: 1.65;
}
</style>
