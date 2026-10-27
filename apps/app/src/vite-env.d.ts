/// <reference types="vite/client" />

// 让模板中的 <SButton> 等全局组件获得类型提示
import type {} from "@qingfeng346/ui-kit"

declare interface Window {
  __TAURI_INTERNALS__?: unknown
}