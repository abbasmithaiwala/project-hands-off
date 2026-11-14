import { Application, Container, Graphics, Sprite, Texture } from 'pixi.js'
import type { DetectedObject, Widget } from '@/api/client'
import { readCssColor } from '@/lib/color'
import { pickObject, type Point } from '@/lib/geometry'
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
  setWidget: (widget: Widget | null) => void
  fit: () => void
  zoomBy: (factor: number) => void
  destroy: () => void
}

export type CreateStage = (
  container: HTMLElement,
  callbacks: StageCallbacks,
) => Promise<StageHandle>

const FIT_PADDING = 64
const DRAG_THRESHOLD_PX = 4
const WHEEL_ZOOM_SPEED = 0.01
const BUTTON_ZOOM_FACTOR = 1.25

export const createPixiStage: CreateStage = async (container, callbacks) => {
  const app = new Application()
  await app.init({
    resizeTo: container,
    antialias: true,
    autoDensity: true,
    resolution: window.devicePixelRatio || 1,
    backgroundAlpha: 0,
  })
  const canvas = app.canvas
  canvas.style.display = 'block'
  canvas.style.touchAction = 'none'
  container.appendChild(canvas)

  const world = new Container()
  const overlay = new Graphics()
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
  let widget: Widget | null = null
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
      overlay
        .poly(
          object.outline.flatMap((p) => [p.x * imageSize.width, p.y * imageSize.height]),
          true,
        )
        .fill({ color: accent, alpha: isSelected ? 0.18 : 0.1 })
        .stroke({ width: onScreen(isSelected ? 2.5 : 2), color: accent, alpha: 1, join: 'round' })
    }

    if (widget) drawWidget(widget, accent, onScreen)
  }

  function drawWidget(w: Widget, accent: number, onScreen: (px: number) => number) {
    const dot = (p: Point, filled: boolean) =>
      overlay
        .circle(p.x, p.y, onScreen(7))
        .fill({ color: filled ? 0xffffff : 0x000000, alpha: filled ? 1 : 0.35 })
        .stroke({ width: onScreen(2.5), color: accent })
    if (w.kind === 'slider') {
      const start = toPixels(w.axis.start)
      const end = toPixels(w.axis.end)
      overlay
        .moveTo(start.x, start.y)
        .lineTo(end.x, end.y)
        .stroke({ width: onScreen(3), color: accent })
      dot(start, true)
      dot(end, false)
    } else if (w.kind === 'control_points') {
      const points = w.points.map(toPixels)
      overlay
        .poly(
          points.flatMap((p) => [p.x, p.y]),
          true,
        )
        .stroke({ width: onScreen(1.5), color: 0xffffff, alpha: 0.9 })
      points.forEach((p) => dot(p, true))
    } else {
      w.handles.map(toPixels).forEach((p) => dot(p, true))
    }
  }

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
    setWidget(next) {
      widget = next
      drawOverlay()
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
