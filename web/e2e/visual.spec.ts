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

  test(`editing a change with a live preview @visual (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' })
    await openEditor(page)
    await clickCouch(page)
    await page.getByRole('button', { name: 'Couch length' }).click()
    const handle = page.getByRole('slider', { name: /Couch length of couch/ })
    await handle.focus()
    for (let i = 0; i < 8; i++) await page.keyboard.press('ArrowLeft')
    await page.mouse.move(2, 2)
    await page.waitForTimeout(600)
    await expect(page).toHaveScreenshot(`editing-${scheme}.png`)
  })

  test(`reviewing the result @visual (${scheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' })
    await openEditor(page)
    await clickCouch(page)
    await page.getByRole('button', { name: 'Couch length' }).click()
    await page.getByRole('slider', { name: /Couch length of couch/ }).focus()
    for (let i = 0; i < 8; i++) await page.keyboard.press('ArrowLeft')
    await page.getByRole('button', { name: 'Apply' }).click()
    await expect(page.getByRole('region', { name: 'Review your edit' })).toBeVisible()
    await expect(page.getByAltText('After: the edited photo')).toBeVisible()
    await page.mouse.move(2, 2)
    await page.waitForTimeout(600)
    await expect(page).toHaveScreenshot(`review-${scheme}.png`)
  })
}
