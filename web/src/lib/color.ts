/** Parse `#rgb` or `#rrggbb` into a 24-bit number, falling back to `fallback` for anything else. */
export function parseHexColor(value: string, fallback = 0xffffff): number {
  const hex = value.trim().replace(/^#/, '')
  const full = hex.length === 3 ? [...hex].map((c) => c + c).join('') : hex
  return /^[0-9a-fA-F]{6}$/.test(full) ? Number.parseInt(full, 16) : fallback
}

/** Read a CSS custom property from the document root as a colour number. */
export function readCssColor(name: string, fallback = 0xffffff): number {
  return parseHexColor(getComputedStyle(document.documentElement).getPropertyValue(name), fallback)
}
