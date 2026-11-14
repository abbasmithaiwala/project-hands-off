import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Logo } from './Logo'

describe('Logo', () => {
  it('is decorative, so the name beside it is what screen readers hear', () => {
    const { container } = render(<Logo />)
    const svg = container.querySelector('svg')
    expect(svg).toHaveAttribute('aria-hidden', 'true')
    expect(svg).toHaveAttribute('width', '32')
  })

  it('can be any size', () => {
    const { container } = render(<Logo size={56} />)
    expect(container.querySelector('svg')).toHaveAttribute('height', '56')
  })

  it('gives each logo its own gradient id so several can share a page', () => {
    const { container } = render(
      <>
        <Logo />
        <Logo />
      </>,
    )
    const ids = [...container.querySelectorAll('linearGradient')].map((g) => g.id)
    expect(new Set(ids).size).toBe(2)
  })
})
