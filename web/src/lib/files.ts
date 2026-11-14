const SUPPORTED_TYPES = ['image/jpeg', 'image/png', 'image/webp']

/** True for files the server can read; the server still has the final say. */
export function isSupportedImage(file: Pick<File, 'type'>): boolean {
  return SUPPORTED_TYPES.includes(file.type)
}
