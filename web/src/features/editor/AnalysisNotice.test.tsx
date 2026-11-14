import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AnalysisNotice } from './AnalysisNotice'

const handlers = () => ({ onRetry: vi.fn(), onNewImage: vi.fn() })

describe('AnalysisNotice', () => {
  it('offers a retry when analysis failed', async () => {
    const h = handlers()
    render(<AnalysisNotice analysis="failed" objectCount={0} {...h} />)
    expect(screen.getByRole('alert')).toHaveTextContent('could not finish')
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }))
    expect(h.onRetry).toHaveBeenCalledOnce()
  })

  it('explains an empty result and offers another photo', async () => {
    const h = handlers()
    render(<AnalysisNotice analysis="done" objectCount={0} {...h} />)
    expect(screen.getByRole('status')).toHaveTextContent('Nothing editable')
    await userEvent.click(screen.getByRole('button', { name: 'Choose another photo' }))
    expect(h.onNewImage).toHaveBeenCalledOnce()
  })

  it('is silent while running and when objects were found', () => {
    const h = handlers()
    const { rerender, container } = render(
      <AnalysisNotice analysis="running" objectCount={0} {...h} />,
    )
    expect(container).toBeEmptyDOMElement()
    rerender(<AnalysisNotice analysis="done" objectCount={2} {...h} />)
    expect(container).toBeEmptyDOMElement()
  })
})
