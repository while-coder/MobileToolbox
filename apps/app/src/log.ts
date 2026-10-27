import type { LogEvent } from "@mobiletoolbox/protocol"

const MAX_ENTRIES = 800
const entries: LogEvent[] = []

function write(event: LogEvent): void {
  entries.push(event)
  if (entries.length > MAX_ENTRIES) entries.splice(0, entries.length - MAX_ENTRIES)
  const method = event.stream === "error" ? "error" : event.stream === "stderr" ? "warn" : "info"
  console[method](`[${new Date(event.time).toISOString()}] [${event.source}] ${event.text}`)
}

/** 应用内部日志统一入口；原生命令输出与前端诊断共用，不参与页面布局。 */
export const log = {
  write,
  info(source: string, text: string): void { write({ time: Date.now(), source, text, stream: "info" }) },
  warn(source: string, text: string): void { write({ time: Date.now(), source, text, stream: "stderr" }) },
  error(source: string, text: string): void { write({ time: Date.now(), source, text, stream: "error" }) },
  getEntries(): readonly LogEvent[] { return entries.slice() },
}
