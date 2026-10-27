import { onBeforeUnmount, onMounted, ref } from "vue"
import { isTauri } from "@tauri-apps/api/core"
import { Menu, MenuItem, Submenu } from "@tauri-apps/api/menu"
import { log } from "../log"

/** 与 STerm 一致：桌面端使用原生菜单，浏览器及初始化失败时保留页面菜单。 */
export function useNativeAppMenu(onSettings: () => void) {
  const supported = isTauri() && !/Android|iPhone|iPad|iPod/i.test(navigator.userAgent)
  const active = ref(supported)
  let disposed = false

  async function initialize(): Promise<void> {
    if (!supported) return
    try {
      const settings = await MenuItem.new({
        id: "app-menu:settings", text: "设置…", accelerator: "CmdOrCtrl+,",
        action: () => { if (!disposed) onSettings() },
      })
      const file = await Submenu.new({ text: "文件", items: [settings] })
      const menu = await Menu.new({ items: [file] })
      if (disposed) return
      if (/Mac/i.test(navigator.platform)) await menu.setAsAppMenu()
      else await menu.setAsWindowMenu()
      active.value = true
    } catch (error) {
      active.value = false
      log.error("系统菜单", `初始化失败：${String(error)}`)
    }
  }

  onMounted(() => { void initialize() })
  onBeforeUnmount(() => { disposed = true })
  return { active }
}
