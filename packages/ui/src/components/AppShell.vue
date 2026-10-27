<script setup lang="ts">
import { computed, reactive, ref } from "vue"
import { ChevronDown, PackageSearch, Settings2, Smartphone } from "lucide-vue-next"
import ResizeHandle from "./ResizeHandle.vue"
import type { ViewId } from "../types"

const props = withDefaults(defineProps<{
  title: string
  subtitle?: string
  activeView: ViewId
  appVersion: string
  packageCount?: number
  deviceCount?: number
  statusTone?: "ok" | "warn" | "error"
  statusText?: string
  nativeMenuActive?: boolean
}>(), { subtitle: "", packageCount: 0, deviceCount: 0, statusTone: "ok", statusText: "" })
const emit = defineEmits<{ navigate: [view: ViewId]; settings: [] }>()
const navItems = computed(() => [
  { id: "packages" as const, label: "安装包", icon: PackageSearch, count: props.packageCount },
  { id: "devices" as const, label: "设备（Android 设备）", icon: Smartphone, count: props.deviceCount },
])
const systemAnchor = ref<HTMLElement | null>(null)
const systemMenu = reactive({ show: false, x: 0, y: 0 })
const systemOptions = [
  { label: "设置", value: "settings" },
]
const menuWasOpen = ref(false)
function toggleSystemMenu(): void {
  if (menuWasOpen.value) { systemMenu.show = false; return }
  const rect = systemAnchor.value?.getBoundingClientRect()
  systemMenu.x = Math.max(4, Math.min(rect?.left ?? 0, window.innerWidth - 200))
  systemMenu.y = (rect?.bottom ?? 0) + 4
  systemMenu.show = true
}
function selectSystemItem(value: string): void {
  systemMenu.show = false
  if (value === "settings") emit("settings")
}
function navigate(view: ViewId): void {
  emit("navigate", view)
}
</script>

<template>
  <div class="shell">
    <header class="topbar">
      <div class="brand">
        <img src="/mobiletoolbox-icon.png" alt="" />
        <div><strong>MobileToolbox</strong><small>移动应用与设备工具箱</small></div>
      </div>
      <nav class="nav" aria-label="主导航">
        <button v-for="item in navItems" :key="item.id" type="button"
          :class="{ active: activeView === item.id }" :aria-current="activeView === item.id ? 'page' : undefined"
          @click="navigate(item.id)">
          <component :is="item.icon" :size="15" />
          {{ item.label }}<span v-if="item.count" class="nav-count">{{ item.count }}</span>
        </button>
      </nav>
      <div class="topbar-actions">
        <span v-if="!nativeMenuActive" ref="systemAnchor" @pointerdown.capture="menuWasOpen = systemMenu.show" @keydown.capture="menuWasOpen = systemMenu.show">
          <SButton size="small" aria-haspopup="menu" :aria-expanded="systemMenu.show" @click="toggleSystemMenu">
            <Settings2 :size="14" />系统<ChevronDown :size="12" />
          </SButton>
        </span>
        <SDropdown v-if="!nativeMenuActive" :show="systemMenu.show" :x="systemMenu.x" :y="systemMenu.y" :options="systemOptions"
          :on-clickoutside="() => { systemMenu.show = false }" @select="selectSystemItem" />
        <slot name="actions" />
      </div>
    </header>
    <main class="workspace-host" :aria-label="title">
      <div v-show="activeView === 'packages'" class="packages-layout">
        <aside class="package-library" aria-label="安装包库">
          <ResizeHandle label="调整安装包库宽度" :min="240" :max="560" :remaining="480" />
          <slot name="library" />
        </aside>
        <section class="package-analysis" aria-label="安装包分析"><slot name="packages" /></section>
      </div>
      <div v-show="activeView === 'devices'" class="devices-view"><slot name="devices" /></div>
    </main>
    <footer class="statusbar">
      <span class="status" :class="statusTone"><i />{{ statusText || "环境检测中" }}</span>
      <span class="status-hint">拖动分隔线调整宽度</span>
      <span>v{{ appVersion }}</span>
    </footer>
  </div>
</template>

<style scoped>
.shell { display: flex; width: 100%; height: 100%; min-width: 0; min-height: 0; flex-direction: column; background: var(--apk-bg); }
.topbar { display: flex; min-height: 64px; flex: 0 0 auto; align-items: center; gap: 20px; padding: 10px 16px; border-bottom: 1px solid var(--apk-border); background: var(--apk-panel-soft); }
.brand { display: flex; flex: 0 0 auto; align-items: center; gap: 10px; }
.brand img { width: 36px; height: 36px; object-fit: contain; }
.brand div { display: flex; flex-direction: column; gap: 3px; }
.brand strong { color: var(--apk-text); font-size: 16px; }
.brand small { color: var(--apk-muted); font-size: 11px; }
.nav { display: flex; align-items: center; gap: 4px; }
.nav button { display: flex; align-items: center; gap: 6px; padding: 8px 10px; border: 1px solid transparent; border-radius: 6px; background: transparent; color: var(--apk-muted); font-size: 12px; cursor: pointer; white-space: nowrap; }
.nav button:hover { background: var(--apk-panel-strong); color: var(--apk-text); }
.nav button.active { border-color: var(--apk-border-strong); background: var(--apk-accent-soft); color: var(--apk-accent); }
.nav button:focus-visible { outline: 2px solid var(--apk-accent); outline-offset: 2px; }
.nav-count { padding: 0 5px; border-radius: 8px; background: var(--apk-panel-strong); font-size: 10px; }
.topbar-actions { display: flex; min-width: 0; align-items: center; gap: 8px; margin-left: auto; flex-wrap: wrap; justify-content: flex-end; }
.workspace-host { display: flex; min-width: 0; min-height: 0; flex: 1; padding: 12px; }
.packages-layout { --pane-size: initial; display: grid; min-width: 0; min-height: 0; flex: 1; grid-template-columns: var(--pane-size, 320px) minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); border: 1px solid var(--apk-border); }
.package-library { position: relative; min-width: 0; min-height: 0; border-right: 1px solid var(--apk-border); }
.package-analysis { min-width: 0; min-height: 0; overflow: hidden; }
.devices-view { min-width: 0; min-height: 0; flex: 1; overflow: auto; border: 1px solid var(--apk-border); }
.statusbar { display: flex; min-height: 26px; flex: 0 0 auto; align-items: center; gap: 16px; padding: 4px 16px; border-top: 1px solid var(--apk-border); color: var(--apk-muted); font-size: 10px; }
.status { display: flex; min-width: 0; align-items: center; gap: 6px; }
.status i { width: 6px; height: 6px; flex: 0 0 auto; border-radius: 50%; background: var(--apk-green); }
.status.warn i { background: var(--apk-amber); }
.status.error i { background: var(--apk-red); }
.status-hint { margin-left: auto; }
@media (max-width: 1100px) {
  .topbar { flex-wrap: wrap; gap: 8px 16px; }
  .brand small { display: none; }
  .workspace-host { padding: 8px; }
  .status-hint { display: none; }
  .statusbar > :last-child { margin-left: auto; }
}
</style>
