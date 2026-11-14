import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  retries: process.env['CI'] ? 1 : 0,
  reporter: process.env['CI'] ? 'github' : 'list',
  use: {
    baseURL: 'http://localhost:5173',
    viewport: { width: 1280, height: 800 },
    launchOptions: { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] },
    trace: 'retain-on-failure',
  },
  expect: { toHaveScreenshot: { maxDiffPixelRatio: 0.02 } },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env['CI'],
  },
})
