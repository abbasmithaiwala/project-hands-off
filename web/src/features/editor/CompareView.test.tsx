import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { CompareView } from './CompareView'

function setup(view = { scale: 0.5, x: 20, y: 10 }) {
  render(
    <CompareView
      beforeUrl="/before.jpg"
      afterUrl="/after.jpg"
      view={view}
      imageSize={{ width: 800, height: 400 }}
    />,
  )
  return {
    frame: screen.getByTestId('compare-view'),
    grip: screen.getByRole('slider', { name: 'Compare before and after' }),
    before: screen.getByAltText('Before: the original photo'),
  }
}

describe('CompareView', () => {
  it('covers exactly the image as it is currently placed', () => {
    const { frame } = setup()
    expect(frame).toHaveStyle({ left: '20px', top: '10px', width: '400px', height: '200px' })
  })

  it('shows the original on the left of the divider and the edit on the right', () => {
    const { before } = setup()
    expect(before).toHaveStyle({ clipPath: 'inset(0 50% 0 0)' })
    expect(screen.getByAltText('After: the edited photo')).toHaveAttribute('src', '/after.jpg')
  })

  it('exposes the divider as an accessible slider', () => {
    const { grip } = setup()
    expect(grip).toHaveAttribute('aria-valuenow', '50')
    expect(grip).toHaveAttribute('aria-valuemin', '0')
    expect(grip).toHaveAttribute('aria-valuemax', '100')
  })

  it('moves with the arrow keys, finer with Shift, and jumps with Home and End', async () => {
    const { grip, before } = setup()
    grip.focus()
    await userEvent.keyboard('{ArrowRight}')
    expect(grip).toHaveAttribute('aria-valuenow', '55')
    await userEvent.keyboard('{Shift>}{ArrowLeft}{/Shift}')
    expect(grip).toHaveAttribute('aria-valuenow', '54')
    await userEvent.keyboard('{End}')
    expect(grip).toHaveAttribute('aria-valuenow', '100')
    expect(before).toHaveStyle({ clipPath: 'inset(0 0% 0 0)' })
    await userEvent.keyboard('{Home}')
    expect(grip).toHaveAttribute('aria-valuenow', '0')
    await userEvent.keyboard('{ArrowLeft}')
    expect(grip).toHaveAttribute('aria-valuenow', '0')
  })

  it('follows the pointer while dragging and stops afterwards', () => {
    const { frame, grip } = setup()
    frame.getBoundingClientRect = () => ({
      left: 100,
      width: 400,
      top: 0,
      height: 200,
      right: 500,
      bottom: 200,
      x: 100,
      y: 0,
      toJSON: () => ({}),
    })
    fireEvent.pointerDown(frame, { clientX: 200, pointerId: 1 })
    expect(grip).toHaveAttribute('aria-valuenow', '25')
    fireEvent.pointerMove(frame, { clientX: 400, pointerId: 1 })
    expect(grip).toHaveAttribute('aria-valuenow', '75')
    fireEvent.pointerUp(frame, { pointerId: 1 })
    fireEvent.pointerMove(frame, { clientX: 120, pointerId: 1 })
    expect(grip).toHaveAttribute('aria-valuenow', '75')
  })

  it('keeps the divider inside the image', () => {
    const { frame, grip } = setup()
    frame.getBoundingClientRect = () => ({
      left: 0,
      width: 400,
      top: 0,
      height: 200,
      right: 400,
      bottom: 200,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    })
    fireEvent.pointerDown(frame, { clientX: 9999, pointerId: 1 })
    expect(grip).toHaveAttribute('aria-valuenow', '100')
    fireEvent.pointerMove(frame, { clientX: -500, pointerId: 1 })
    expect(grip).toHaveAttribute('aria-valuenow', '0')
  })
})
