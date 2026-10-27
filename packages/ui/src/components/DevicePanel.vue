<script setup lang="ts">
import ResizeHandle from "./ResizeHandle.vue"
import { computed, reactive, ref, watch } from "vue"
import {
  Cable,
  FolderUp,
  FolderOpen,
  PackageOpen,
  PlugZap,
  RefreshCw,
  Search,
  Smartphone,
  Trash2,
  Usb,
} from "lucide-vue-next"
import { adbStateLabel, deviceLabel, localizedLabel, type AndroidDevice, type InstalledApp, type PackageInfo } from "@mobiletoolbox/protocol"
import { toast } from "@qingfeng346/ui-kit"

const props = withDefaults(defineProps<{
  devices: AndroidDevice[]
  selectedId: string | null
  loading: boolean
  connecting: boolean
  busyText?: string | null
  packages: PackageInfo[]
  installedApps: InstalledApp[]
  appsLoading: boolean
  appsError: string
}>(), { busyText: "" })

const emit = defineEmits<{
  refresh: []
  "refresh-apps": []
  select: [id: string]
  connect: [address: string]
  disconnect: [address: string]
  "pick-install": []
  "pick-obb": []
  "install-library": [packageId: string]
  uninstall: [packageName: string]
}>()

const locale = typeof navigator === "undefined" ? "zh-CN" : navigator.language
const address = ref("")
const libraryId = ref<string>("")
const appQuery = ref("")
const detailTab = ref<"apps" | "logcat" | "files">("apps")
const appDirectory = ref("/storage/emulated/0")
const directoryMenu = reactive({ show: false, x: 0, y: 0, packageName: "" })
const directoryOptions = [
  { label: "共享存储目录（Android/data）", value: "external" },
  { label: "内部数据目录（data/data）", value: "internal" },
]
function showAppDirectories(packageName: string, event: MouseEvent): void {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  directoryMenu.packageName = packageName
  directoryMenu.x = Math.max(4, Math.min(rect.left, window.innerWidth - 280))
  directoryMenu.y = rect.bottom + 4
  directoryMenu.show = true
}
function openAppDirectory(location: string): void {
  directoryMenu.show = false
  if (location !== "external" && location !== "internal") return
  appDirectory.value = location === "external"
    ? `/storage/emulated/0/Android/data/${directoryMenu.packageName}`
    : `/data/data/${directoryMenu.packageName}`
  detailTab.value = "files"
}
const appFilter = ref<"third-party" | "system" | "all">("third-party")
const appFilters = [
  { id: "third-party" as const, label: "第三方应用" },
  { id: "system" as const, label: "系统应用" },
  { id: "all" as const, label: "全部" },
]
const filteredApps = computed(() => {
  const keyword = appQuery.value.trim().toLowerCase()
  return props.installedApps.filter((app) =>
    (appFilter.value === "all" || app.system === (appFilter.value === "system")) &&
    (!keyword || app.packageName.toLowerCase().includes(keyword) || app.appName.toLowerCase().includes(keyword)),
  )
})
watch(() => props.selectedId, () => {
  appQuery.value = ""
  appDirectory.value = "/storage/emulated/0"
  directoryMenu.show = false
})

const quickTargets = [
  { label: "MuMu 模拟器", address: "127.0.0.1:7555" },
  { label: "夜神模拟器", address: "127.0.0.1:62001" },
  { label: "蓝叠模拟器", address: "127.0.0.1:5555" },
]

const selectedDevice = computed(() => props.devices.find((device) => device.id === props.selectedId) ?? null)
const packageOptions = computed(() =>
  props.packages.map((pkg) => ({
    label: `${localizedLabel(pkg, locale)} · ${pkg.packageName || pkg.fileName}`,
    value: pkg.id,
  })),
)
const onlineCount = computed(() => props.devices.filter((device) => device.state === "device").length)
const canAct = computed(() => Boolean(selectedDevice.value) && selectedDevice.value?.state === "device")
const busy = computed(() => Boolean(props.busyText))

watch(
  () => props.packages,
  (packages) => {
    if (!packages.some((pkg) => pkg.id === libraryId.value)) libraryId.value = packages[0]?.id ?? ""
  },
  { immediate: true },
)

function stateClass(state: string): string {
  if (state === "device") return "online"
  if (state === "unauthorized") return "warn"
  return "error"
}

function submitConnect(): void {
  const value = address.value.trim()
  if (!value) {
    toast.show("warning", "请输入 adb 连接地址，例如 127.0.0.1:7555")
    return
  }
  emit("connect", value)
}

function useQuickTarget(value: string): void {
  address.value = value
}

function installFromLibrary(): void {
  if (!canAct.value) {
    toast.show("warning", "请先选择一台在线设备")
    return
  }
  if (!libraryId.value) {
    toast.show("warning", "安装包库为空，请先导入 APK")
    return
  }
  emit("install-library", libraryId.value)
}

</script>

<template>
  <div class="devices-page">
    <aside class="device-column">
      <ResizeHandle label="调整 ADB 设备列表宽度" :min="260" />
      <header class="column-head">
        <div class="column-title">
          <Smartphone :size="15" />
          <span>ADB 设备</span>
          <span class="count">{{ onlineCount }}/{{ devices.length }}</span>
        </div>
        <SIconButton size="28" variant="ghost" title="刷新设备" :disabled="loading" @click="emit('refresh')">
          <RefreshCw :size="14" :class="{ spinning: loading }" />
        </SIconButton>
      </header>

      <div class="connect-box">
        <label class="field-label"><Cable :size="12" /> 网络连接</label>
        <div class="connect-line">
          <SInput
            v-model:value="address"
            size="small"
            placeholder="127.0.0.1:7555"
            :disabled="connecting"
            @keydown.enter="submitConnect"
          />
          <SButton type="primary" size="small" :loading="connecting" @click="submitConnect">
            <PlugZap v-if="!connecting" :size="13" />
            连接
          </SButton>
        </div>
        <div class="quick-targets">
          <button v-for="target in quickTargets" :key="target.address" type="button" class="quick-chip" @click="useQuickTarget(target.address)">
            {{ target.label }}
          </button>
        </div>
      </div>

      <div class="device-list">
        <div v-if="loading && devices.length === 0" class="device-state">
          <span class="apk-spinner" />
          正在扫描设备…
        </div>
        <div v-else-if="devices.length === 0" class="device-state empty">
          <Usb :size="26" :stroke-width="1.4" />
          <strong>未发现设备</strong>
          <p>打开 USB 调试，或连接上方模拟器端口后刷新。</p>
        </div>
        <button
          v-for="device in devices"
          v-else
          :key="device.id"
          type="button"
          class="device-card"
          :class="{ selected: device.id === selectedId }"
          @click="emit('select', device.id)"
        >
          <span class="state-dot" :class="stateClass(device.state)" />
          <span class="device-summary">
            <span class="device-name">{{ deviceLabel(device) }}</span>
            <span class="device-id apk-mono">{{ device.id }}</span>
            <span class="device-meta">
              <span>{{ device.manufacturer || "未知厂商" }}</span>
              <span v-if="device.sdkInt"> · API {{ device.sdkInt }}</span>
            </span>
          </span>
          <STag :type="device.state === 'device' ? 'success' : device.state === 'unauthorized' ? 'warning' : 'error'" size="small" :bordered="false">
            {{ adbStateLabel(device.state) }}
          </STag>
        </button>
      </div>

      <div class="adb-hint">
        <span>系统 adb 不可用时自动使用内置 adb</span>
      </div>
    </aside>

    <section class="device-main">
      <template v-if="selectedDevice">
        <div class="device-hero">
          <div class="hero-icon">
            <Smartphone :size="30" :stroke-width="1.4" />
            <span class="state-dot" :class="stateClass(selectedDevice.state)" />
          </div>
          <div class="hero-text">
            <h2>{{ deviceLabel(selectedDevice) }}</h2>
            <div class="hero-meta apk-mono">{{ selectedDevice.id }}</div>
            <div class="hero-tags">
              <STag size="small" :bordered="false">{{ selectedDevice.product || "未知产品" }}</STag>
              <STag size="small" :bordered="false">API {{ selectedDevice.sdkInt || "?" }}</STag>
              <STag size="small" :bordered="false">{{ adbStateLabel(selectedDevice.state) }}</STag>
            </div>
          </div>
          <div class="hero-actions">
            <SButton
              v-if="selectedDevice.id.includes(':')"
              size="small"
              :disabled="busy"
              @click="emit('disconnect', selectedDevice.id)"
            >
              断开连接
            </SButton>
            <SButton size="small" :disabled="loading" @click="emit('refresh')">
              <RefreshCw :size="13" />
              刷新
            </SButton>
          </div>
        </div>

        <div v-if="busyText" class="busy-bar">
          <span class="apk-spinner" />
          <span>{{ busyText }}</span>
        </div>

        <div class="apps-filters" role="group" aria-label="设备功能">
          <button type="button" :class="{ active: detailTab === 'apps' }" :aria-pressed="detailTab === 'apps'" @click="detailTab = 'apps'">应用管理</button>
          <button type="button" :class="{ active: detailTab === 'logcat' }" :aria-pressed="detailTab === 'logcat'" @click="detailTab = 'logcat'">ADB logcat</button>
          <button type="button" :class="{ active: detailTab === 'files' }" :aria-pressed="detailTab === 'files'" @click="detailTab = 'files'">文件管理</button>
        </div>
        <div v-if="detailTab === 'logcat'" class="device-logcat"><slot name="logcat" /></div>
        <div v-else-if="detailTab === 'files'" class="device-files"><slot name="files" :path="appDirectory" /></div>
        <div v-else class="device-apps">
          <div class="action-grid">
            <div class="action-card">
              <div class="action-title"><PackageOpen :size="14" /> 安装 APK</div>
              <p>从安装包库选择一个 APK 安装到当前设备，或在系统中挑选文件。</p>
              <SSelect
                v-model:value="libraryId"
                size="small"
                :options="packageOptions"
                placeholder="安装包库为空"
                :disabled="busy || packages.length === 0"
              />
              <div class="action-buttons">
                <SButton type="primary" size="small" :disabled="!canAct || busy || packages.length === 0" @click="installFromLibrary">
                  <PackageOpen :size="13" />
                  安装所选安装包
                </SButton>
                <SButton size="small" :disabled="!canAct || busy" @click="emit('pick-install')">
                  选择 APK 文件
                </SButton>
                <SButton size="small" :disabled="!canAct || busy" @click="emit('pick-obb')">
                  <FolderUp :size="13" />
                  选择 OBB 文件并推送
                </SButton>
              </div>
            </div>
          </div>
          <section class="apps-card" aria-label="设备应用管理">
            <header class="apps-heading">
              <div class="action-title"><PackageOpen :size="14" /> 应用管理 <span class="apps-count">{{ filteredApps.length }} / {{ installedApps.length }}</span></div>
              <SButton size="small" :loading="appsLoading" :disabled="!canAct || busy" @click="emit('refresh-apps')">
                <RefreshCw v-if="!appsLoading" :size="13" />刷新应用
              </SButton>
            </header>
            <div class="apps-toolbar">
              <div class="apps-filters" role="group" aria-label="应用分类">
                <button v-for="filter in appFilters" :key="filter.id" type="button"
                  :class="{ active: appFilter === filter.id }" :aria-pressed="appFilter === filter.id"
                  @click="appFilter = filter.id">{{ filter.label }}</button>
              </div>
              <div class="apps-search">
                <Search :size="14" aria-hidden="true" />
                <SInput v-model:value="appQuery" size="small" clearable class="apps-search-input"
                  placeholder="搜索应用名称或包名…" aria-label="搜索设备应用名称或包名" />
              </div>
            </div>
            <div v-if="!canAct" class="apps-state">设备未在线，请连接并授权后刷新。</div>
            <div v-else-if="appsLoading" class="apps-state" role="status"><span class="apk-spinner" />正在读取设备应用…</div>
            <div v-else-if="appsError" class="apps-state error" role="alert">
              <span>{{ appsError }}</span>
              <SButton size="small" @click="emit('refresh-apps')">重试</SButton>
            </div>
            <div v-else-if="filteredApps.length === 0" class="apps-state">
              {{ appQuery.trim() ? '没有匹配的应用' : '暂无此类应用' }}
            </div>
            <ul v-else class="apps-list" aria-label="已安装应用">
              <li v-for="app in filteredApps" :key="app.packageName" class="app-row">
                <PackageOpen :size="18" class="app-icon" aria-hidden="true" />
                <div class="app-info">
                  <strong class="app-name" :title="app.appName">{{ app.appName }}</strong>
                  <span class="app-package apk-mono" :title="app.packageName">{{ app.packageName }}</span>
                </div>
                <STag size="small" :bordered="false">{{ app.system ? '系统' : '第三方' }}</STag>
                <SButton size="small" :disabled="!canAct || busy" :title="`打开 ${app.packageName} 的应用目录`" @click="showAppDirectories(app.packageName, $event)">
                  <FolderOpen :size="13" />打开应用目录
                </SButton>
                <SButton size="small" type="error" tertiary :disabled="busy || app.system"
                  :title="app.system ? '系统应用暂不提供卸载' : `卸载 ${app.packageName}`"
                  @click="emit('uninstall', app.packageName)">
                  <Trash2 :size="13" />卸载
                </SButton>
              </li>
            </ul>
          </section>
        </div>

        <div class="tips-card">
          <strong>连接提示</strong>
          <ul>
            <li>真机需要在开发者选项中打开 USB 调试，并在手机上确认授权。</li>
            <li>模拟器先启动，再点击「连接」；默认端口：MuMu 7555、夜神 62001、蓝叠 5555。</li>
            <li>若出现 unauthorized，请在设备上撤销 USB 调试授权后重新连接。</li>
          </ul>
        </div>
      </template>

      <div v-else class="device-empty">
        <div class="empty-orbit"><Smartphone :size="36" :stroke-width="1.2" /></div>
        <h2>选择或连接一台设备</h2>
        <p>支持 USB 真机与常见 Android 模拟器，连接后可直接安装安装包库中的 APK。</p>
      </div>
    </section>
    <SDropdown :show="directoryMenu.show" :x="directoryMenu.x" :y="directoryMenu.y" :options="directoryOptions"
      :on-clickoutside="() => { directoryMenu.show = false }" @select="openAppDirectory" />
  </div>
</template>

<style scoped>
.devices-page {
  --pane-size: initial;
  display: grid;
  height: 100%;
  min-height: 0;
  grid-template-columns: var(--pane-size, minmax(300px, 360px)) minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  overflow: hidden;
}
.device-column {
  position: relative;
  min-width: 0;
  display: flex;
  min-height: 0;
  flex-direction: column;
  border-right: 1px solid var(--apk-border);
  background: rgba(9, 17, 30, 0.55);
}
.column-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 13px 14px 8px;
}
.column-title {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--apk-text-soft);
  font-size: 12.5px;
  font-weight: 600;
}
.column-title .count {
  padding: 0 6px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
  color: var(--apk-muted);
  font-family: var(--apk-mono);
  font-size: 10px;
  line-height: 17px;
}
.spinning {
  animation: apk-spin 0.9s linear infinite;
}
.connect-box {
  margin: 0 12px 10px;
  padding: 10px;
  border: 1px solid var(--apk-border);
  border-radius: 10px;
  background: rgba(148, 163, 184, 0.035);
}
.field-label {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-bottom: 6px;
  color: var(--apk-faint);
  font-size: 10.5px;
}
.connect-line {
  display: flex;
  align-items: center;
  gap: 6px;
}
.quick-targets {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 8px;
}
.quick-chip {
  padding: 3px 8px;
  border: 1px solid var(--apk-border);
  border-radius: 999px;
  background: transparent;
  color: var(--apk-muted);
  cursor: pointer;
  font-size: 10px;
}
.quick-chip:hover {
  border-color: rgba(56, 189, 248, 0.35);
  background: rgba(56, 189, 248, 0.08);
  color: var(--apk-accent);
}
.device-list {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 8px;
}
.device-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 30px 16px;
  color: var(--apk-muted);
  font-size: 11.5px;
  text-align: center;
}
.device-state.empty {
  flex-direction: column;
  gap: 6px;
}
.device-state.empty strong {
  color: var(--apk-text-soft);
  font-size: 12.5px;
}
.device-state.empty p {
  max-width: 230px;
  margin: 0;
  color: var(--apk-faint);
  font-size: 11px;
  line-height: 1.6;
}
.device-card {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 9px;
  margin-bottom: 2px;
  padding: 9px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
}
.device-card:hover {
  background: rgba(148, 163, 184, 0.07);
}
.device-card.selected {
  border-color: rgba(56, 189, 248, 0.25);
  background: linear-gradient(90deg, rgba(56, 189, 248, 0.12), rgba(56, 189, 248, 0.03));
}
.state-dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: var(--apk-faint);
}
.state-dot.online {
  background: var(--apk-green);
  box-shadow: 0 0 0 3px rgba(52, 211, 153, 0.13);
}
.state-dot.warn {
  background: var(--apk-amber);
  box-shadow: 0 0 0 3px rgba(251, 191, 36, 0.13);
}
.state-dot.error {
  background: var(--apk-red);
  box-shadow: 0 0 0 3px rgba(251, 113, 133, 0.13);
}
.device-summary {
  display: flex;
  min-width: 0;
  flex-direction: column;
  overflow: hidden;
}
.device-name {
  color: var(--apk-text);
  font-size: 12px;
  font-weight: 600;
}
.device-id {
  color: var(--apk-muted);
  font-size: 10px;
}
.device-meta {
  color: var(--apk-faint);
  font-size: 10px;
}
.device-card > .device-summary {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 1px;
}
.adb-hint {
  padding: 7px 14px;
  border-top: 1px solid var(--apk-border);
  color: var(--apk-faint);
  font-size: 10px;
}
.device-main {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  padding: 14px 16px;
}
.device-main > :not(.device-apps):not(.device-logcat):not(.device-files) { flex-shrink: 0; }
.device-apps { display: flex; min-height: 360px; flex: 1; flex-direction: column; gap: 12px; }
.device-logcat, .device-files { min-height: 360px; flex: 1; }
.device-logcat > :deep(*), .device-files > :deep(*) { height: 100%; }
.device-hero {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: linear-gradient(180deg, rgba(20, 34, 56, 0.65), rgba(12, 22, 38, 0.5));
}
.hero-icon {
  position: relative;
  display: grid;
  width: 58px;
  height: 58px;
  place-items: center;
  border: 1px solid rgba(56, 189, 248, 0.22);
  border-radius: 16px;
  background: rgba(56, 189, 248, 0.07);
  color: var(--apk-accent);
}
.hero-icon .state-dot {
  position: absolute;
  right: 7px;
  bottom: 7px;
  width: 8px;
  height: 8px;
  border: 2px solid var(--apk-panel);
}
.hero-text {
  min-width: 0;
  flex: 1;
}
.hero-text h2 {
  margin: 0 0 3px;
  color: var(--apk-text);
  font-size: 17px;
  font-weight: 650;
}
.hero-meta {
  color: var(--apk-accent);
  font-size: 11px;
}
.hero-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 7px;
}
.hero-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 7px;
}
.busy-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  padding: 9px 12px;
  border: 1px solid rgba(56, 189, 248, 0.2);
  border-radius: 8px;
  background: rgba(56, 189, 248, 0.07);
  color: #7dd3fc;
  font-size: 11.5px;
}
.action-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 12px;
  margin-top: 0;
  flex: 0 0 auto;
  align-items: start;
}
.action-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: rgba(12, 22, 38, 0.55);
}
.action-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--apk-text-soft);
  font-size: 12px;
  font-weight: 650;
}
.action-title svg {
  color: var(--apk-accent);
}
.action-card p {
  margin: 0;
  color: var(--apk-faint);
  font-size: 10.5px;
  line-height: 1.6;
}
.action-card p .apk-mono {
  color: var(--apk-muted);
}
.action-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}
.apps-card {
  display: flex;
  min-height: 160px;
  flex: 1;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: rgba(9, 17, 30, 0.55);
}
.apps-heading, .apps-toolbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--apk-border);
}
.apps-count { color: var(--apk-muted); font-size: 11px; font-weight: 400; }
.apps-filters { display: flex; gap: 4px; }
.apps-filters button {
  padding: 5px 8px;
  border: 1px solid transparent;
  border-radius: var(--apk-radius-sm);
  background: transparent;
  color: var(--apk-muted);
  cursor: pointer;
  font-size: 12px;
}
.apps-filters button:hover { background: rgba(148, 163, 184, 0.08); color: var(--apk-text); }
.apps-filters button.active { background: rgba(56, 189, 248, 0.13); color: var(--apk-accent); }
.apps-filters button:focus-visible { outline: 2px solid var(--apk-accent); }
.apps-search { display: flex; min-width: 180px; flex: 1; align-items: center; gap: 6px; color: var(--apk-muted); }
.apps-search-input { min-width: 0; flex: 1; }
.apps-search > svg, .app-icon { flex: 0 0 auto; }
.apps-list { min-height: 0; flex: 1; margin: 0; padding: 0; overflow-y: auto; list-style: none; }
.app-row { display: flex; min-height: 48px; align-items: center; gap: 10px; padding: 8px 14px; }
.app-row + .app-row { border-top: 1px solid var(--apk-border); }
.app-row:hover { background: rgba(148, 163, 184, 0.04); }
.app-icon { color: var(--apk-muted); }
.app-info { display: flex; min-width: 0; flex: 1; flex-direction: column; gap: 3px; }
.app-name { overflow: hidden; color: var(--apk-text-soft); font-size: 13px; font-weight: 500; text-overflow: ellipsis; white-space: nowrap; }
.app-package { overflow: hidden; color: var(--apk-muted); font-size: 11px; text-overflow: ellipsis; white-space: nowrap; }
.apps-state { display: flex; min-height: 100px; align-items: center; justify-content: center; flex-wrap: wrap; gap: 8px; padding: 20px; color: var(--apk-muted); font-size: 12px; overflow-wrap: anywhere; }
.apps-state.error { color: var(--apk-red); }
.tips-card {
  margin-top: 0;
  padding: 14px;
  border: 1px solid var(--apk-border);
  border-radius: var(--apk-radius);
  background: rgba(148, 163, 184, 0.035);
}
.tips-card strong {
  color: var(--apk-text-soft);
  font-size: 12px;
}
.tips-card ul {
  margin: 7px 0 0;
  padding-left: 17px;
  color: var(--apk-faint);
  font-size: 10.5px;
  line-height: 1.8;
}
.device-empty {
  display: flex;
  height: 100%;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 30px;
  text-align: center;
}
.empty-orbit {
  display: grid;
  width: 92px;
  height: 92px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px dashed rgba(56, 189, 248, 0.28);
  border-radius: 28px;
  background: radial-gradient(circle, rgba(56, 189, 248, 0.12), transparent 68%);
  color: var(--apk-accent);
}
.device-empty h2 {
  margin: 0;
  color: var(--apk-text);
  font-size: 16px;
  font-weight: 650;
}
.device-empty p {
  max-width: 420px;
  margin: 0;
  color: var(--apk-faint);
  font-size: 11.5px;
  line-height: 1.65;
}
@media (max-width: 1000px) {
  .devices-page {
    grid-template-columns: var(--pane-size, minmax(270px, 320px)) minmax(0, 1fr);
  }
}
</style>
