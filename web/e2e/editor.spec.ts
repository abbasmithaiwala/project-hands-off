import { expect, test } from '@playwright/test'
import { clickCouch, mockApi, openEditor } from './support.js'

test.beforeEach(async ({ page }) => {
  await mockApi(page)
})

test('landing page invites a photo', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Edit by dragging, not typing.' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Choose an image' })).toBeVisible()
})

test('uploading a photo shows it, finds the object and reports progress', async ({ page }) => {
  await openEditor(page)
  await expect(page.getByText('1 object found')).toBeVisible()
  await expect(page.getByTestId('canvas-stage').locator('canvas')).toBeVisible()
})

test('clicking an object selects it and offers AI-suggested changes', async ({ page }) => {
  await openEditor(page)
  await clickCouch(page)

  await expect(page.getByRole('region', { name: 'Edit couch' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Couch length' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Armrest contour' })).toBeVisible()
  await expect(page.getByText('couch', { exact: true }).first()).toBeVisible()
})

test('picking a change marks it active, and Escape clears the selection', async ({ page }) => {
  await openEditor(page)
  await clickCouch(page)
  const length = page.getByRole('button', { name: 'Couch length' })
  await length.click()
  await expect(length).toHaveAttribute('aria-pressed', 'true')

  await page.getByTestId('canvas-stage').press('Escape')
  await expect(page.getByRole('region', { name: 'Edit couch' })).toBeHidden()
})

test('the whole flow works from the keyboard', async ({ page }) => {
  await openEditor(page)
  await page.getByTestId('canvas-stage').focus()
  await page.keyboard.press('ArrowRight')
  await expect(page.getByRole('region', { name: 'Edit couch' })).toBeVisible()
  await expect(page.getByText('Selected couch')).toBeAttached()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('region', { name: 'Edit couch' })).toBeHidden()
})

test('the activity panel shows the analysis log', async ({ page }) => {
  await openEditor(page)
  await page.getByRole('button', { name: 'Activity' }).click()
  const panel = page.getByRole('complementary', { name: 'Activity log' })
  await expect(panel).toContainText('Finding objects')
  await expect(panel).toContainText('Found 1 objects')
})

test('zooming and panning the canvas do not break it', async ({ page }) => {
  await openEditor(page)
  const box = await page.getByTestId('canvas-stage').boundingBox()
  if (!box) throw new Error('canvas not visible')
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.keyboard.down('Control')
  await page.mouse.wheel(0, -300)
  await page.keyboard.up('Control')
  await page.mouse.move(box.x + 200, box.y + 200)
  await page.mouse.down()
  await page.mouse.move(box.x + 320, box.y + 260, { steps: 5 })
  await page.mouse.up()
  await page.getByTestId('canvas-stage').press('0')
  await expect(page.getByText('1 object found')).toBeVisible()
})

test('New image returns to the landing page', async ({ page }) => {
  await openEditor(page)
  await page.getByRole('button', { name: 'New image' }).click()
  await expect(page.getByRole('heading', { name: 'Edit by dragging, not typing.' })).toBeVisible()
})

test('the theme can be switched and is remembered', async ({ page }) => {
  await openEditor(page)
  await page.getByRole('button', { name: 'Settings' }).click()
  await page.getByRole('radio', { name: 'Dark' }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
})
