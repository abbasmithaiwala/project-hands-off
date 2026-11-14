import { expect, test } from '@playwright/test'
import { clickCouch, mockApi, openEditor } from './support.js'

// Screenshot baselines depend on the platform's fonts, so they are generated and checked
// locally (`npm run test:visual`) and are not part of the default run.
test.beforeEach(async ({ page }) => {
  await mockApi(page)
  await page.emulateMedia({ reducedMotion: 'reduce' })
})

for (const scheme of ['dark', 'light'] as const) {
  test(`landing page @visual (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' })
    await page.goto('/')
    await expect(page).toHaveScreenshot(`landing-${scheme}.png`)
  })

  test(`editor with a selected object @visual (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' })
    await openEditor(page)
    await clickCouch(page)
    await page.getByRole('button', { name: 'Couch length' }).click()
    await page.mouse.move(2, 2)
    await page.waitForTimeout(400)
    await expect(page).toHaveScreenshot(`editor-${scheme}.png`)
  })
}
