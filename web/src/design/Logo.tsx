import { useId } from 'react'

interface LogoProps {
  /** Width and height in pixels. */
  size?: number
}

/**
 * The Hands-Off mark: a shape with a dot on each corner, one of them grabbed.
 * Decorative: put the name next to it. Keep in step with `public/favicon.svg`.
 */
export function Logo({ size = 32 }: LogoProps) {
  const gradient = useId()
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id={gradient} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ffab57" />
          <stop offset="1" stopColor="#e8710a" />
        </linearGradient>
      </defs>
      <rect
        x="1"
        y="1"
        width="62"
        height="62"
        rx="15"
        fill={`url(#${gradient})`}
        stroke="rgba(255,255,255,0.7)"
        strokeWidth="2"
      />
      <path
        d="M17 41 L23 20 L43 15 L51 35 L35 48 Z"
        fill="none"
        stroke="#fff"
        strokeWidth="3.5"
        strokeLinejoin="round"
      />
      <circle cx="17" cy="41" r="4" fill="#fff" />
      <circle cx="23" cy="20" r="4" fill="#fff" />
      <circle cx="43" cy="15" r="4" fill="#fff" />
      <circle cx="35" cy="48" r="4" fill="#fff" />
      <circle cx="51" cy="35" r="9" fill="rgba(255,255,255,0.28)" stroke="#fff" strokeWidth="3" />
      <circle cx="51" cy="35" r="4.5" fill="#fff" />
    </svg>
  )
}
