<script setup lang="ts">
import { computed, ref, watch } from "vue"
import { ArrowUp, File, Folder, RefreshCw } from "lucide-vue-next"
import { formatBytes, formatDateTime, type DeviceFileEntry } from "@mobiletoolbox/protocol"
import { adb, normalizeDevicePath } from "../../platform/adb"
import { errorText } from "../../utils/errors"
import { log } from "../../log"

const props = defineProps<{ deviceId: string | null }>()
const path = ref("/")
const inputPath = ref(path.value)
const entries = ref<DeviceFileEntry[]>([])
const loading = ref(false)
const error = ref("")
const keyword = ref("")
let request = 0
const visible = computed(() => entries.value.filter(entry => entry.name.toLowerCase().includes(keyword.value.trim().toLowerCase())))

async function browse(target: string): Promise<void> {
  const token = ++request
  const deviceId = props.deviceId
  if (!deviceId) { entries.value = []; loading.value = false; error.value = "请选择在线设备"; return }
  loading.value = true
  error.value = ""
  try {
    const normalized = normalizeDevicePath(target)
    const result = await adb.listFiles(deviceId, normalized)
    if (token !== request) return
    entries.value = result
    path.value = normalized
    inputPath.value = normalized
  } catch (cause) {
    if (token !== request) return
    entries.value = []
    error.value = errorText(cause)
    log.error("设备文件", error.value)
  } finally {
    if (token === request) loading.value = false
  }
}
function open(entry: DeviceFileEntry): void {
  if (entry.directory) void browse(`${path.value}/${entry.name}`)
}
watch(() => props.deviceId, () => { path.value = "/"; inputPath.value = path.value; keyword.value = ""; void browse(path.value) }, { immediate: true })
</script>

<template>
  <section class="device-files" aria-label="Android 设备文件管理">
    <div class="files-toolbar">
      <SButton size="small" :disabled="loading || !deviceId || path === '/'" @click="browse(`${path}/..`)"><ArrowUp :size="14" />上级</SButton>
      <SInput v-model:value="inputPath" class="path-input" size="small" aria-label="设备目录路径" placeholder="/sdcard" spellcheck="false" @keydown.enter.prevent="browse(inputPath)" />
      <SButton size="small" :disabled="loading || !deviceId" @click="browse(inputPath)">转到</SButton>
      <SButton size="small" :loading="loading" :disabled="!deviceId" @click="browse(path)"><RefreshCw :size="13" />刷新</SButton>
    </div>
    <div class="files-toolbar">
      <SButton v-for="target in ['/sdcard', '/sdcard/Download', '/sdcard/DCIM', '/']" :key="target" size="small" :disabled="loading || !deviceId" @click="browse(target)">{{ target === '/' ? '根目录' : target.split('/').pop() }}</SButton>
      <SInput v-model:value="keyword" class="files-search" size="small" clearable placeholder="筛选文件名…" aria-label="筛选设备文件" />
    </div>
    <SAlert v-if="error" type="error" class="files-error">{{ error }}</SAlert>
    <div class="files-table-wrap" :aria-busy="loading">
      <div v-if="loading" class="files-state"><span class="apk-spinner" />正在读取目录…</div>
      <div v-else-if="!error && !visible.length" class="files-state">{{ keyword.trim() ? '没有匹配的文件' : '目录为空' }}</div>
      <table v-else-if="!error" class="files-table">
        <thead><tr><th scope="col">名称</th><th scope="col">类型</th><th scope="col">大小</th><th scope="col">修改时间</th></tr></thead>
        <tbody><tr v-for="entry in visible" :key="entry.name">
          <td><button v-if="entry.directory" class="file-name directory" type="button" @click="open(entry)"><Folder :size="16" />{{ entry.name }}</button><span v-else class="file-name"><File :size="16" />{{ entry.name }}</span></td>
          <td>{{ entry.directory ? '文件夹' : entry.symbolicLink ? '链接' : '文件' }}</td>
          <td class="apk-mono">{{ entry.directory ? '—' : formatBytes(entry.size) }}</td>
          <td>{{ formatDateTime(entry.modifiedAt) }}</td>
        </tr></tbody>
      </table>
    </div>
    <footer>{{ path }} · {{ visible.length }} / {{ entries.length }} 项</footer>
  </section>
</template>

<style scoped>
.device-files { display: flex; height: 100%; min-width: 0; min-height: 0; flex-direction: column; overflow: hidden; border: 1px solid var(--apk-border); border-radius: var(--apk-radius); background: var(--apk-panel-soft); }
.files-toolbar { display: flex; flex: 0 0 auto; align-items: center; gap: 8px; padding: 10px 12px; border-bottom: 1px solid var(--apk-border); }
.path-input { flex: 1; min-width: 0; }
.files-search { margin-left: auto; width: 220px; min-width: 100px; }
.files-error { flex: 0 0 auto; margin: 10px 12px; }
.files-table-wrap { flex: 1; min-height: 0; overflow: auto; }
.files-state { display: flex; justify-content: center; align-items: center; gap: 8px; padding: 32px; color: var(--apk-muted); font-size: 12px; }
.files-table { width: 100%; border-collapse: collapse; font-size: 12px; text-align: left; }
.files-table th { position: sticky; top: 0; z-index: 1; background: var(--apk-panel); color: var(--apk-muted); font-weight: 500; }
.files-table td, .files-table th { padding: 9px 12px; border-bottom: 1px solid var(--apk-border); }
.files-table td:not(:first-child) { color: var(--apk-muted); white-space: nowrap; }
.file-name { display: flex; align-items: center; gap: 8px; padding: 0; border: 0; background: transparent; color: var(--apk-text-soft); text-align: left; overflow-wrap: anywhere; }
.file-name svg { flex: 0 0 auto; color: var(--apk-muted); }
.directory { color: var(--apk-accent); cursor: pointer; }
.directory:focus-visible { outline: 2px solid var(--apk-accent); outline-offset: 3px; }
.directory:hover { text-decoration: underline; }
footer { flex: 0 0 auto; padding: 8px 12px; border-top: 1px solid var(--apk-border); color: var(--apk-muted); font-size: 11px; overflow-wrap: anywhere; }
</style>
