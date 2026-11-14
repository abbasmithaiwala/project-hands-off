import { FRAGMENT_SHADER, VERTEX_SHADER } from './blobShader'

/** The canvas is rendered at no more than this many device pixels per CSS pixel. */
const MAX_PIXEL_RATIO = 1.5
/** Seconds of animation time shown when motion is turned off. */
const STILL_TIME = 14

export interface BlobFieldOptions {
  /** Draw one frame and stop, for people who prefer reduced motion. */
  still: boolean
  /** Whether to paint on the dark ground. May change while running. */
  isDark: () => boolean
}

function compile(gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null {
  const shader = gl.createShader(type)
  if (!shader) return null
  gl.shaderSource(shader, source)
  gl.compileShader(shader)
  return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null
}

function link(gl: WebGLRenderingContext): WebGLProgram | null {
  const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER)
  const fragment = compile(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER)
  const program = gl.createProgram()
  if (!vertex || !fragment || !program) return null
  gl.attachShader(program, vertex)
  gl.attachShader(program, fragment)
  gl.linkProgram(program)
  return gl.getProgramParameter(program, gl.LINK_STATUS) ? program : null
}

/**
 * Paint drifting, grainy blobs onto `canvas` with WebGL.
 *
 * Returns a function that stops the animation and frees the GL resources, or `null` when WebGL
 * is unavailable (the caller keeps its CSS fallback).
 */
export function startBlobField(
  canvas: HTMLCanvasElement,
  { still, isDark }: BlobFieldOptions,
): (() => void) | null {
  const gl = canvas.getContext('webgl', { antialias: false, alpha: false })
  const program = gl ? link(gl) : null
  if (!gl || !program) return null

  gl.useProgram(program)
  const buffer = gl.createBuffer()
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer)
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW)
  const position = gl.getAttribLocation(program, 'position')
  gl.enableVertexAttribArray(position)
  gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0)

  const resolution = gl.getUniformLocation(program, 'uResolution')
  const time = gl.getUniformLocation(program, 'uTime')
  const dark = gl.getUniformLocation(program, 'uDark')

  const resize = () => {
    const ratio = Math.min(window.devicePixelRatio || 1, MAX_PIXEL_RATIO)
    const width = Math.max(1, Math.round(canvas.clientWidth * ratio))
    const height = Math.max(1, Math.round(canvas.clientHeight * ratio))
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width
      canvas.height = height
    }
    gl.viewport(0, 0, width, height)
    gl.uniform2f(resolution, width, height)
  }

  const draw = (seconds: number) => {
    resize()
    gl.uniform1f(time, seconds)
    gl.uniform1f(dark, isDark() ? 1 : 0)
    gl.drawArrays(gl.TRIANGLES, 0, 3)
  }

  let frame = 0
  const started = performance.now()
  const loop = () => {
    draw((performance.now() - started) / 1000 + STILL_TIME)
    frame = requestAnimationFrame(loop)
  }
  const onVisibility = () => {
    cancelAnimationFrame(frame)
    if (!document.hidden) frame = requestAnimationFrame(loop)
  }
  const onResize = () => draw(STILL_TIME)

  if (still) {
    draw(STILL_TIME)
    window.addEventListener('resize', onResize)
  } else {
    frame = requestAnimationFrame(loop)
    document.addEventListener('visibilitychange', onVisibility)
  }

  return () => {
    cancelAnimationFrame(frame)
    document.removeEventListener('visibilitychange', onVisibility)
    window.removeEventListener('resize', onResize)
    gl.deleteProgram(program)
    gl.deleteBuffer(buffer)
  }
}
