<script setup lang="ts">
import { computed } from "vue"

const props = withDefaults(defineProps<{
  name?: string | null
  url?: string
  size?: number
  radius?: number
}>(), { name: "", url: "", size: 40, radius: 11 })

const initial = computed(() => {
  const value = (props.name || "?").trim()
  return Array.from(value)[0]?.toUpperCase() ?? "?"
})

// 用名称散列固定一个渐变，占位图标之间也能一眼区分
const hue = computed(() => {
  let hash = 0
  for (const char of props.name || "?") hash = (hash * 31 + (char.codePointAt(0) ?? 0)) % 360
  return hash
})

const style = computed(() => ({
  width: `${props.size}px`,
  height: `${props.size}px`,
  borderRadius: `${props.radius}px`,
  fontSize: `${Math.max(12, props.size * 0.38)}px`,
  background: props.url
    ? "rgba(255, 255, 255, 0.045)"
    : `linear-gradient(140deg, hsl(${hue.value} 58% 44%), hsl(${(hue.value + 52) % 360} 70% 28%))`,
}))
</script>

<template>
  <div class="package-icon" :class="{ 'has-image': Boolean(url) }" :style="style" :title="name || undefined">
    <img v-if="url" :src="url" alt="" draggable="false" />
    <span v-else aria-hidden="true">{{ initial }}</span>
  </div>
</template>

<style scoped>
.package-icon {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: #f8fafc;
  font-weight: 700;
  letter-spacing: -0.02em;
  user-select: none;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.12), 0 2px 8px rgba(0, 0, 0, 0.28);
}
.package-icon.has-image {
  background: #fff;
  border-color: rgba(255, 255, 255, 0.12);
}
.package-icon img {
  width: 80%;
  height: 80%;
  object-fit: contain;
}
</style>