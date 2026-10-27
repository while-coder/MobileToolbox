<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue"

const props = withDefaults(defineProps<{
  label: string
  horizontal?: boolean
  min?: number
  max?: number
  remaining?: number
}>(), { horizontal: false, min: 180, max: 640, remaining: 280 })

const handle = ref<HTMLElement | null>(null)
const dragging = ref(false)
const size = ref(0)
let observer: ResizeObserver | undefined
let cleanup: (() => void) | undefined
let adjusted = false

function host(): HTMLElement | null { return handle.value?.parentElement ?? null }
function limit(): number {
  const panel = host()
  if (!panel) return props.max
  const available = props.horizontal
    ? panel.getBoundingClientRect().bottom - 80
    : (panel.parentElement?.clientWidth ?? 0) - props.remaining
  return Math.max(0, Math.min(props.max, available))
}
function apply(value: number): void {
  const upper = limit()
  size.value = Math.round(Math.max(Math.min(props.min, upper), Math.min(upper, value)))
  const panel = host()
  const target = props.horizontal ? panel : panel?.parentElement
  target?.style.setProperty("--pane-size", `${size.value}px`)
  adjusted = true
}
function measure(): number {
  const rect = host()?.getBoundingClientRect()
  return (props.horizontal ? rect?.height : rect?.width) ?? 0
}
function start(event: PointerEvent): void {
  if (event.button !== 0 || dragging.value) return
  event.preventDefault()
  const element = handle.value!
  const origin = props.horizontal ? event.clientY : event.clientX
  const initial = measure()
  const cursor = document.body.style.cursor
  const selection = document.body.style.userSelect
  dragging.value = true
  document.body.style.cursor = props.horizontal ? "row-resize" : "col-resize"
  document.body.style.userSelect = "none"
  element.setPointerCapture(event.pointerId)
  const move = (next: PointerEvent) => {
    if (next.pointerId !== event.pointerId) return
    const delta = (props.horizontal ? next.clientY : next.clientX) - origin
    apply(initial + (props.horizontal ? -delta : delta))
  }
  const end = (next: PointerEvent) => {
    if (next.pointerId === event.pointerId) cleanup?.()
  }
  cleanup = () => {
    element.removeEventListener("pointermove", move)
    element.removeEventListener("pointerup", end)
    element.removeEventListener("pointercancel", end)
    element.removeEventListener("lostpointercapture", end)
    window.removeEventListener("blur", stop)
    if (element.hasPointerCapture(event.pointerId)) element.releasePointerCapture(event.pointerId)
    document.body.style.cursor = cursor
    document.body.style.userSelect = selection
    dragging.value = false
    cleanup = undefined
  }
  const stop = () => cleanup?.()
  element.addEventListener("pointermove", move)
  element.addEventListener("pointerup", end)
  element.addEventListener("pointercancel", end)
  element.addEventListener("lostpointercapture", end)
  window.addEventListener("blur", stop)
}
function keydown(event: KeyboardEvent): void {
  const increase = props.horizontal ? "ArrowUp" : "ArrowRight"
  const decrease = props.horizontal ? "ArrowDown" : "ArrowLeft"
  if (![increase, decrease, "Home", "End"].includes(event.key)) return
  event.preventDefault()
  apply(event.key === "Home" ? props.min : event.key === "End" ? limit()
    : measure() + (event.key === increase ? 1 : -1) * (event.shiftKey ? 40 : 10))
}
onMounted(() => {
  observer = new ResizeObserver(() => {
    if (adjusted) apply(size.value)
    else size.value = Math.round(measure())
  })
  const target = props.horizontal ? host()?.closest("main") : host()?.parentElement
  if (target) observer.observe(target)
})
onBeforeUnmount(() => { cleanup?.(); observer?.disconnect() })
</script>

<template>
  <div ref="handle" class="resize-handle" :class="{ horizontal, dragging }"
    role="separator" tabindex="0" :aria-label="label" :title="`${label}（方向键调整）`"
    :aria-orientation="horizontal ? 'horizontal' : 'vertical'"
    :aria-valuemin="Math.min(min, limit())" :aria-valuemax="limit()" :aria-valuenow="size"
    @pointerdown="start" @keydown="keydown" @click.stop />
</template>

<style scoped>
.resize-handle {
  position: absolute;
  z-index: 10;
  top: 0;
  right: -4px;
  bottom: 0;
  width: 8px;
  cursor: col-resize;
  touch-action: none;
}
.resize-handle::after {
  position: absolute;
  inset: 0 3px;
  content: "";
}
.resize-handle.horizontal {
  top: -4px;
  right: 0;
  bottom: auto;
  left: 0;
  width: auto;
  height: 8px;
  cursor: row-resize;
}
.resize-handle.horizontal::after { inset: 3px 0; }
.resize-handle:hover::after,
.resize-handle:focus-visible::after,
.resize-handle.dragging::after { background: var(--apk-accent); }
.resize-handle:focus-visible { outline: 1px solid var(--apk-accent); outline-offset: -1px; }
</style>
