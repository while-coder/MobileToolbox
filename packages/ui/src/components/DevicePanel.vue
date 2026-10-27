<script setup lang="ts">
import ResizeHandle from "./ResizeHandle.vue"
import { computed, ref, watch } from "vue"
import {
  Cable,
  FolderUp,
  PackageOpen,
  PlugZap,
  RefreshCw,
  Smartphone,
  Trash2,
  Usb,
} from "lucide-vue-next"
import { adbStateLabel, deviceLabel, localizedLabel, type AndroidDevice, type PackageInfo } from "@mobiletoolbox/protocol"
import { toast } from "@qingfeng346/ui-kit"

const props = withDefaults(defineProps<{
  devices: AndroidDevice[]
  selectedId: string | null
  loading: boolean
  connecting: boolean
  busyText?: string | null
  packages: PackageInfo[]
}>(), { busyText: "" })

const emit = defineEmits<{
  refresh: []
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
const uninstallName = ref("")

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

function submitUninstall(): void {
  if (!canAct.value) {
    toast.show("warning", "请先选择一台在线设备")
    return
  }
  const name = uninstallName.value.trim()
  if (!name) {
    toast.show("warning", "请输入要卸载的应用包名")
    return
  }
  emit("uninstall", name)
  uninstallName.value = ""
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
          <span class="device-main">
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
            </div>
          </div>

          <div class="action-card">
            <div class="action-title"><FolderUp :size="14" /> 推送 OBB</div>
            <p>选择 <span class="apk-mono">main.*.obb</span>，自动推送到 <span class="apk-mono">/sdcard/Android/obb/&lt;包名&gt;/</span>。</p>
            <div class="action-buttons single">
              <SButton size="small" :disabled="!canAct || busy" @click="emit('pick-obb')">
                <FolderUp :size="13" />
                选择 OBB 文件并推送
              </SButton>
            </div>
          </div>

          <div class="action-card">
            <div class="action-title"><Trash2 :size="14" /> 卸载应用</div>
            <p>输入设备上的应用包名，例如 <span class="apk-mono">com.example.app</span>。</p>
            <div class="connect-line">
              <SInput
                v-model:value="uninstallName"
                size="small"
                placeholder="应用包名"
                :disabled="busy"
                @keydown.enter="submitUninstall"
              />
              <SButton type="error" size="small" tertiary :disabled="!canAct || busy" @click="submitUninstall">
                卸载
              </SButton>
            </div>
          </div>
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
  </div>
</template>

<style scoped>
.devices-page {
  --pane-size: initial;
  display: grid;
  height: 100%;
  min-height: 0;
  grid-template-columns: var(--pane-size, minmax(300px, 360px)) minmax(0, 1fr);
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
.device-main {
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
.device-card > .device-main {
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
  overflow-y: auto;
  padding: 18px 20px 24px;
}
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
  margin-top: 14px;
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
.action-buttons.single {
  margin-top: 2px;
}
.tips-card {
  margin-top: 14px;
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