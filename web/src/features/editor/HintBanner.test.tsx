import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it } from 'vitest'
import { HintBanner } from './HintBanner'
import { readDoneHints, type HintContext } from './hints'

const idle: HintContext = {
  hasObjects: true,
  hasSelection: false,
  hasActiveWidget: false,
  hasPendingEdits: false,
  widgetKind: null,
}

beforeEach(() => window.localStorage.clear())

describe('HintBanner', () => {
  it('shows the tip that fits and remembers once it is dismissed', async () => {
    const { rerender } = render(<HintBanner context={idle} />)
    expect(screen.getByRole('status')).toHaveTextContent('click it')
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss tip' }))
    expect(screen.queryByRole('status')).toBeNull()
    rerender(<HintBanner context={{ ...idle }} />)
    expect(screen.queryByRole('status')).toBeNull()
    expect(readDoneHints().has('pick-object')).toBe(true)
  })

  it('retires a tip when the user moves on without dismissing it', () => {
    const { rerender } = render(<HintBanner context={idle} />)
    rerender(<HintBanner context={{ ...idle, hasSelection: true, hasActiveWidget: true }} />)
    expect(readDoneHints().has('pick-object')).toBe(true)
    expect(screen.getByRole('status')).toHaveTextContent('Drag the handle')
  })

  it('words the drag tip for the control on screen', () => {
    render(
      <HintBanner
        context={{ ...idle, hasSelection: true, hasActiveWidget: true, widgetKind: 'rotate' }}
      />,
    )
    expect(screen.getByRole('status')).toHaveTextContent(
      'Drag the handle around the ring to turn it.',
    )
  })

  it('does not show tips that were already finished', () => {
    window.localStorage.setItem('hands-off.hints-done', '["pick-object"]')
    render(<HintBanner context={idle} />)
    expect(screen.queryByRole('status')).toBeNull()
  })
})
