import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: { '/api': 'http://127.0.0.1:8080' },
  },
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
    // Component tests render a lot of DOM; on a busy machine the default 5 s is not enough.
    testTimeout: 20_000,
    coverage: { provider: 'v8', include: ['src/**'], exclude: ['src/test/**', 'src/main.tsx'] },
  },
})
