import { expect, test, type Page } from '@playwright/test'
import { clickCouch, mockApi, openEditor, photoPath, SAVED_PROJECT } from './support.js'

async function applyAndAccept(page: Page) {
  await clickCouch(page)
  await page.getByRole('button', { name: 'Couch length' }).click()
  const handle = page.getByRole('slider', { name: /Couch length of couch/ })
  await handle.focus()
  for (let i = 0; i < 8; i++) await page.keyboard.press('ArrowLeft')
  await page.getByRole('button', { name: 'Apply' }).click()
  await page.getByRole('button', { name: 'Accept' }).click()
  await expect(page.getByText('Applied: Shortened the couch')).toBeVisible()
}

test('an accepted edit becomes a version that can be undone and redone', async ({ page }) => {
  await mockApi(page)
  await openEditor(page)
  await expect(page.getByRole('button', { name: 'Undo' })).toBeDisabled()
  await applyAndAccept(page)

  await page.getByRole('button', { name: 'History' }).click()
  const history = page.getByRole('complementary', { name: 'Version history' })
  await expect(history.getByRole('button')).toHaveCount(2)
  await expect(history.getByRole('button').first()).toHaveAttribute('aria-current', 'step')

  await page.getByRole('button', { name: 'Undo' }).click()
  await expect(history.getByRole('button', { name: /Original/ })).toHaveAttribute(
    'aria-current',
    'step',
  )
  await expect(page.getByRole('button', { name: 'Redo' })).toBeEnabled()

  await page.keyboard.press('ControlOrMeta+Shift+Z')
  await expect(history.getByRole('button').first()).toHaveAttribute('aria-current', 'step')
})

test('reloading reopens the project', async ({ page }) => {
  await mockApi(page)
  await openEditor(page)
  await applyAndAccept(page)
  await page.reload()
  await expect(page.getByRole('button', { name: 'Undo' })).toBeEnabled()
  await expect(page.getByText('1 object found')).toBeVisible()
})

test('a project can be renamed from the top bar', async ({ page }) => {
  const captured = await mockApi(page)
  await openEditor(page)
  await page.getByRole('button', { name: /Rename/ }).click()
  await page.getByRole('textbox', { name: 'Project name' }).fill('Living room')
  await page.keyboard.press('Enter')
  await expect.poll(() => captured.renames).toEqual(['Living room'])
  await expect(page.getByRole('button', { name: /Living room/ })).toBeVisible()
})

test('the edit log can be downloaded', async ({ page }) => {
  await mockApi(page)
  await openEditor(page)
  await page.getByRole('button', { name: 'Export' }).click()
  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: 'Download edit log' }).click()
  expect((await download).suggestedFilename()).toBe('untitled-edits.json')
})

test('discarding a result removes it from the server', async ({ page }) => {
  const captured = await mockApi(page)
  await openEditor(page)
  await clickCouch(page)
  await page.getByRole('button', { name: 'Couch length' }).click()
  await page.getByRole('slider', { name: /Couch length of couch/ }).focus()
  for (let i = 0; i < 8; i++) await page.keyboard.press('ArrowLeft')
  await page.getByRole('button', { name: 'Apply' }).click()
  await page.getByRole('button', { name: 'Discard' }).click()
  await expect.poll(() => captured.discards).toBe(1)
})

test('saved projects appear on the landing screen and can be opened or deleted', async ({
  page,
}) => {
  const captured = await mockApi(page, { recent: [SAVED_PROJECT] })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'Recent projects' })).toBeVisible()

  await page.getByRole('button', { name: 'Delete Living room' }).click()
  await page.getByRole('button', { name: 'Delete', exact: true }).click()
  await expect.poll(() => captured.deletedProjects).toHaveLength(1)
  await expect(page.getByRole('heading', { name: 'Recent projects' })).toBeHidden()
})

test('opening a saved project shows the editor', async ({ page }) => {
  await mockApi(page, { recent: [SAVED_PROJECT] })
  await page.goto('/')
  await page.getByRole('button', { name: 'Open Living room' }).click()
  await expect(page.getByText('1 object found')).toBeVisible()
})

test('first-run tips guide the first edit', async ({ page }) => {
  await mockApi(page)
  await openEditor(page)
  await expect(page.getByRole('status').filter({ hasText: 'click it' })).toBeVisible()
  await clickCouch(page)
  await page.getByRole('button', { name: 'Couch length' }).click()
  await expect(page.getByRole('status').filter({ hasText: 'Drag the handle' })).toBeVisible()
})

test('the sample photo opens without any upload dialog', async ({ page }) => {
  await mockApi(page)
  await page.route('**/sample/couch.jpg', async (route) =>
    route.fulfill({ status: 200, contentType: 'image/jpeg', path: photoPath }),
  )
  await page.goto('/')
  await page.getByRole('button', { name: 'Try a sample photo' }).click()
  await expect(page.getByText('1 object found')).toBeVisible()
})
