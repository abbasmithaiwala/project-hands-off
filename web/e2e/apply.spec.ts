import { expect, test, type Page } from '@playwright/test'
import { clickCouch, mockApi, openEditor, type Captured, type EditOutcome } from './support.js'

async function editLength(page: Page) {
  await clickCouch(page)
  await page.getByRole('button', { name: 'Couch length' }).click()
  const handle = page.getByRole('slider', { name: /Couch length of couch/ })
  await handle.focus()
  for (let i = 0; i < 8; i++) await page.keyboard.press('ArrowLeft')
}

async function start(page: Page, edit: EditOutcome = 'complete'): Promise<Captured> {
  const captured = await mockApi(page, { edit })
  await openEditor(page)
  await editLength(page)
  return captured
}

test('applying sends the typed edits, the affected region and the on-screen preview', async ({
  page,
}) => {
  const captured = await start(page)
  await page.getByRole('button', { name: 'Apply' }).click()
  await expect(page.getByRole('region', { name: 'Review your edit' })).toBeVisible()

  expect(captured.edits).toHaveLength(1)
  const [request] = captured.edits as [Record<string, any>]
  expect(request.edits).toEqual([
    {
      object_id: 0,
      property: 'couch_length',
      change: {
        kind: 'slider',
        value: expect.closeTo(0.6, 5),
        axis: { start: { x: 0.215, y: 0.72 }, end: { x: 0.69, y: 0.72 } },
      },
    },
  ])

  const png = Buffer.from(request.preview_png as string, 'base64')
  expect([...png.subarray(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  expect(png.length).toBeGreaterThan(10_000)
  expect(request).not.toHaveProperty('model')
})

test('reshaping sends the new outline drawn on the photo instead of a warped preview', async ({
  page,
}) => {
  const captured = await mockApi(page)
  await openEditor(page)
  await clickCouch(page)
  await page.getByRole('button', { name: 'Armrest contour' }).click()
  const handle = page.getByRole('button', { name: /Armrest contour point 1 of/ })
  await handle.focus()
  for (let i = 0; i < 6; i++) await page.keyboard.press('ArrowLeft')
  await page.getByRole('button', { name: 'Apply' }).click()
  await expect(page.getByRole('region', { name: 'Review your edit' })).toBeVisible()

  const [request] = captured.edits as [Record<string, any>]
  expect(request.preview_kind).toBe('outline')
  const png = Buffer.from(request.preview_png as string, 'base64')
  expect([...png.subarray(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  expect(png.length).toBeGreaterThan(10_000)
})

test('the result can be reviewed against the original, then accepted as a new image', async ({
  page,
}) => {
  const captured = await start(page)
  await page.getByRole('button', { name: 'Apply' }).click()

  const review = page.getByRole('region', { name: 'Review your edit' })
  await expect(review).toContainText('Shortened the couch to about 60% of its length')
  await expect(review).toContainText('gemini-3-pro-image · 19.2s · $0.14')
  await expect(page.getByAltText('Before: the original photo')).toBeVisible()
  await expect(page.getByAltText('After: the edited photo')).toBeVisible()

  const divider = page.getByRole('slider', { name: 'Compare before and after' })
  await divider.focus()
  await page.keyboard.press('ArrowRight')
  await expect(divider).toHaveAttribute('aria-valuenow', '55')
  await page.screenshot({ path: 'test-results/shot-review.png' })

  await page.getByRole('button', { name: 'Accept' }).click()
  await expect(page.getByText('Applied: Shortened the couch')).toBeVisible()
  await expect(page.getByRole('region', { name: 'Review your edit' })).toBeHidden()
  await expect(page.getByText('1 object found')).toBeVisible()
  expect(captured.accepts).toBe(1)
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
})

test('the activity panel shows what happened, including time and cost', async ({ page }) => {
  await start(page)
  await page.getByRole('button', { name: 'Apply' }).click()
  await expect(page.getByRole('region', { name: 'Review your edit' })).toBeVisible()
  await page.getByRole('button', { name: 'Activity' }).click()
  await expect(page.getByRole('complementary', { name: 'Activity log' })).toContainText(
    'in 19.2s · $0.140',
  )
})

test('a faster draft retries with the draft model', async ({ page }) => {
  const captured = await start(page)
  await page.getByRole('button', { name: 'Apply' }).click()
  await page.getByRole('button', { name: 'Faster draft' }).click()
  await expect.poll(() => captured.edits.length).toBe(2)
  expect(captured.edits[1]).toMatchObject({ model: 'google/gemini-3.1-flash-image' })
})

test('discarding the result returns to editing with the changes intact', async ({ page }) => {
  await start(page)
  await page.getByRole('button', { name: 'Apply' }).click()
  await page.getByRole('button', { name: 'Discard' }).click()
  await expect(page.getByRole('region', { name: 'Review your edit' })).toBeHidden()
  await expect(page.getByRole('slider', { name: /Couch length of couch/ })).toHaveAttribute(
    'aria-valuenow',
    '60',
  )
  await expect(page.getByRole('button', { name: 'Apply' })).toBeEnabled()
})

test('a running edit can be cancelled', async ({ page }) => {
  const captured = await start(page, 'hang')
  await page.getByRole('button', { name: 'Apply' }).click()
  const progress = page.getByRole('status', { name: 'Applying your edits' })
  await expect(progress).toBeVisible()
  await expect(page.getByRole('slider')).toHaveCount(0)

  await progress.getByRole('button', { name: 'Cancel' }).click()

  await expect(progress).toBeHidden()
  await expect.poll(() => captured.cancels).toBe(1)
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeVisible()
})

test('a refusal is explained and the edits are kept so another try is possible', async ({
  page,
}) => {
  await start(page, 'fail')
  await page.getByRole('button', { name: 'Apply' }).click()
  await expect(page.getByText('declined this edit on content-policy grounds')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Apply' })).toBeEnabled()
})

test('a missing key during Apply opens the key prompt', async ({ page }) => {
  await start(page, 'fail-key')
  await page.getByRole('button', { name: 'Apply' }).click()
  await expect(page.getByRole('dialog', { name: 'Connect OpenRouter' })).toBeVisible()
})
