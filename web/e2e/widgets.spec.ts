import { expect, test } from '@playwright/test'
import { clickCouch, dragHandle, mockApi, openEditor } from './support.js'

test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await openEditor(page)
  await clickCouch(page)
})

const pick = (page: import('@playwright/test').Page, name: string) =>
  page.getByRole('button', { name }).click()

test('dragging the length slider shortens the couch and records one pending change', async ({
  page,
}) => {
  await pick(page, 'Couch length')
  const handle = page.getByRole('slider', { name: /Couch length of couch/ })
  await expect(handle).toHaveAttribute('aria-valuenow', '100')

  await dragHandle(page, handle, -180, 0)

  const value = Number(await handle.getAttribute('aria-valuenow'))
  expect(value).toBeLessThan(100)
  expect(value).toBeGreaterThanOrEqual(50)

  await expect(page.getByRole('group', { name: 'Pending edits' })).toContainText('1 change')
  await page.getByRole('button', { name: '1 change' }).click()
  await expect(page.getByRole('list', { name: 'Pending changes' })).toContainText(
    `Couch length · couch${value}%`,
  )
  await expect(page.getByRole('note')).toContainText('Preview is approximate')
  await page.screenshot({ path: 'test-results/shot-slider.png' })
})

test('arrow keys nudge the slider; Shift makes fine steps; Home resets', async ({ page }) => {
  await pick(page, 'Couch length')
  const handle = page.getByRole('slider', { name: /Couch length of couch/ })
  await handle.focus()

  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('ArrowLeft')
  await expect(handle).toHaveAttribute('aria-valuenow', '90')
  await page.keyboard.press('Shift+ArrowLeft')
  await expect(handle).toHaveAttribute('aria-valuenow', '89')
  await page.keyboard.press('ArrowRight')
  await expect(handle).toHaveAttribute('aria-valuenow', '94')

  await page.keyboard.press('Home')
  await expect(handle).toHaveAttribute('aria-valuenow', '100')
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
})

test('the slider cannot go past its limits', async ({ page }) => {
  await pick(page, 'Couch length')
  const handle = page.getByRole('slider', { name: /Couch length of couch/ })
  await dragHandle(page, handle, -2000, 0)
  await expect(handle).toHaveAttribute('aria-valuenow', '50')
})

test('control points reshape the object', async ({ page }) => {
  await pick(page, 'Armrest contour')
  const handles = page.getByRole('button', { name: /Armrest contour point \d+ of 9 on couch/ })
  await expect(handles).toHaveCount(9)

  await dragHandle(page, handles.nth(2), 60, -40)

  await page.getByRole('button', { name: '1 change' }).click()
  await expect(page.getByRole('list', { name: 'Pending changes' })).toContainText('1 point moved')
  await page.screenshot({ path: 'test-results/shot-points.png' })
})

test('multi-handle moves parts independently', async ({ page }) => {
  await pick(page, 'Cushion placement')
  const handles = page.getByRole('button', { name: /Cushion placement handle \d of 3 on couch/ })
  await expect(handles).toHaveCount(3)

  await dragHandle(page, handles.nth(0), 50, 0)
  await dragHandle(page, handles.nth(2), -40, 20)

  await page.getByRole('button', { name: '1 change' }).click()
  await expect(page.getByRole('list', { name: 'Pending changes' })).toContainText('2 points moved')
})

test('handles can be moved from the keyboard and reset one at a time', async ({ page }) => {
  await pick(page, 'Cushion placement')
  const first = page.getByRole('button', { name: /Cushion placement handle 1 of 3/ })
  await first.focus()
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('ArrowRight')
  await page.getByRole('button', { name: '1 change' }).click()
  await expect(page.getByRole('list', { name: 'Pending changes' })).toContainText('1 point moved')

  await first.focus()
  await page.keyboard.press('Home')
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
})

test('edits survive switching between properties', async ({ page }) => {
  await pick(page, 'Couch length')
  const slider = page.getByRole('slider', { name: /Couch length of couch/ })
  await slider.focus()
  await page.keyboard.press('ArrowLeft')
  await page.keyboard.press('ArrowLeft')

  await pick(page, 'Cushion placement')
  await expect(page.getByRole('slider')).toHaveCount(0)
  await pick(page, 'Couch length')
  await expect(page.getByRole('slider', { name: /Couch length of couch/ })).toHaveAttribute(
    'aria-valuenow',
    '90',
  )
  await expect(page.getByRole('img', { name: 'edited' })).toBeVisible()
})

test('a pending change can be reset from the list, or all can be cancelled', async ({ page }) => {
  await pick(page, 'Couch length')
  const slider = page.getByRole('slider', { name: /Couch length of couch/ })
  await slider.focus()
  await page.keyboard.press('ArrowLeft')
  await page.getByRole('button', { name: '1 change' }).click()
  await page.getByRole('button', { name: 'Reset Couch length · couch' }).click()
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
  await expect(slider).toHaveAttribute('aria-valuenow', '100')

  await slider.focus()
  await page.keyboard.press('ArrowLeft')
  await page.getByRole('button', { name: 'Cancel' }).click()
  await expect(page.getByRole('group', { name: 'Pending edits' })).toBeHidden()
  await expect(slider).toHaveAttribute('aria-valuenow', '100')
})
