import { expect, test } from '@playwright/test'
import { clickCouch, mockApi, openEditor } from './support.js'

test('the API key is requested inline, only when first needed', async ({ page }) => {
  await mockApi(page, { needsKey: true })
  await openEditor(page)
  await expect(page.getByRole('dialog')).toBeHidden()

  await clickCouch(page)
  const dialog = page.getByRole('dialog', { name: 'Connect OpenRouter' })
  await expect(dialog).toBeVisible()

  await dialog.getByLabel('OpenRouter API key').fill('sk-or-v1-test-key-0000')
  await dialog.getByRole('button', { name: 'Save key' }).click()

  await expect(dialog).toBeHidden()
  await expect(page.getByRole('button', { name: 'Couch length' })).toBeVisible()
})
