<script setup lang="ts">
import ResizeHandle from "./ResizeHandle.vue"
import { computed } from "vue"
import { PackageSearch, Settings2, Smartphone } from "lucide-vue-next"
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
}>(), {
  subtitle: "",
  packageCount: 0,
  deviceCount: 0,
  statusTone: "ok",
  statusText: "",
})

const emit = defineEmits<{ navigate: [view: ViewId] }>()

const navItems = computed(() => [
  { id: "packages" as const, label: "安装包", icon: PackageSearch, count: props.packageCount },
  { id: "devices" as const, label: "设备", icon: Smartphone, count: props.deviceCount },
  { id: "settings" as const, label: "设置", icon: Settings2, count: 0 },
])
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <ResizeHandle label="调整菜单宽度" :min="72" :max="360" :remaining="600" />
      <div class="brand">
        <div class="brand-mark" aria-hidden="true">
          <svg viewBox="0 0 32 32" fill="none">
            <defs>
              <linearGradient id="mobiletoolbox-mark" x1="4" y1="2" x2="29" y2="30" gradientUnits="userSpaceOnUse">
                <stop stop-color="#7dd3fc" />
                <stop offset="0.55" stop-color="#38bdf8" />
                <stop offset="1" stop-color="#a78bfa" />
              </linearGradient>
            </defs>
            <path d="M16 2.8 27.4 9.2v13.6L16 29.2 4.6 22.8V9.2L16 2.8Z" stroke="url(#mobiletoolbox-mark)" stroke-width="1.5" stroke-linejoin="round" />
            <path d="M10.2 21.5V10.5l5.8 5.9 5.8-5.9v11" stroke="url(#mobiletoolbox-mark)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M6.6 15.7h18.8" stroke="url(#mobiletoolbox-mark)" stroke-width="0.7" stroke-dasharray="1.5 2.5" opacity="0.85" />
          </svg>
        </div>
        <div class="brand-text">
          <strong>MobileToolbox</strong>
          <span>移动工具箱</span>
        </div>
      </div>

      <nav class="nav" aria-label="主导航">
        <button
          v-for="item in navItems"
          :key="item.id"
          type="button"
          class="nav-item"
          :class="{ active: activeView === item.id }"
          :title="item.label"
          @click="emit('navigate', item.id)"
        >
          <component :is="item.icon" :size="18" :stroke-width="1.8" />
          <span class="nav-label">{{ item.label }}</span>
          <span v-if="item.count > 0" class="nav-count">{{ item.count > 999 ? "999+" : item.count }}</span>
        </button>
      </nav>

      <div class="sidebar-foot">
        <div class="env-line" :title="statusText">
          <span class="env-dot" :class="statusTone" />
          <span class="env-text">{{ statusText || "环境检测中" }}</span>
        </div>
        <span class="version">v{{ appVersion }}</span>
      </div>
    </aside>

    <main class="main">
      <header class="topbar">
        <div class="topbar-title">
          <h1>{{ title }}</h1>
          <p v-if="subtitle">{{ subtitle }}</p>
        </div>
        <div class="topbar-actions">
          <slot name="actions" />
        </div>
      </header>

      <div class="content">
        <slot />
      </div>

      <div class="footer-dock">
        <slot name="footer" />
      </div>
    </main>
  </div>
</template>

<style scoped>
.shell {
  --pane-size: initial;
  display: grid;
  width: 100%;
  height: 100%;
  grid-template-columns: var(--pane-size, 216px) minmax(0, 1fr);
  background:
    radial-gradient(circle at 12% -12%, rgba(56, 189, 248, 0.12), transparent 34%),
    radial-gradient(circle at 92% 8%, rgba(167, 139, 250, 0.09), transparent 30%),
    var(--apk-bg);
}

.sidebar {
  position: relative;
  min-width: 0;
  container-type: inline-size;
  display: flex;
  min-height: 0;
  flex-direction: column;
  padding: 18px 12px 14px;
  border-right: 1px solid var(--apk-border);
  background: linear-gradient(180deg, rgba(12, 22, 38, 0.92), rgba(7, 13, 24, 0.96));
  backdrop-filter: blur(16px);
}

.brand {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 2px 8px 20px;
}
.brand-mark {
  display: grid;
  width: 38px;
  height: 38px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid rgba(56, 189, 248, 0.28);
  border-radius: 11px;
  background: linear-gradient(150deg, rgba(56, 189, 248, 0.16), rgba(167, 139, 250, 0.12));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1), 0 4px 18px rgba(56, 189, 248, 0.12);
}
.brand-mark svg {
  width: 28px;
  height: 28px;
}
.brand-text {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}
.brand-text strong {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 0.01em;
  color: var(--apk-text);
}
.brand-text span {
  overflow: hidden;
  color: var(--apk-muted);
  font-size: 11px;
  letter-spacing: 0.08em;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.nav-item {
  display: flex;
  width: 100%;
  min-height: 40px;
  align-items: center;
  gap: 10px;
  padding: 0 11px;
  border: 1px solid transparent;
  border-radius: var(--apk-radius-sm);
  background: transparent;
  color: var(--apk-muted);
  cursor: pointer;
  font-size: 13px;
  text-align: left;
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}
.nav-item:hover {
  background: rgba(148, 163, 184, 0.08);
  color: var(--apk-text);
}
.nav-item.active {
  border-color: rgba(56, 189, 248, 0.22);
  background: linear-gradient(90deg, rgba(56, 189, 248, 0.14), rgba(56, 189, 248, 0.045));
  color: #7dd3fc;
  box-shadow: inset 2px 0 0 var(--apk-accent);
}
.nav-item:focus-visible {
  outline: 2px solid var(--apk-accent);
  outline-offset: -2px;
}
.nav-label {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.nav-count {
  min-width: 20px;
  padding: 1px 6px;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.12);
  color: var(--apk-muted);
  font-family: var(--apk-mono);
  font-size: 10px;
  line-height: 16px;
  text-align: center;
}
.nav-item.active .nav-count {
  background: rgba(56, 189, 248, 0.18);
  color: #7dd3fc;
}

.sidebar-foot {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin-top: auto;
  padding: 12px 9px 0;
  border-top: 1px solid var(--apk-border);
}
.env-line {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 7px;
  color: var(--apk-muted);
  font-size: 11px;
}
.env-dot {
  width: 6px;
  height: 6px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: var(--apk-green);
  box-shadow: 0 0 0 3px rgba(52, 211, 153, 0.12);
}
.env-dot.warn {
  background: var(--apk-amber);
  box-shadow: 0 0 0 3px rgba(251, 191, 36, 0.12);
}
.env-dot.error {
  background: var(--apk-red);
  box-shadow: 0 0 0 3px rgba(251, 113, 133, 0.12);
}
.env-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.version {
  color: var(--apk-faint);
  font-family: var(--apk-mono);
  font-size: 10px;
}

.main {
  display: grid;
  min-width: 0;
  min-height: 0;
  /* 底部固定 34px 的状态条；展开后的日志面板用绝对定位向上覆盖，不参与布局。 */
  grid-template-rows: auto minmax(0, 1fr) 34px;
  overflow: hidden;
}

.footer-dock {
  position: relative;
  z-index: 20;
  min-width: 0;
}

.topbar {
  display: flex;
  min-height: 68px;
  align-items: center;
  gap: 16px;
  padding: 14px 20px 12px;
  border-bottom: 1px solid var(--apk-border);
  background: rgba(7, 13, 24, 0.5);
  backdrop-filter: blur(12px);
}
.topbar-title {
  min-width: 0;
  flex: 1;
}
.topbar-title h1 {
  margin: 0;
  color: var(--apk-text);
  font-size: 17px;
  font-weight: 650;
  letter-spacing: -0.01em;
}
.topbar-title p {
  margin: 3px 0 0;
  overflow: hidden;
  color: var(--apk-muted);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.topbar-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
}

.content {
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

@media (max-width: 1080px) {
  .shell {
    grid-template-columns: var(--pane-size, 72px) minmax(0, 1fr);
  }
}

@container (max-width: 120px) {
  .brand {
    justify-content: center;
    padding: 2px 0 20px;
  }
  .brand-text,
  .nav-label,
  .nav-count,
  .env-text,
  .version {
    display: none;
  }
  .nav-item {
    justify-content: center;
    padding: 0;
  }
  .sidebar-foot {
    align-items: center;
    padding: 12px 0 0;
  }
}
</style>