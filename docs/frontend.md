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

## Editing controls

Picking a change places its control over the canvas. Controls are DOM elements over the canvas, so they are keyboard- and screen-reader-accessible by construction; the canvas only draws pixels.

| Kind | Looks like | Dragging it | Edit sent |
|---|---|---|---|
| `slider` | one handle on a line, resting at the object's edge | stretches the object along the line, anchored at the line's start | `{ kind: "slider", value }` (a multiplier, `1` = unchanged) |
| `control_points` | handles joined into an outline | reshapes the object around the dragged point | `{ kind: "control_points", origin, target }` |
| `multi_handle` | free-standing handles | moves parts independently, like puppet joints | `{ kind: "multi_handle", origin, target }` |

All coordinates are normalised to the image, so an edit means the same thing at any zoom or resolution. The component for each kind lives in `features/editor/widgets/` and is registered in `registry.ts`; a `satisfies` clause makes the compiler fail if the API gains a widget kind without one.

- **Constraints.** Points stay inside the image and within a fraction of the object's size of where they started. Dropping a point almost where it began snaps it back. Holding Shift locks a drag to one axis.
- **Keyboard.** Arrow keys move a focused handle (1% of the image; sliders 5%); Shift gives fine steps (0.1%; sliders 1%); Home resets it. Handles announce their label and position, for example "Shape point 3 of 8 on couch, 42 percent across, 61 percent down".
- **Pending edits.** One edit per object and property, kept when the user switches property or object. The action bar lists them, each with a reset button; a control dragged back to where it started removes its edit. Cancel clears everything.
- **Apply** is shown but disabled with an explanation until generation is connected.

## The live preview

Dragging shows an approximate preview straight away, with no network call, and says so on screen.

1. When an object is first edited, its mask is fetched and used to cut the object out of the photo (a texture), and a low-resolution, smoothly filled patch is built to cover the spot it leaves behind.
2. The cutout is mapped onto a 28 x 28 grid mesh. Each drag recomputes the vertex positions: sliders stretch along the axis; points use affine Moving Least Squares (Schaefer et al., 2006), which passes exactly through every dragged point and is smooth in between. Eight pins around the object keep the warp local.
3. PixiJS draws the mesh. The selection outline is deformed with the same function, so it follows the object.

Several edits on one object are applied in order; several edited objects each get their own preview layer.

The maths is in `lib/warp.ts` (pure, in image pixels so aspect ratio never skews it), the background fill in `lib/fill.ts`, and the slider and constraint helpers in `lib/widgetMath.ts`.

**Performance.** Deforming the whole grid with 24 control points takes about 0.15 ms (`warp.perf.test.ts`), roughly a hundredth of a 60 fps frame, so no Web Worker is used; the GPU draws the mesh. The fill runs once per object, at 96 px.

**Limits.** It is a preview, not the result: the patch is a blur of nearby colours, not inpainting, and the stretch moves existing pixels rather than inventing new ones. Real quality comes from the image model in Phase 7.

## Tests

| Layer | Tool | Covers |
|---|---|---|
| Unit | Vitest | geometry, viewport and warp maths (including property checks such as exact interpolation), fill, widget maths, previews, stores, API helpers, SSE, theme |
| Component | Vitest + Testing Library | every screen and dialog, all three widgets (drag, keyboard, constraints), the edit flow and previews, the API key flow, error states |
| Accessibility | vitest-axe | landing, editor with a selection, both dialogs |
| End to end | Playwright (real Chromium, mocked API) | upload, analysis, selection, AI suggestions, key prompt, dragging every widget kind, keyboard control, reset and cancel, zoom and pan, theme |
| Visual | Playwright screenshots | landing, selected object and mid-edit with live preview, in dark and light (local only: baselines depend on the platform's fonts) |

The canvas pixels themselves are only checked by the end-to-end and visual tests.

## Not yet built

Applying edits (generation) is Phase 7. Undo history and project persistence come in Phase 8.
