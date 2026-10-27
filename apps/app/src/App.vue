<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue"
import { PackageOpen, RefreshCw, Upload } from "lucide-vue-next"
import type { UnlistenFn } from "@tauri-apps/api/event"
import { getCurrentWebview } from "@tauri-apps/api/webview"
import { toast } from "@qingfeng346/ui-kit"
import type {
  LoadDirectory,
  LoadImage,
  LoadText,
  PackageInfo,
} from "@mobiletoolbox/protocol"
import AppShell from "@mobiletoolbox/ui/shell"
import PackageList from "@mobiletoolbox/ui/package-list"
import PackageDetails from "@mobiletoolbox/ui/package-details"
import DevicePanel from "@mobiletoolbox/ui/devices"
import SettingsPanel from "@mobiletoolbox/ui/settings"
import { log } from "./log"
import LogcatPanel from "./features/devices/LogcatPanel.vue"
import DeviceFilesPanel from "./features/devices/DeviceFilesPanel.vue"
import type { ViewId } from "@mobiletoolbox/ui/types"
import { native } from "./platform/native"
import { errorText } from "./utils/errors"
import UpdaterDialog from "./features/updater/UpdaterDialog.vue"
import { useTauriUpdater } from "./features/updater/useTauriUpdater"
import { useNativeAppMenu } from "./composables/useNativeAppMenu"
import {
  activeView,
  environment,
  environmentError,
  environmentLoading,
  handleProgress,
  loadEnvironment,
  runningList,
} from "./stores/appStore"
import {
  busy,
  busyStage,
  buildJar,
  clearAll,
  decodePackage,
  decompilePackage,
  iconFor,
  icons,
  importFromPaths,
  importing,
  loadPackages,
  loading as packagesLoading,
  packages,
  pickAndImport,
  query,
  removePackage,
  selectedId,
  selectedPackage,
  selectPackage,
} from "./stores/packagesStore"
import {
  appsError,
  appsLoading,
  busyText,
  clearInstalledApps,
  connectDevice,
  connecting,
  devices,
  disconnectDevice,
  installPackage,
  installPaths,
  installedApps,
  loading as devicesLoading,
  onlineCount,
  pickAndInstall,
  pickAndPushObb,
  refreshDevices,
  refreshInstalledApps,
  selectedDevice,
  selectedDeviceId,
  selectDevice,
  uninstallPackage,
} from "./stores/devicesStore"

watch(() => activeView.value === "devices" && selectedDevice.value?.state === "device"
  ? selectedDevice.value.id : null, (deviceId) => {
  if (deviceId) void refreshInstalledApps()
  else clearInstalledApps()
}, { immediate: true })

const dragActive = ref(false)
const settingsVisible = ref(false)
const settingsTab = ref("environment")
const unlisteners: UnlistenFn[] = []

const views: Record<ViewId, { title: string; subtitle: string }> = {
  packages: { title: "安装包分析", subtitle: "导入 APK，查看清单、资源与反编译源码。" },
  devices: { title: "设备管理", subtitle: "通过 adb 管理真机与模拟器，管理设备应用并查看 logcat。" },
}

const pageTitle = computed(() => views[activeView.value].title)
const pageSubtitle = computed(() => views[activeView.value].subtitle)
const appVersion = computed(() => environment.value?.appVersion ?? "0.1.0")

const updater = useTauriUpdater()
const { updateStatus, updateStatusText, updaterSupported, checkForUpdate } = updater
const { active: nativeMenuActive } = useNativeAppMenu(() => openSettings())

/** 导入任务在拿到安装包 id 之前只有文件名，因此用 id 为空来识别。 */
const importTask = computed(() => runningList.value.find((task) => task.id === null) ?? null)
const importPercent = computed(() => {
  const ratio = importTask.value?.ratio
  if (typeof ratio !== "number" || !Number.isFinite(ratio)) return null
  return Math.max(0, Math.min(100, Math.round(ratio * 100)))
})

const statusTone = computed<"ok" | "warn" | "error">(() => {
  if (!native.isTauri) return "warn"
  if (!environment.value) return environmentError.value ? "error" : "ok"
  if (!environment.value.java.available) return "error"
  const required = [
    environment.value.apktool,
    environment.value.dex2jar,
    environment.value.cfr,
  ]
  return required.some((tool) => !tool.available) ? "warn" : "ok"
})

const statusText = computed(() => {
  if (!native.isTauri) return "Web 预览模式"
  if (!environment.value) {
    return environmentError.value ? "环境检测失败" : "环境检测中"
  }
  if (!environment.value.java.available) return "Java 未就绪"
  const required = [
    environment.value.apktool,
    environment.value.dex2jar,
    environment.value.cfr,
  ]
  return required.some((tool) => !tool.available) ? "部分工具缺失" : "工具链就绪"
})

const dropTitle = computed(() =>
  activeView.value === "devices" ? "松开以安装 APK" : "松开以导入 APK",
)
const dropSubtitle = computed(() =>
  activeView.value === "devices"
    ? "文件会安装到当前选中的在线设备"
    : "支持一次拖入多个 .apk 文件",
)

const loadDirectory: LoadDirectory = (root, relativePath) => {
  const pkg = selectedPackage.value
  if (!pkg) return Promise.reject(new Error("尚未选择安装包"))
  return native.listPackageDirectory(pkg.id, root, relativePath)
}

const loadText: LoadText = (root, relativePath) => {
  const pkg = selectedPackage.value
  if (!pkg) return Promise.reject(new Error("尚未选择安装包"))
  return native.readPackageText(pkg.id, root, relativePath)
}

const loadImage: LoadImage = (root, relativePath) => {
  const pkg = selectedPackage.value
  if (!pkg) return Promise.reject(new Error("尚未选择安装包"))
  return native.readPackageImage(pkg.id, root, relativePath)
}

function navigate(next: ViewId): void {
  activeView.value = next
  if (next === "devices") void refreshDevices(true)
}

function openSettings(tab = "environment"): void {
  settingsTab.value = tab
  settingsVisible.value = true
  void loadEnvironment()
}

function handleQuery(value: string): void {
  query.value = value
}

function decode(pkg: PackageInfo): void {
  void decodePackage(pkg)
}

function makeJar(pkg: PackageInfo): void {
  void buildJar(pkg)
}

function decompile(pkg: PackageInfo): void {
  void decompilePackage(pkg)
}

function openFolder(pkg: PackageInfo, kind: "root" | "source" | "java"): void {
  native.openPackageFolder(pkg.id, kind).catch((error) => {
    toast.show("error", `打开目录失败：${errorText(error)}`)
  })
}

function reveal(pkg: PackageInfo): void {
  native.revealPackage(pkg.id).catch((error) => {
    toast.show("error", `定位文件失败：${errorText(error)}`)
  })
}

function remove(pkg: PackageInfo): void {
  void removePackage(pkg)
}

function install(pkg: PackageInfo): void {
  const device = selectedDevice.value
  if (!device) {
    toast.show("warning", "请先在「设备」页连接并选择一台设备。")
    activeView.value = "devices"
    void refreshDevices(true)
    return
  }
  if (device.state !== "device") {
    toast.show("warning", "所选设备不在线，请检查连接状态。")
    return
  }
  void installPackage(device.id, pkg)
}

function installFromLibrary(packageId: string): void {
  const pkg = packages.value.find((item) => item.id === packageId)
  if (!pkg) {
    toast.show("warning", "安装包不存在，请刷新安装包库。")
    return
  }
  install(pkg)
}

function pickDeviceInstall(): void {
  if (!selectedDeviceId.value) {
    toast.show("warning", "请先选择一台设备。")
    return
  }
  void pickAndInstall(selectedDeviceId.value)
}

function pickDeviceObb(): void {
  if (!selectedDeviceId.value) {
    toast.show("warning", "请先选择一台设备。")
    return
  }
  void pickAndPushObb(selectedDeviceId.value)
}

function uninstallFromDevice(packageName: string): void {
  if (!selectedDeviceId.value) {
    toast.show("warning", "请先选择一台设备。")
    return
  }
  void uninstallPackage(selectedDeviceId.value, packageName)
}

function openWorkDir(): void {
  const path = environment.value?.workDir
  if (!path) {
    toast.show("warning", "尚未获取到工作目录。")
    return
  }
  native.openPath(path).catch((error) => {
    toast.show("error", `打开目录失败：${errorText(error)}`)
  })
}

function handleDrop(paths: string[]): void {
  const apks = paths.filter((path) => path.toLowerCase().endsWith(".apk"))
  if (apks.length === 0) {
    toast.show("warning", "只支持拖入 .apk 文件。")
    return
  }
  if (activeView.value === "devices") {
    const device = selectedDevice.value
    if (!device || device.state !== "device") {
      toast.show("warning", "请先选择一台在线设备。")
      return
    }
    void installPaths(device.id, apks)
    return
  }
  void importFromPaths(apks)
}

function onKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "o") {
    event.preventDefault()
    void pickAndImport()
  }
}

async function setupNative(): Promise<void> {
  if (native.isTauri) {
    unlisteners.push(await native.onLog(log.write))
    unlisteners.push(await native.onProgress(handleProgress))
    try {
      const webview = getCurrentWebview()
      unlisteners.push(
        await webview.onDragDropEvent((event) => {
          const payload = event.payload
          if (payload.type === "enter" || payload.type === "over") {
            dragActive.value = true
            return
          }
          if (payload.type === "leave") {
            dragActive.value = false
            return
          }
          dragActive.value = false
          if (payload.type === "drop" && payload.paths.length > 0) {
            handleDrop(payload.paths)
          }
        }),
      )
    } catch (error) {
      log.error("拖拽", errorText(error))
    }
  }

  await Promise.all([loadEnvironment(), loadPackages(), refreshDevices(true)])
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown)
  void setupNative()
})

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown)
  for (const unlisten of unlisteners) unlisten()
  unlisteners.length = 0
})
</script>

<template>
  <div class="app-root">
    <AppShell
      :title="pageTitle"
      :subtitle="pageSubtitle"
      :active-view="activeView"
      :app-version="appVersion"
      :package-count="packages.length"
      :device-count="devices.length"
      :status-tone="statusTone"
      :status-text="statusText"
      :native-menu-active="nativeMenuActive"
      @navigate="navigate"
      @settings="openSettings()"
    >
      <template #actions>
        <template v-if="activeView === 'packages'">
          <SButton size="small" :loading="packagesLoading" @click="loadPackages">
            <RefreshCw v-if="!packagesLoading" :size="13" />
            刷新
          </SButton>
          <SButton type="primary" size="small" :loading="importing" @click="pickAndImport">
            <Upload v-if="!importing" :size="13" />
            {{ importing ? (importPercent === null ? "解析中" : `解析中 ${importPercent}%`) : "导入 APK" }}
          </SButton>
        </template>
        <SButton
          v-else-if="activeView === 'devices'"
          size="small"
          :loading="devicesLoading"
          @click="refreshDevices(false)"
        >
          <RefreshCw v-if="!devicesLoading" :size="13" />
          刷新设备
        </SButton>
      </template>

      <template #library>
        <PackageList
          :packages="packages"
          :selected-id="selectedId"
          :icons="icons"
          :busy="busy"
          :query="query"
          :importing="importing"
          :import-task="importTask"
          @select="selectPackage"
          @import="pickAndImport"
          @delete="remove"
          @reveal="reveal"
          @update:query="handleQuery"
        />
      </template>
      <template #packages>
        <PackageDetails
          :pkg="selectedPackage"
          :icon-url="iconFor(selectedId)"
          :busy-stage="busyStage"
          :device-count="onlineCount"
          :load-directory="loadDirectory"
          :load-text="loadText"
          :load-image="loadImage"
          @decode="decode"
          @build-jar="makeJar"
          @decompile="decompile"
          @open-folder="openFolder"
          @reveal="reveal"
          @install="install"
          @delete="remove"
        />
      </template>

      <template #devices>
        <DevicePanel
          :devices="devices"
          :selected-id="selectedDeviceId"
          :loading="devicesLoading"
          :connecting="connecting"
          :busy-text="busyText"
          :packages="packages"
          :installed-apps="installedApps"
          :apps-loading="appsLoading"
          :apps-error="appsError"
          @refresh="refreshDevices(false)"
          @refresh-apps="refreshInstalledApps"
          @select="selectDevice"
          @connect="connectDevice"
          @disconnect="disconnectDevice"
          @pick-install="pickDeviceInstall"
          @pick-obb="pickDeviceObb"
          @install-library="installFromLibrary"
          @uninstall="uninstallFromDevice"
        >
          <template #logcat>
            <LogcatPanel :device-id="selectedDevice?.state === 'device' ? selectedDevice.id : null" />
          </template>
          <template #files="{ path }">
            <DeviceFilesPanel :device-id="selectedDevice?.state === 'device' ? selectedDevice.id : null" :initial-path="path" />
          </template>
        </DevicePanel>
      </template>

    </AppShell>

    <SModal :show="settingsVisible" preset="card" title="设置"
      :style="{ width: 'min(880px, calc(100vw - 32px))' }"
      @update:show="settingsVisible = $event">
      <div class="settings-dialog-content">
        <SettingsPanel
          v-model:tab="settingsTab"
          :app-version="appVersion"
          :environment="environment"
          :packages-count="packages.length"
          :loading="environmentLoading"
          :update-status="updateStatus"
          :update-status-text="updateStatusText"
          :updater-supported="updaterSupported"
          @refresh="loadEnvironment"
          @open-workdir="openWorkDir"
          @clear-all="clearAll"
          @check-update="updateStatus === 'installed' ? updater.relaunchApp() : checkForUpdate({ silent: false })"
        />
      </div>
    </SModal>

    <UpdaterDialog v-if="native.isTauri" :current-version="appVersion" />

    <Transition name="drop">
      <div v-if="dragActive" class="drop-overlay">
        <div class="drop-card">
          <div class="drop-icon"><PackageOpen :size="34" :stroke-width="1.4" /></div>
          <strong>{{ dropTitle }}</strong>
          <span>{{ dropSubtitle }}</span>
        </div>
      </div>
    </Transition>

    <SMessageHost />
  </div>
</template>

<style scoped>
.app-root {
  width: 100%;
  height: 100%;
}

.settings-dialog-content {
  height: min(600px, calc(100dvh - 148px));
  min-height: 0;
}

.drop-overlay {
  position: fixed;
  z-index: 90;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(4, 10, 20, 0.74);
  backdrop-filter: blur(6px);
  pointer-events: none;
}

.drop-card {
  display: flex;
  width: min(420px, 82vw);
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 44px 32px;
  border: 1px dashed rgba(56, 189, 248, 0.55);
  border-radius: 18px;
  background: linear-gradient(160deg, rgba(15, 26, 43, 0.96), rgba(9, 17, 30, 0.96));
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  color: var(--apk-text);
}
.drop-icon {
  display: grid;
  width: 68px;
  height: 68px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px solid rgba(56, 189, 248, 0.3);
  border-radius: 18px;
  background: rgba(56, 189, 248, 0.1);
  color: var(--apk-accent);
}
.drop-card strong {
  font-size: 17px;
  font-weight: 650;
}
.drop-card span {
  color: var(--apk-muted);
  font-size: 12.5px;
}

.drop-enter-active,
.drop-leave-active {
  transition: opacity 0.16s ease;
}
.drop-enter-from,
.drop-leave-to {
  opacity: 0;
}

</style>
