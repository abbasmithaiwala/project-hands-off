/**
 * Fill masked pixels with a smooth blend of the colours around them, so the spot an object
 * left behind reads as plain background in a preview. It is a cheap diffusion, not real
 * inpainting: good enough for a preview, never used for the final image.
 */
export function fillMasked(
  rgb: Uint8ClampedArray,
  masked: Uint8Array,
  width: number,
  height: number,
  iterations = 150,
): Uint8ClampedArray {
  const count = width * height
  const out = new Float32Array(count * 3)
  let knownCount = 0
  const mean = [0, 0, 0]
  for (let i = 0; i < count; i++) {
    for (let c = 0; c < 3; c++) out[i * 3 + c] = rgb[i * 3 + c] ?? 0
    if (!masked[i]) {
      knownCount++
      for (let c = 0; c < 3; c++) mean[c] = (mean[c] ?? 0) + (rgb[i * 3 + c] ?? 0)
    }
  }
  const result = new Uint8ClampedArray(rgb)
  if (knownCount === 0 || knownCount === count) return result

  for (let i = 0; i < count; i++) {
    if (masked[i]) for (let c = 0; c < 3; c++) out[i * 3 + c] = (mean[c] ?? 0) / knownCount
  }

  const next = new Float32Array(out)
  for (let step = 0; step < iterations; step++) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = y * width + x
        if (!masked[i]) continue
        const left = x > 0 ? i - 1 : i
        const right = x < width - 1 ? i + 1 : i
        const up = y > 0 ? i - width : i
        const down = y < height - 1 ? i + width : i
        for (let c = 0; c < 3; c++) {
          next[i * 3 + c] =
            ((out[left * 3 + c] ?? 0) +
              (out[right * 3 + c] ?? 0) +
              (out[up * 3 + c] ?? 0) +
              (out[down * 3 + c] ?? 0)) /
            4
        }
      }
    }
    out.set(next)
  }

  for (let i = 0; i < count; i++) {
    if (masked[i]) for (let c = 0; c < 3; c++) result[i * 3 + c] = Math.round(out[i * 3 + c] ?? 0)
  }
  return result
}

/** Grow a binary mask by `radius` pixels (square structuring element). */
export function dilate(
  mask: Uint8Array,
  width: number,
  height: number,
  radius: number,
): Uint8Array {
  if (radius <= 0) return new Uint8Array(mask)
  const horizontal = new Uint8Array(mask.length)
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      for (let dx = -radius; dx <= radius && !horizontal[y * width + x]; dx++) {
        const nx = x + dx
        if (nx >= 0 && nx < width && mask[y * width + nx]) horizontal[y * width + x] = 1
      }
    }
  }
  const out = new Uint8Array(mask.length)
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      for (let dy = -radius; dy <= radius && !out[y * width + x]; dy++) {
        const ny = y + dy
        if (ny >= 0 && ny < height && horizontal[ny * width + x]) out[y * width + x] = 1
      }
    }
  }
  return out
}
