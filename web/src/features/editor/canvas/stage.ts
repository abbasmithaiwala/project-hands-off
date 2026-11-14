import { drawGuide } from '@/lib/guide'
import {
  Application,
  Container,
  Graphics,
  MeshSimple,
  RenderTexture,
  Sprite,
  Texture,
} from 'pixi.js'
import type { DetectedObject } from '@/api/client'
import { dilate, fillMasked } from '@/lib/fill'
import { readCssColor } from '@/lib/color'
import { pickObject, type Point } from '@/lib/geometry'
import {
  deformPositions,
  gridMesh,
  objectRegion,
  PREVIEW_MARGIN,
  type Deformation,
  type GridMesh,
  type Rect,
} from '@/lib/warp'
import {
  fitViewport,
  imageToScreen,
  panBy,
  screenToImage,
  zoomAt,
  type Size,
  type Viewport,
} from '@/lib/viewport'

export interface StageCallbacks {
  onHover: (id: number | null) => void
  onSelect: (id: number | null) => void
  onViewChange: (view: Viewport) => void
}

/** What the React layer can ask of the canvas. */
export interface StageHandle {
  setImage: (url: string) => Promise<Size>
  setObjects: (objects: readonly DetectedObject[]) => void
  setHighlight: (hovered: number | null, selected: number | null) => void
  /** Show (or with `null`, remove) the warped preview of one object. */
  setPreview: (objectId: number, spec: PreviewSpec | null) => void
  /** Render the photo with the previews applied (no outlines or controls) as a PNG. */
  capturePreview: (maxSide: number) => Promise<Blob | null>
  /** Render the original photo with each shape (points in `0..1`) drawn over it as a PNG. */
  captureGuide: (maxSide: number, shapes: readonly (readonly Point[])[]) => Promise<Blob | null>
  fit: () => void
  zoomBy: (factor: number) => void
  destroy: () => void
}

/** What to preview for one object: its mask and how its pixels should move (in image pixels). */
export interface PreviewSpec {
  bbox: { x: number; y: number; width: number; height: number }
  maskUrl: string
  deform: Deformation | null
  /** Draw the moved pixels. Border-only previews still move the outline and are used when applying. */
  showPixels: boolean
}

function canvasToBlob(canvas: HTMLCanvasElement | OffscreenCanvas): Promise<Blob | null> {
  if ('convertToBlob' in canvas) return canvas.convertToBlob({ type: 'image/png' })
  return new Promise((resolve) => canvas.toBlob(resolve, 'image/png'))
}

export type CreateStage = (
  container: HTMLElement,
  callbacks: StageCallbacks,
) => Promise<StageHandle>

// Room kept clear for the top bar and the floating menus, so controls are never underneath them.
const FIT_PADDING = { top: 72, right: 64, bottom: 210, left: 64 }
const DRAG_THRESHOLD_PX = 4
const WHEEL_ZOOM_SPEED = 0.01
const BUTTON_ZOOM_FACTOR = 1.25
const PREVIEW_GRID = 28
const PREVIEW_TEXTURE_MAX = 1024
const FILL_RESOLUTION = 96
const FILL_GROW_PX = 2

interface PreviewLayer {
  container: Container
  mesh: MeshSimple
  grid: GridMesh
  maskUrl: string
}

function makeCanvas(width: number, height: number) {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d', { willReadFrequently: true })
  if (!context) throw new Error('2D canvas is not available')
  return { canvas, context }
}

/**
 * Cut the object out of the photo using its mask, build a smooth patch for the spot it leaves
 * behind, and a mesh to deform the cutout. The patch and mesh share one container.
 */
async function buildPreviewLayer(
  photo: ImageBitmap,
  imageSize: Size,
  spec: PreviewSpec,
): Promise<PreviewLayer> {
  const region = objectRegion(spec.bbox, imageSize, PREVIEW_MARGIN)
  const scale = Math.min(1, PREVIEW_TEXTURE_MAX / Math.max(region.width, region.height))
  const width = Math.max(1, Math.round(region.width * scale))
  const height = Math.max(1, Math.round(region.height * scale))

  const maskBitmap = await createImageBitmap(await (await fetch(spec.maskUrl)).blob())
  const crop = makeCanvas(width, height)
  crop.context.drawImage(
    photo,
    region.x,
    region.y,
    region.width,
    region.height,
    0,
    0,
    width,
    height,
  )

  const maskCrop = makeCanvas(width, height)
  maskCrop.context.filter = 'blur(1px)'
  maskCrop.context.drawImage(
    maskBitmap,
    (region.x * maskBitmap.width) / imageSize.width,
    (region.y * maskBitmap.height) / imageSize.height,
    (region.width * maskBitmap.width) / imageSize.width,
    (region.height * maskBitmap.height) / imageSize.height,
    0,
    0,
    width,
    height,
  )
  maskBitmap.close()

  const pixels = crop.context.getImageData(0, 0, width, height)
  const maskPixels = maskCrop.context.getImageData(0, 0, width, height)
  for (let i = 0; i < width * height; i++) {
    pixels.data[i * 4 + 3] = maskPixels.data[i * 4] ?? 0
  }
  crop.context.putImageData(pixels, 0, 0)
  const cutout = Texture.from(crop.canvas)

  const patch = buildPatch(photo, region, maskCrop.canvas, width, height)

  const grid = gridMesh(region, PREVIEW_GRID, PREVIEW_GRID)
  const mesh = new MeshSimple({
    texture: cutout,
    vertices: new Float32Array(grid.positions),
    uvs: grid.uvs,
    indices: grid.indices,
  })
  const container = new Container()
  container.addChild(patch, mesh)
  return { container, mesh, grid, maskUrl: spec.maskUrl }
}

/** A low-resolution, smoothly filled stand-in for the background behind the object. */
function buildPatch(
  photo: ImageBitmap,
  region: Rect,
  maskCanvas: HTMLCanvasElement,
  width: number,
  height: number,
): Sprite {
  const shrink = Math.min(1, FILL_RESOLUTION / Math.max(width, height))
  const w = Math.max(1, Math.round(width * shrink))
  const h = Math.max(1, Math.round(height * shrink))

  const small = makeCanvas(w, h)
  small.context.drawImage(photo, region.x, region.y, region.width, region.height, 0, 0, w, h)
  const rgba = small.context.getImageData(0, 0, w, h).data
  const rgb = new Uint8ClampedArray(w * h * 3)
  for (let i = 0; i < w * h; i++) {
    rgb[i * 3] = rgba[i * 4] ?? 0
    rgb[i * 3 + 1] = rgba[i * 4 + 1] ?? 0
    rgb[i * 3 + 2] = rgba[i * 4 + 2] ?? 0
  }

  const smallMask = makeCanvas(w, h)
  smallMask.context.drawImage(maskCanvas, 0, 0, w, h)
  const maskData = smallMask.context.getImageData(0, 0, w, h).data
  const masked = new Uint8Array(w * h)
  for (let i = 0; i < w * h; i++) masked[i] = (maskData[i * 4] ?? 0) > 127 ? 1 : 0
  const covered = dilate(masked, w, h, FILL_GROW_PX)
  const filled = fillMasked(rgb, covered, w, h)

  const out = makeCanvas(w, h)
  const image = out.context.createImageData(w, h)
  for (let i = 0; i < w * h; i++) {
    image.data[i * 4] = filled[i * 3] ?? 0
    image.data[i * 4 + 1] = filled[i * 3 + 1] ?? 0
    image.data[i * 4 + 2] = filled[i * 3 + 2] ?? 0
    image.data[i * 4 + 3] = covered[i] ? 255 : 0
  }
  out.context.putImageData(image, 0, 0)

  const sprite = new Sprite(Texture.from(out.canvas))
  sprite.position.set(region.x, region.y)
  sprite.width = region.width
  sprite.height = region.height
  return sprite
}

export const createPixiStage: CreateStage = async (container, callbacks) => {
  const app = new Application()
  await app.init({
    resizeTo: container,
    antialias: true,
    autoDensity: true,
    resolution: window.devicePixelRatio || 1,
    backgroundAlpha: 0,
    preference: 'webgl',
  })
  const canvas = app.canvas
  canvas.style.display = 'block'
  canvas.style.touchAction = 'none'
  container.appendChild(canvas)

  const world = new Container()
  const previewLayer = new Container()
  const overlay = new Graphics()
  world.addChild(previewLayer)
  world.addChild(overlay)
  app.stage.addChild(world)

  let sprite: Sprite | null = null
  let imageSize: Size = { width: 1, height: 1 }
  let view: Viewport = { scale: 1, x: 0, y: 0 }
  let autoFit = true
  let objects: readonly DetectedObject[] = []
  let pickables: Array<{ id: number; bbox: { width: number; height: number }; outline: Point[] }> =
    []
  let hovered: number | null = null
  let selected: number | null = null
  let imageBitmap: ImageBitmap | null = null
  const wanted = new Map<number, PreviewSpec>()
  const layers = new Map<number, PreviewLayer>()
  let frame = 0

  const containerSize = (): Size => ({
    width: container.clientWidth || 1,
    height: container.clientHeight || 1,
  })
  const fitView = () => fitViewport(imageSize, containerSize(), FIT_PADDING)
  const toPixels = (p: Point): Point => ({ x: p.x * imageSize.width, y: p.y * imageSize.height })

  function rebuildPickables() {
    pickables = objects.map((o) => ({ id: o.id, bbox: o.bbox, outline: o.outline.map(toPixels) }))
  }

  function drawOverlay() {
    const accent = readCssColor('--color-accent', 0xff8a1f)
    const onScreen = (pixels: number) => pixels / view.scale
    overlay.clear()

    for (const object of objects) {
      const isSelected = object.id === selected
      if (!isSelected && object.id !== hovered) continue
      const deform = wanted.get(object.id)?.deform
      const outline = object.outline.map((p) => {
        const pixel = toPixels(p)
        return deform ? deform(pixel) : pixel
      })
      overlay
        .poly(
          outline.flatMap((p) => [p.x, p.y]),
          true,
        )
        .fill({ color: accent, alpha: isSelected ? 0.18 : 0.1 })
        .stroke({ width: onScreen(isSelected ? 2.5 : 2), color: accent, alpha: 1, join: 'round' })
    }
  }

  function removeLayer(objectId: number) {
    const layer = layers.get(objectId)
    if (!layer) return
    previewLayer.removeChild(layer.container)
    layer.container.destroy({ children: true, texture: true })
    layers.delete(objectId)
  }

  function clearPreviews() {
    for (const id of [...layers.keys()]) removeLayer(id)
    wanted.clear()
  }

  function applyDeformation(layer: PreviewLayer, spec: PreviewSpec) {
    const { deform } = spec
    layer.container.visible = deform !== null && spec.showPixels
    if (deform) deformPositions(layer.grid.positions, deform, layer.mesh.vertices as Float32Array)
  }

  const building = new Set<number>()

  function apply() {
    world.scale.set(view.scale)
    world.position.set(view.x, view.y)
    drawOverlay()
    if (!frame) {
      frame = requestAnimationFrame(() => {
        frame = 0
        callbacks.onViewChange(view)
      })
    }
  }

  function setView(next: Viewport, manual: boolean) {
    view = next
    if (manual) autoFit = false
    apply()
  }

  function pointerPoint(event: { clientX: number; clientY: number }): Point {
    const rect = canvas.getBoundingClientRect()
    return { x: event.clientX - rect.left, y: event.clientY - rect.top }
  }

  function pick(event: { clientX: number; clientY: number }): number | null {
    return pickObject(pickables, screenToImage(view, pointerPoint(event)))
  }

  let drag: {
    startX: number
    startY: number
    lastX: number
    lastY: number
    moved: boolean
  } | null = null

  const onPointerDown = (event: PointerEvent) => {
    canvas.setPointerCapture(event.pointerId)
    drag = {
      startX: event.clientX,
      startY: event.clientY,
      lastX: event.clientX,
      lastY: event.clientY,
      moved: false,
    }
  }
  const onPointerMove = (event: PointerEvent) => {
    if (!drag) {
      const id = pick(event)
      canvas.style.cursor = id === null ? 'grab' : 'pointer'
      if (id !== hovered) callbacks.onHover(id)
      return
    }
    if (
      !drag.moved &&
      Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) < DRAG_THRESHOLD_PX
    )
      return
    drag.moved = true
    canvas.style.cursor = 'grabbing'
    setView(panBy(view, event.clientX - drag.lastX, event.clientY - drag.lastY), true)
    drag.lastX = event.clientX
    drag.lastY = event.clientY
  }
  const onPointerUp = (event: PointerEvent) => {
    if (drag && !drag.moved) callbacks.onSelect(pick(event))
    drag = null
    canvas.style.cursor = pick(event) === null ? 'grab' : 'pointer'
  }
  const onPointerLeave = () => callbacks.onHover(null)
  const onWheel = (event: WheelEvent) => {
    event.preventDefault()
    if (event.ctrlKey || event.metaKey) {
      const factor = Math.exp(-event.deltaY * WHEEL_ZOOM_SPEED)
      setView(zoomAt(view, factor, pointerPoint(event), fitView().scale), true)
    } else {
      setView(panBy(view, -event.deltaX, -event.deltaY), true)
    }
  }

  canvas.addEventListener('pointerdown', onPointerDown)
  canvas.addEventListener('pointermove', onPointerMove)
  canvas.addEventListener('pointerup', onPointerUp)
  canvas.addEventListener('pointerleave', onPointerLeave)
  canvas.addEventListener('wheel', onWheel, { passive: false })
  canvas.style.cursor = 'grab'

  const resizeObserver = new ResizeObserver(() => {
    if (autoFit) setView(fitView(), false)
    else apply()
  })
  resizeObserver.observe(container)

  return {
    async setImage(url) {
      const response = await fetch(url)
      const bitmap = await createImageBitmap(await response.blob())
      clearPreviews()
      imageBitmap?.close()
      imageBitmap = bitmap
      const texture = Texture.from(bitmap)
      texture.source.autoGenerateMipmaps = true
      sprite?.destroy({ texture: true })
      sprite = new Sprite(texture)
      world.addChildAt(sprite, 0)
      imageSize = { width: bitmap.width, height: bitmap.height }
      rebuildPickables()
      autoFit = true
      setView(fitView(), false)
      return imageSize
    },
    setObjects(next) {
      objects = next
      rebuildPickables()
      drawOverlay()
    },
    setHighlight(nextHovered, nextSelected) {
      hovered = nextHovered
      selected = nextSelected
      drawOverlay()
    },
    setPreview(objectId, spec) {
      if (!spec) {
        wanted.delete(objectId)
        removeLayer(objectId)
        drawOverlay()
        return
      }
      wanted.set(objectId, spec)
      const existing = layers.get(objectId)
      if (existing && existing.maskUrl === spec.maskUrl) {
        applyDeformation(existing, spec)
        drawOverlay()
        return
      }
      drawOverlay()
      removeLayer(objectId)
      if (building.has(objectId) || !imageBitmap) return
      building.add(objectId)
      buildPreviewLayer(imageBitmap, imageSize, spec)
        .then((layer) => {
          building.delete(objectId)
          const latest = wanted.get(objectId)
          if (!latest || latest.maskUrl !== layer.maskUrl) {
            layer.container.destroy({ children: true, texture: true })
            return
          }
          layers.set(objectId, layer)
          previewLayer.addChild(layer.container)
          applyDeformation(layer, latest)
          drawOverlay()
        })
        .catch(() => building.delete(objectId))
    },
    async captureGuide(maxSide, shapes) {
      if (!imageBitmap) return null
      const scale = Math.min(1, maxSide / Math.max(imageSize.width, imageSize.height))
      const width = Math.max(1, Math.round(imageSize.width * scale))
      const height = Math.max(1, Math.round(imageSize.height * scale))
      const guide = document.createElement('canvas')
      guide.width = width
      guide.height = height
      const context = guide.getContext('2d')
      if (!context) return null
      context.drawImage(imageBitmap, 0, 0, width, height)
      drawGuide(context, shapes, width, height)
      return canvasToBlob(guide)
    },
    async capturePreview(maxSide) {
      if (!sprite) return null
      const scale = Math.min(1, maxSide / Math.max(imageSize.width, imageSize.height))
      const target = RenderTexture.create({
        width: Math.max(1, Math.round(imageSize.width * scale)),
        height: Math.max(1, Math.round(imageSize.height * scale)),
        resolution: 1,
      })
      const saved = { scale: world.scale.x, x: world.position.x, y: world.position.y }
      world.scale.set(scale)
      world.position.set(0, 0)
      overlay.visible = false
      for (const [id, layer] of layers) layer.container.visible = wanted.get(id)?.deform != null
      try {
        app.renderer.render({ container: world, target })
        const canvas = app.renderer.extract.canvas({ target })
        return await canvasToBlob(canvas as HTMLCanvasElement | OffscreenCanvas)
      } finally {
        world.scale.set(saved.scale)
        world.position.set(saved.x, saved.y)
        overlay.visible = true
        for (const [id, layer] of layers) {
          const spec = wanted.get(id)
          layer.container.visible = spec?.deform != null && spec.showPixels
        }
        target.destroy(true)
      }
    },
    fit() {
      autoFit = true
      setView(fitView(), false)
    },
    zoomBy(factor) {
      const centre = { x: containerSize().width / 2, y: containerSize().height / 2 }
      setView(zoomAt(view, factor === 0 ? 1 : factor, centre, fitView().scale), true)
    },
    destroy() {
      cancelAnimationFrame(frame)
      clearPreviews()
      imageBitmap?.close()
      resizeObserver.disconnect()
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointerleave', onPointerLeave)
      canvas.removeEventListener('wheel', onWheel)
      app.destroy({ removeView: true }, { children: true, texture: true })
    },
  }
}

export { BUTTON_ZOOM_FACTOR, imageToScreen }
