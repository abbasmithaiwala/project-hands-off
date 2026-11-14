import { readFileSync } from 'node:fs'
import type { Locator, Page, Route } from '@playwright/test'

const photo = readFileSync(new URL('../../eval/golden/couch.jpg', import.meta.url))
const fixture = (name: string) => readFileSync(new URL(`./fixtures/${name}`, import.meta.url))
const couch = JSON.parse(fixture('couch-object.json').toString('utf8'))
const mask = fixture('couch-mask.png')
const afterPhoto = fixture('couch-after.jpg')

const SLIDER = {
  kind: 'slider',
  axis: { start: { x: 0.215, y: 0.72 }, end: { x: 0.69, y: 0.72 } },
  min: 0.5,
  max: 1.5,
  initial: 1,
}

const WIDGETS: Record<string, unknown> = {
  couch_length: SLIDER,
  armrest_shape: JSON.parse(fixture('widget-control-points.json').toString('utf8')),
  cushions: JSON.parse(fixture('widget-multi-handle.json').toString('utf8')),
}

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
  draft_generator: 'google/gemini-3.1-flash-image',
  fallback: 'anthropic/claude-haiku-5.5',
}

const PROPERTIES = {
  properties: [
    { key: 'couch_length', label: 'Couch length', widget: 'slider' },
    { key: 'armrest_shape', label: 'Armrest contour', widget: 'control_points' },
    { key: 'cushions', label: 'Cushion placement', widget: 'multi_handle' },
  ],
}

const json = (route: Route, body: unknown, status = 200) =>
  route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) })

const sse = (events: Array<{ type: string } & Record<string, unknown>>) =>
  events.map((e) => `event: ${e.type}\ndata: ${JSON.stringify(e)}\n\n`).join('')

const EDIT_RESULT = {
  summary: 'Shortened the couch',
  changes: [{ edit_index: 0, text: 'Shortened the couch to about 60% of its length' }],
  instruction: 'Make the couch about 40% shorter, keeping its left end fixed.',
  model: 'google/gemini-3-pro-image',
  cost_usd: 0.14,
  generation_ms: 19200,
}

const ACCEPTED_IMAGE = { ...IMAGE, id: '22222222-2222-2222-2222-222222222222' }

/** How the mocked edit job behaves. */
export type EditOutcome = 'complete' | 'hang' | 'fail' | 'fail-key'

/** What the page sent to the mocked API, for assertions. */
export interface Captured {
  edits: Array<Record<string, unknown>>
  cancels: number
  accepts: number
}

export interface ApiMockOptions {
  /** How the edit job ends. Defaults to completing. */
  edit?: EditOutcome
  /** Settings returned by `GET /settings`. Defaults to a configured key. */
  configured?: boolean
  /** When set, the properties route fails with this error until a key is saved. */
  needsKey?: boolean
}

/** Answers every API route with deterministic fixtures, so the UI runs without a backend. */
export async function mockApi(page: Page, options: ApiMockOptions = {}): Promise<Captured> {
  const captured: Captured = { edits: [], cancels: 0, accepts: 0 }
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
  await page.route(`${base}/images/*/objects/0/mask`, (route) =>
    route.fulfill({ status: 200, contentType: 'image/png', body: mask }),
  )
  await page.route(`${base}/images/*/objects/0/widget`, (route) => {
    const { property } = route.request().postDataJSON() as { property: string }
    const widget = WIDGETS[property]
    return widget
      ? json(route, widget)
      : json(route, { code: 'not_found', message: 'property not found' }, 404)
  })
  await page.route(`${base}/images/*/edits`, (route) => {
    captured.edits.push(route.request().postDataJSON() as Record<string, unknown>)
    return json(route, { job_id: 'job-2' }, 202)
  })
  await page.route(`${base}/jobs/job-2/events`, (route) => {
    const outcome = options.edit ?? 'complete'
    if (outcome === 'hang') return undefined // never answer, so the job stays running
    const progress = [
      { type: 'progress', stage: 'analysing', message: 'Preparing your edits' },
      { type: 'progress', stage: 'generating', message: 'Generating the edited image' },
      { type: 'log', line: 'Generated by google/gemini-3-pro-image in 19.2s · $0.140' },
    ]
    const ending =
      outcome === 'complete'
        ? [{ type: 'completed', result: EDIT_RESULT }]
        : [
            {
              type: 'failed',
              code: outcome === 'fail-key' ? 'unavailable' : 'refused',
              message:
                outcome === 'fail-key'
                  ? 'An OpenRouter API key is required. Add one in Settings.'
                  : 'The image model declined this edit on content-policy grounds.',
            },
          ]
    return route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: sse([...progress, ...ending]),
    })
  })
  await page.route(`${base}/jobs/job-2/result`, (route) =>
    route.fulfill({ status: 200, contentType: 'image/jpeg', body: afterPhoto }),
  )
  await page.route(`${base}/jobs/job-2/accept`, (route) => {
    captured.accepts += 1
    return json(route, ACCEPTED_IMAGE, 201)
  })
  await page.route(`${base}/jobs/job-2`, (route) => {
    if (route.request().method() === 'DELETE') captured.cancels += 1
    return route.fulfill({ status: 202 })
  })
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
  return captured
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

/** Drag a handle by `(dx, dy)` screen pixels using the mouse. */
export async function dragHandle(page: Page, handle: Locator, dx: number, dy: number) {
  const box = await handle.boundingBox()
  if (!box) throw new Error('handle not visible')
  const x = box.x + box.width / 2
  const y = box.y + box.height / 2
  await page.mouse.move(x, y)
  await page.mouse.down()
  await page.mouse.move(x + dx / 2, y + dy / 2, { steps: 4 })
  await page.mouse.move(x + dx, y + dy, { steps: 4 })
  await page.mouse.up()
}
