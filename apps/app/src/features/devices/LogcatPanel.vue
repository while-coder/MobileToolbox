<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue"
import { Download, Play, Square, Terminal, Trash2 } from "lucide-vue-next"
import { lines, running, starting, stopping, error, savedPath, exporting, clearing, start, stop, exportDisplayed, exportComplete, openCache, clearCache } from "../../stores/logcatStore"

defineProps<{ deviceId: string | null }>()
const keyword = ref("")
const level = ref("V")
const follow = ref(true)
const body = ref<HTMLElement>()
const levels = ["V", "D", "I", "W", "E", "F"]
const levelOptions = levels.map(value => ({ value, label: `${value} 及以上` }))

function lineLevel(line: string): string {
  return line.match(/^\d{2}-\d{2}\s+\S+\s+\d+\s+\d+\s+([VDIWEF])\s/)?.[1] ?? ""
}
const filtered = computed(() => {
  const query = keyword.value.trim().toLowerCase()
  const minimum = levels.indexOf(level.value)
  return lines.value.filter(line => {
    const severity = lineLevel(line)
    return (!query || line.toLowerCase().includes(query)) &&
      (!severity || levels.indexOf(severity) >= minimum)
  })
})

watch([filtered, follow], async () => {
  if (!follow.value) return
  await nextTick()
  if (body.value) body.value.scrollTop = body.value.scrollHeight
})
</script>

<template>
  <section class="logcat-card" aria-label="ADB logcat">
    <header class="logcat-toolbar logcat-actions">
      <strong><Terminal :size="15" /> ADB logcat</strong>
      <span role="status">{{ starting ? '正在启动…' : stopping ? '正在停止…' : running ? '采集中' : '已停止' }}</span>
      <SButton v-if="!running" size="small" type="primary" :disabled="!deviceId || starting || stopping || clearing" @click="start">
        <Play :size="13" />开始
      </SButton>
      <SButton v-else size="small" :disabled="stopping" @click="stop"><Square :size="13" />停止</SButton>
      <SButton size="small" :loading="exporting" :disabled="!savedPath || starting || stopping || clearing" @click="exportComplete"><Download :size="13" />导出完整日志</SButton>
      <SButton size="small" :loading="exporting" :disabled="!filtered.length || starting || stopping || clearing" @click="exportDisplayed(filtered)"><Download :size="13" />导出当前显示</SButton>
      <SButton size="small" :disabled="!lines.length" @click="lines = []"><Trash2 :size="13" />清空显示</SButton>
      <SButton size="small" @click="openCache">打开日志缓存</SButton>
      <SButton size="small" :loading="clearing" :disabled="running || starting || stopping || exporting" @click="clearCache">清理日志缓存</SButton>
    </header>
    <div class="logcat-toolbar logcat-filters">
      <SInput v-model:value="keyword" size="small" clearable placeholder="搜索日志内容、Tag 或 PID…" aria-label="搜索 logcat" class="logcat-search" />
      <SSelect v-model:value="level" size="small" :options="levelOptions" aria-label="最低日志级别" class="logcat-level" />
      <label class="logcat-follow"><input v-model="follow" type="checkbox" />自动滚动</label>
      <span>{{ filtered.length }} / {{ lines.length }}</span>
    </div>
    <p v-if="error" class="logcat-error" role="alert">{{ error }}</p>
    <div ref="body" class="logcat-output" tabindex="0" aria-label="logcat 输出">
      <p v-if="!deviceId" class="logcat-empty">请选择在线设备并确认 USB 调试授权。</p>
      <p v-else-if="!lines.length" class="logcat-empty">{{ running ? '等待设备日志…' : '点击「开始」查看设备最近的日志和实时输出。' }}</p>
      <p v-else-if="!filtered.length" class="logcat-empty">没有匹配的日志。</p>
      <div v-for="(line, index) in filtered" :key="index" :class="['logcat-line', `level-${lineLevel(line)}`]">{{ line }}</div>
    </div>
    <footer><span>界面保留最近 5,000 行 · 切换页面继续采集 · 清空显示不影响缓存</span><span v-if="savedPath" class="cache-path" :title="savedPath">缓存：{{ savedPath }}</span></footer>
  </section>
</template>

<style scoped>
.logcat-card { display: flex; min-width: 0; min-height: 0; height: 100%; flex-direction: column; border: 1px solid var(--apk-border); border-radius: var(--apk-radius); background: var(--apk-panel-soft); overflow: hidden; }
.logcat-card > :not(.logcat-output) { flex-shrink: 0; }
.logcat-toolbar { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; padding: 10px 14px; border-bottom: 1px solid var(--apk-border); }
.logcat-actions { flex-wrap: nowrap; overflow-x: auto; }
.logcat-actions > * { flex-shrink: 0; }
.logcat-filters { display: grid; grid-template-columns: minmax(100px, 1fr) 130px auto auto; gap: 8px; }
.logcat-toolbar strong { display: flex; align-items: center; gap: 6px; color: var(--apk-text); margin-right: auto; }
.logcat-toolbar > span, .logcat-follow, footer { font-size: 12px; color: var(--apk-muted); }
.logcat-search { min-width: 0; width: 100%; }
.logcat-level { width: 130px; }
.logcat-follow { display: flex; align-items: center; gap: 5px; cursor: pointer; }
.logcat-output { min-height: 0; flex: 1; overflow: auto; padding: 10px 14px; background: var(--apk-bg); font: 12px/1.6 var(--apk-mono); }
.logcat-output:focus-visible { outline: 1px solid var(--apk-accent); outline-offset: -2px; }
.logcat-line { white-space: pre; color: var(--apk-text-soft); }
.level-W { color: var(--apk-amber); }
.level-E, .level-F, .logcat-error { color: var(--apk-red); }
.logcat-error { padding: 10px 14px; margin: 0; overflow-wrap: anywhere; }
.logcat-empty { color: var(--apk-muted); white-space: normal; }
footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 14px; border-top: 1px solid var(--apk-border); }
.cache-path { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
