import type { ImageRecord } from '@/api/client'

/** The text shown for a version: what its edit did, or "Original" for the first one. */
export function versionTitle(version: ImageRecord): string {
  return version.edit?.summary ?? 'Original'
}

/** A project's display name, falling back to a friendly default. */
export function projectName(version: Pick<ImageRecord, 'name'>): string {
  return version.name ?? 'Untitled'
}

/** A file name for a download, safe on every platform and ending in `extension`. */
export function downloadName(name: string, extension: string): string {
  const base = name
    .normalize('NFKD')
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase()
  return `${base || 'hands-off'}.${extension}`
}

/** The JSON edit log of a project: one entry per version, original first. */
export function editLog(versions: readonly ImageRecord[]): string {
  const entries = versions.map((version, index) => ({
    version: index,
    image_id: version.id,
    created_at: version.created_at,
    edit: version.edit ?? null,
  }))
  return JSON.stringify(
    { name: versions[0] ? projectName(versions[0]) : null, versions: entries },
    null,
    2,
  )
}

const UNITS: readonly (readonly [Intl.RelativeTimeFormatUnit, number])[] = [
  ['day', 86_400],
  ['hour', 3_600],
  ['minute', 60],
]

/** "3 hours ago", "yesterday" and the like, in the user's language. */
export function timeAgo(iso: string, now: number = Date.now()): string {
  const seconds = Math.round((Date.parse(iso) - now) / 1000)
  const format = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) return format.format(Math.round(seconds / size), unit)
  }
  return format.format(0, 'second')
}
