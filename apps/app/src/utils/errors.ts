/** Tauri 的 invoke 失败时抛出的可能是字符串，这里统一成可读文案。 */
export function errorText(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message)
  }
  return "未知错误"
}

/** 同时兼容 / 与 \ 的文件名提取，用于展示拖入的文件路径。 */
export function baseName(path: string): string {
  const index = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"))
  return index >= 0 ? path.slice(index + 1) : path
}