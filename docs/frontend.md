# Frontend

React 19, TypeScript (strict), Vite, PixiJS for the canvas, TanStack Query for server state and Zustand for local state.

```sh
just dev                     # Vite dev server on :5173, proxies /api to :8080
just serve                   # the API (set HANDS_OFF_DEMO_AI=1 to try it without a key)
cd web && npm test           # unit and component tests (Vitest)
cd web && npm run test:e2e   # Playwright against a mocked API
cd web && npm run test:visual           # screenshot comparison (local, macOS baselines)
cd web && npm run test:visual:update    # regenerate the baselines
```

## Experience

- **Zero setup.** Drop, paste or choose a photo. Nothing is configured up front.
- **Canvas first.** The image fills the screen; chrome is small, translucent and out of the way. Controls appear in context: hover an object to outline it, click to select it, pick a change to see its control.
- **Inline key prompt.** The OpenRouter key is requested only when an AI call first needs it, in a small dialog, and the failed request is retried automatically after saving.
- **Advanced is hidden.** Theme, key and model choices live in a Settings dialog behind a gear icon. Models are tucked in a collapsed "Advanced" section.
- **Keyboard and screen readers.** The canvas is focusable: arrow keys move between objects, Escape clears, `+` `-` zoom, `0` fits. The selection is announced through a live region. Dialogs trap focus and restore it.
- **Themes.** Dark, light or system, applied before first paint and remembered.
- **Motion.** Short, purposeful transitions that switch off under `prefers-reduced-motion`.
- **Performance.** The landing page loads about 87 kB of JavaScript (gzipped); the editor and PixiJS load only once an image opens. Panning and zooming never touch the network.

## Structure

```
web/src/
  api/        generated OpenAPI types and client, typed hooks, SSE subscription, error helpers
  app/        providers, error boundary, theme, top-level App
  design/     tokens.css and primitives (Button, IconButton, Panel, Dialog, Tooltip, Toaster, Skeleton)
  features/
    landing/  drop zone, paste, file picker
    editor/   editor screen, top bar, property menu, activity panel, action bar
      canvas/ PixiJS stage (plain TypeScript) and its React wrapper
    settings/ API key and settings dialogs
  lib/        pure helpers (geometry, viewport math, colour, files), fully unit tested
  state/      Zustand stores
  test/       setup and shared test utilities
```

Conventions: components stay small and presentational; data fetching lives in `api/hooks.ts`; canvas logic is plain TypeScript with no React; colours, spacing, radii and motion come from `design/tokens.css` only.

## The canvas

`features/editor/canvas/stage.ts` owns the PixiJS application: image sprite, outline overlay, pan and zoom, hover and selection hit-testing. It talks to React through a small `StageHandle` interface, so the rest of the UI is tested with a fake stage and no WebGL.

- Wheel pans; Ctrl/Cmd + wheel (and trackpad pinch) zooms at the cursor; dragging pans; a click without dragging selects.
- Outlines are drawn with a constant on-screen width at any zoom. When objects overlap, the smallest containing object wins, so a cup held by a person stays selectable.
- Object labels are DOM elements positioned from the viewport, so they stay crisp and accessible.

## Tests

| Layer | Tool | Covers |
|---|---|---|
| Unit | Vitest | geometry, viewport math, colour parsing, stores, API helpers, SSE subscription, theme |
| Component | Vitest + Testing Library | every screen and dialog, the API key flow, keyboard control, error states |
| Accessibility | vitest-axe | landing, editor with a selection, both dialogs |
| End to end | Playwright (real Chromium, mocked API) | upload, analysis, selection, AI suggestions, key prompt, keyboard flow, zoom and pan, theme |
| Visual | Playwright screenshots | landing and editor in dark and light (local only: baselines depend on the platform's fonts) |

The canvas pixels themselves are only checked by the end-to-end and visual tests.

## Not yet built

Dragging the control, the live warp preview and Apply/Cancel behaviour arrive in Phase 6; the action bar and the pending-edit list already exist in the shell. Undo history and project persistence come in Phase 8.
