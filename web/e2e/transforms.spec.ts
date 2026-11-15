import { expect, test } from '@playwright/test'
import { clickCouch, dragHandle, mockApi, openEditor, type Captured } from './support.js'

let captured: Captured

test.beforeEach(async ({ page }) => {
  captured = await mockApi(page, { transforms: true })
  await openEditor(page)
  await clickCouch(page)
})

const pick = (page: import('@playwright/test').Page, name: string) =>
  page.getByRole('button', { name }).click()

test('a tilt is a ring you turn, not a slider', async ({ page }) => {
  await pick(page, 'Couch tilt')
  const handle = page.getByRole('slider', { name: /Couch tilt of couch/ })
  await expect(handle).toHaveAttribute('aria-roledescription', 'rotation handle')
  await expect(handle).toHaveAttribute('aria-valuetext', 'not turned')

  await dragHandle(page, handle, 140, 60)

  const degrees = Number(await handle.getAttribute('aria-valuenow'))
  expect(degrees).toBeGreaterThan(5)
  expect(degrees).toBeLessThanOrEqual(45)
  await page.getByRole('button', { name: '1 change' }).click()
  await expect(page.getByRole('list', { name: 'Pending changes' })).toContainText(
    `Couch tilt · couch+${degrees}°`,
  )
  await expect(page.getByRole('note')).toContainText('Preview is approximate')
  await page.screenshot({ path: 'test-results/shot-rotate.png' })
})

test('the tilt can be turned either way from the keyboard and reset', async ({ page }) => {
  await pick(page, 'Couch tilt')
  const handle = page.getByRole('slider', { name: /Couch tilt of couch/ })
  await handle.focus()

  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('ArrowLeft')
  await expect(handle).toHaveAttribute('aria-valuenow', '-10')
  await expect(handle).toHaveAttribute('aria-valuetext', '10 degrees anticlockwise')
  await page.keyboard.press('Shift+ArrowRight')
  await expect(handle).toHaveAttribute('aria-valuenow', '-9')
  await page.keyboard.press('Home')
  await expect(handle).toHaveAttribute('aria-valuenow', '0')
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
})

test('resizing grows the whole object evenly from a corner handle', async ({ page }) => {
  await pick(page, 'Couch size')
  const handle = page.getByRole('slider', { name: /Couch size of couch/ })
  await expect(handle).toHaveAttribute('aria-roledescription', 'resize handle')

  await dragHandle(page, handle, 70, -50)

  expect(Number(await handle.getAttribute('aria-valuenow'))).toBeGreaterThan(100)
  await page.screenshot({ path: 'test-results/shot-scale.png' })
})

test('moving carries the object, and the edit is sent as a move', async ({ page }) => {
  await pick(page, 'Couch position')
  const handle = page.getByRole('button', { name: /Couch position of couch/ })
  await expect(handle).toHaveAttribute('aria-roledescription', 'move handle')

  await dragHandle(page, handle, -90, -30)

  await expect(handle).toHaveAttribute('aria-valuetext', /moved \d+% left, \d+% up/)
  await page.screenshot({ path: 'test-results/shot-move.png' })

  await page.getByRole('button', { name: 'Apply' }).click()
  await expect.poll(() => captured.edits.length).toBe(1)
  const sent = captured.edits[0]?.['edits'] as
    Array<{ change: Record<string, unknown> }> | undefined
  const change = sent?.[0]?.change
  expect(change?.['kind']).toBe('move')
  expect(Number(change?.['dx'])).toBeLessThan(0)
  expect(Number(change?.['dy'])).toBeLessThan(0)
})

test('the tip describes the control that is showing', async ({ page }) => {
  await pick(page, 'Couch tilt')
  await expect(page.getByRole('status').filter({ hasText: 'Drag the handle' })).toContainText(
    'around the ring to turn it',
  )
})
