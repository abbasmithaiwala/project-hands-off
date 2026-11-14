/** Ask the browser to save `url` as `filename`. */
export function downloadUrl(url: string, filename: string): void {
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.click()
}

/** Save in-memory text as a file. */
export function downloadText(text: string, filename: string, type: string): void {
  const url = URL.createObjectURL(new Blob([text], { type }))
  downloadUrl(url, filename)
  URL.revokeObjectURL(url)
}
