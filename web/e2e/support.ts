import { readFileSync } from 'node:fs'
import type { Page, Route } from '@playwright/test'

const photo = readFileSync(new URL('../../eval/golden/couch.jpg', import.meta.url))
const couch = JSON.parse(
  readFileSync(new URL('./fixtures/couch-object.json', import.meta.url), 'utf8'),
)

const IMAGE = {
  id: '11111111-1111-1111-1111-111111111111',
  hash: 'a'.repeat(64),
  format: 'jpeg',
  width: 1255,
  height: 706,
  size_bytes: photo.length,
  created_at: '2026-01-01T00:00:00Z',
}

const MODELS = {
  suggester: 'google/gemini-3.1-flash-lite',
  ui_planner: 'google/gemini-3.1-flash-lite',
  summariser: 'google/gemini-3.1-flash-lite',
  generator: 'google/gemini-3-pro-image',
  fallback: 'anthropic/claude-haiku-5.5',
}

const PROPERTIES = {
  properties: [
    { key: 'couch_length', label: 'Couch length', widget: 'slider' },
    { key: 'armrest_shape', label: 'Armrest contour', widget: 'control_points' },
    { key: 'cushions', label: 'Cushion placement', widget: 'multi_handle' },
  ],
}

const SLIDER = {
  kind: 'slider',
  axis: { start: { x: 0.22, y: 0.7 }, end: { x: 0.66, y: 0.7 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

const json = (route: Route, body: unknown, status = 200) =>
  route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) })

const sse = (events: Array<{ type: string } & Record<string, unknown>>) =>
  events.map((e) => `event: ${e.type}\ndata: ${JSON.stringify(e)}\n\n`).join('')

export interface ApiMockOptions {
  /** Settings returned by `GET /settings`. Defaults to a configured key. */
  configured?: boolean
  /** When set, the properties route fails with this error until a key is saved. */
  needsKey?: boolean
}

/** Answers every API route with deterministic fixtures, so the UI runs without a backend. */
export async function mockApi(page: Page, options: ApiMockOptions = {}) {
  let keySaved = !options.needsKey
  const base = '**/api/v1'

  await page.route(`${base}/images`, (route) => json(route, IMAGE, 201))
  await page.route(`${base}/images/*/original`, (route) =>
    route.fulfill({ status: 200, contentType: 'image/jpeg', body: photo }),
  )
  await page.route(`${base}/images/*/analyze`, (route) => json(route, { job_id: 'job-1' }, 202))
  await page.route(`${base}/jobs/job-1/events`, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: sse([
        { type: 'progress', stage: 'detecting', message: 'Finding objects' },
        { type: 'object_found', object: couch },
        { type: 'log', line: 'Found 1 objects' },
        { type: 'completed', result: { objects: 1 } },
      ]),
    }),
  )
  await page.route(`${base}/images/*/objects/0/properties`, (route) =>
    keySaved
      ? json(route, PROPERTIES)
      : json(
          route,
          {
            code: 'unavailable',
            message: 'An OpenRouter API key is required. Add one in Settings.',
          },
          503,
        ),
  )
  await page.route(`${base}/images/*/objects/0/widget`, (route) => json(route, SLIDER))
  await page.route(`${base}/settings`, (route) =>
    json(route, {
      openrouter: {
        configured: options.configured ?? keySaved,
        origin: keySaved ? 'stored' : 'none',
      },
      models: MODELS,
    }),
  )
  await page.route(`${base}/settings/openrouter-key`, (route) => {
    keySaved = true
    return route.fulfill({ status: 204 })
  })
}

export const photoPath = new URL('../../eval/golden/couch.jpg', import.meta.url).pathname

/** Upload the couch photo and wait for analysis to finish. */
export async function openEditor(page: Page) {
  await page.goto('/')
  await page.getByTestId('file-input').setInputFiles(photoPath)
  await page.getByText('1 object found').waitFor()
  await page.getByTestId('canvas-stage').locator('canvas').waitFor()
}

/** Click where the couch seat is, in the canvas. */
export async function clickCouch(page: Page) {
  const box = await page.getByTestId('canvas-stage').boundingBox()
  if (!box) throw new Error('canvas not visible')
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.62)
}
