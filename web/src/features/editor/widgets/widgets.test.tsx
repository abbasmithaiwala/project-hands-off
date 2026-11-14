import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { EditChange, Widget } from '@/api/client'
import { WidgetLayer } from './WidgetLayer'

const imageSize = { width: 1000, height: 500 }
const view = { scale: 1, x: 0, y: 0 }

const slider: Widget = {
  kind: 'slider',
  axis: { start: { x: 0.2, y: 0.5 }, end: { x: 0.6, y: 0.5 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

const outline: Widget = {
  kind: 'control_points',
  points: [
    { x: 0.2, y: 0.2 },
    { x: 0.6, y: 0.2 },
    { x: 0.6, y: 0.6 },
    { x: 0.2, y: 0.6 },
  ],
}

const joints: Widget = {
  kind: 'multi_handle',
  handles: [
    { x: 0.3, y: 0.3 },
    { x: 0.7, y: 0.7 },
  ],
}

function layer(
  widget: Widget,
  change: EditChange | null,
  onChange: (change: EditChange | null) => void,
  overrides: {
    propertyLabel?: string
    objectLabel?: string
    view?: typeof view
    reach?: number
  } = {},
) {
  return (
    <WidgetLayer
      widget={widget}
      change={change}
      propertyLabel={overrides.propertyLabel ?? 'Shape'}
      objectLabel={overrides.objectLabel ?? 'couch'}
      view={overrides.view ?? view}
      imageSize={imageSize}
      reach={overrides.reach ?? 200}
      onChange={onChange}
    />
  )
}

function setup(widget: Widget, change: EditChange | null = null) {
  const onChange = vi.fn<(change: EditChange | null) => void>()
  const view = render(layer(widget, change, onChange))
  return { onChange, ...view }
}

const lastChange = (onChange: { mock: { calls: unknown[][] } }) =>
  onChange.mock.calls.at(-1)?.[0] as EditChange | null

function expectPoints(change: EditChange | null, kind: 'control_points' | 'multi_handle') {
  if (change?.kind !== kind)
    throw new Error(`expected a ${kind} change, got ${JSON.stringify(change)}`)
  return change
}

function drag(
  handle: HTMLElement,
  to: { x: number; y: number },
  init: { shiftKey?: boolean } = {},
) {
  fireEvent.pointerDown(handle, { clientX: 0, clientY: 0, pointerId: 1 })
  fireEvent.pointerMove(handle, { clientX: to.x, clientY: to.y, pointerId: 1, ...init })
  fireEvent.pointerUp(handle, { pointerId: 1 })
}

const handlesOf = () => screen.getAllByRole('button')

describe('SliderWidget', () => {
  it('is a labelled slider showing its value to assistive technology', () => {
    setup(slider)
    const handle = screen.getByRole('slider', { name: 'Shape of couch' })
    expect(handle).toHaveAttribute('aria-valuenow', '100')
    expect(handle).toHaveAttribute('aria-valuemin', '50')
    expect(handle).toHaveAttribute('aria-valuemax', '150')
    expect(handle).toHaveAttribute('aria-valuetext', '100 percent')
    expect(screen.getByRole('group', { name: 'Shape controls for couch' })).toBeInTheDocument()
  })

  it('rests at the end of its axis', () => {
    setup(slider)
    expect(screen.getByRole('slider')).toHaveStyle({ left: '600px', top: '250px' })
  })

  it('reports the value for where it is dragged', () => {
    const { onChange } = setup(slider)
    drag(screen.getByRole('slider'), { x: 400, y: 250 })
    // The axis runs 200..600, so 400 is halfway along it: half of the initial value.
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: 0.5 })
  })

  it('cannot be dragged outside its allowed range', () => {
    const { onChange } = setup(slider)
    drag(screen.getByRole('slider'), { x: 990, y: 250 })
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: 1.5 })
    drag(screen.getByRole('slider'), { x: 0, y: 250 })
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: 0.5 })
  })

  it('nudges with the arrow keys, finer with Shift, and resets with Home', async () => {
    const { onChange } = setup(slider, { kind: 'slider', value: 0.8 })
    screen.getByRole('slider').focus()

    await userEvent.keyboard('{ArrowRight}')
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: expect.closeTo(0.85, 5) })
    await userEvent.keyboard('{Shift>}{ArrowLeft}{/Shift}')
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: expect.closeTo(0.79, 5) })
    await userEvent.keyboard('{ArrowDown}')
    expect(lastChange(onChange)).toEqual({ kind: 'slider', value: expect.closeTo(0.75, 5) })
    await userEvent.keyboard('{Home}')
    expect(lastChange(onChange)).toBeNull()
  })

  it('shows the current value next to the handle once it has been changed', () => {
    setup(slider, { kind: 'slider', value: 0.64 })
    expect(screen.getByText('64%')).toBeInTheDocument()
  })

  it('does not show a badge before any change', () => {
    setup(slider)
    expect(screen.queryByText(/%$/)).not.toBeInTheDocument()
  })
})

describe('ControlPointsWidget', () => {
  it('shows one labelled handle per point', () => {
    setup(outline)
    expect(handlesOf()).toHaveLength(4)
    expect(handlesOf()[1]).toHaveAccessibleName('Shape point 2 of 4 on couch')
    expect(handlesOf()[1]).toHaveAttribute('aria-valuetext', '60 percent across, 20 percent down')
  })

  it('reports the original and new positions when a point is dragged', () => {
    const { onChange } = setup(outline)
    drag(handlesOf()[0] as HTMLElement, { x: 300, y: 150 })
    const change = expectPoints(lastChange(onChange), 'control_points')
    expect(change.origin).toEqual(outline.kind === 'control_points' ? outline.points : [])
    expect(change.target[0]).toEqual({ x: 0.3, y: 0.3 })
    expect(change.target.slice(1)).toEqual(change.origin.slice(1))
  })

  it('limits how far a point can be dragged from its origin', () => {
    const { onChange } = setup(outline)
    drag(handlesOf()[0] as HTMLElement, { x: 900, y: 450 })
    const moved = expectPoints(lastChange(onChange), 'control_points').target[0] as {
      x: number
      y: number
    }
    const distance = Math.hypot(
      (moved.x - 0.2) * imageSize.width,
      (moved.y - 0.2) * imageSize.height,
    )
    expect(distance).toBeCloseTo(200, 0)
  })

  it('snaps back to the origin when dropped almost where it started', () => {
    const { onChange } = setup(outline)
    drag(handlesOf()[0] as HTMLElement, { x: 203, y: 100 })
    expect(expectPoints(lastChange(onChange), 'control_points').target[0]).toEqual({
      x: 0.2,
      y: 0.2,
    })
  })

  it('locks movement to one axis while Shift is held', () => {
    const { onChange } = setup(outline)
    drag(handlesOf()[0] as HTMLElement, { x: 320, y: 120 }, { shiftKey: true })
    expect(expectPoints(lastChange(onChange), 'control_points').target[0]).toEqual({
      x: 0.32,
      y: 0.2,
    })
  })

  it('nudges one point with the arrow keys and resets just that point with Home', async () => {
    const moved: EditChange = {
      kind: 'control_points',
      origin: outline.kind === 'control_points' ? outline.points : [],
      target: [
        { x: 0.3, y: 0.2 },
        { x: 0.6, y: 0.2 },
        { x: 0.6, y: 0.6 },
        { x: 0.2, y: 0.6 },
      ],
    }
    const { onChange } = setup(outline, moved)
    handlesOf()[0]?.focus()

    await userEvent.keyboard('{ArrowRight}')
    expect(
      (expectPoints(lastChange(onChange), 'control_points').target[0] as { x: number }).x,
    ).toBeCloseTo(0.31, 5)

    await userEvent.keyboard('{Shift>}{ArrowDown}{/Shift}')
    expect(
      (expectPoints(lastChange(onChange), 'control_points').target[0] as { y: number }).y,
    ).toBeCloseTo(0.201, 5)

    await userEvent.keyboard('{Home}')
    expect(expectPoints(lastChange(onChange), 'control_points').target[0]).toEqual({
      x: 0.2,
      y: 0.2,
    })
  })

  it('draws an outline through the points', () => {
    const { container } = setup(outline)
    expect(container.querySelector('polygon')).not.toBeNull()
  })
})

describe('MultiHandleWidget', () => {
  it('shows free-standing handles without an outline', () => {
    const onChange = vi.fn()
    const { container } = render(
      layer(joints, null, onChange, { propertyLabel: 'Pose', objectLabel: 'person' }),
    )
    expect(handlesOf()).toHaveLength(2)
    expect(screen.getByRole('button', { name: 'Pose handle 1 of 2 on person' })).toBeInTheDocument()
    expect(container.querySelector('polygon')).toBeNull()
  })

  it('reports independent handle moves as a multi_handle change', () => {
    const { onChange } = setup(joints)
    drag(handlesOf()[1] as HTMLElement, { x: 650, y: 330 })
    const change = expectPoints(lastChange(onChange), 'multi_handle')
    expect(change.origin).toEqual([
      { x: 0.3, y: 0.3 },
      { x: 0.7, y: 0.7 },
    ])
    expect(change.target[0]).toEqual({ x: 0.3, y: 0.3 })
    expect(change.target[1]).toEqual({ x: 0.65, y: 0.66 })
  })

  it('stays inside the image', () => {
    const { onChange } = setup(joints)
    drag(handlesOf()[0] as HTMLElement, { x: -400, y: -400 })
    const point = expectPoints(lastChange(onChange), 'multi_handle').target[0] as {
      x: number
      y: number
    }
    expect(point.x).toBeGreaterThanOrEqual(0)
    expect(point.y).toBeGreaterThanOrEqual(0)
  })
})

describe('WidgetLayer', () => {
  it('maps pointer positions through the current zoom and pan', () => {
    const onChange = vi.fn()
    render(layer(joints, null, onChange, { view: { scale: 2, x: 100, y: 50 }, reach: 400 }))
    // Screen (700, 350) is image pixel ((700 - 100) / 2, (350 - 50) / 2) = (300, 150) = (0.3, 0.3).
    drag(handlesOf()[0] as HTMLElement, { x: 700, y: 350 })
    expect(expectPoints(lastChange(onChange), 'multi_handle').target[0]).toEqual({ x: 0.3, y: 0.3 })
  })

  it('ignores pointer moves when no drag is in progress', () => {
    const { onChange } = setup(joints)
    fireEvent.pointerMove(handlesOf()[0] as HTMLElement, { clientX: 500, clientY: 200 })
    expect(onChange).not.toHaveBeenCalled()
  })

  it('ignores a stale change of a different kind', () => {
    const stale: EditChange = { kind: 'slider', value: 0.7 }
    setup(joints, stale)
    expect(handlesOf()[0]).toHaveStyle({ left: '300px', top: '150px' })
  })
})
