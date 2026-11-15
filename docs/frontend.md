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
| `rotate` | a handle on a ring round a pivot, a wedge showing the turn, a tick at each limit | turns the object about the pivot (clockwise is positive) | `{ kind: "rotate", degrees, pivot }` |
| `scale` | a corner handle joined to a fixed anchor by a thin line | resizes the whole object evenly about the anchor | `{ kind: "scale", value, pivot }` (a multiplier, `1` = unchanged) |
| `move` | one handle on the object's body, an arrow back to where it started | carries the whole object, shape and size unchanged | `{ kind: "move", dx, dy }` (shares of the image width and height) |

The slider, rotate, scale and move controls also draw a dashed frame round the object that follows the edit, with a faint copy of where it started, so the effect of a drag reads at a glance. Handles with a single meaning carry an icon (a turn arrow, a resize arrow, a move cross); the tip shown on first use is worded for the control on screen.

All coordinates are normalised to the image, so an edit means the same thing at any zoom or resolution. The component for each kind lives in `features/editor/widgets/` and is registered in `registry.ts`; a `satisfies` clause makes the compiler fail if the API gains a widget kind without one.

- **Constraints.** Points stay inside the image and within a fraction of the object's size of where they started. Dropping a point almost where it began snaps it back. Holding Shift locks a drag to one axis.
- **Keyboard.** Arrow keys move a focused handle (1% of the image; sliders and resize 5%; rotate 5 degrees, right or up turns clockwise); Shift gives fine steps (0.1%; sliders and resize 1%; rotate 1 degree); Home resets it. Handles announce their label and position, for example "Shape point 3 of 8 on couch, 42 percent across, 61 percent down".
- **Pending edits.** One edit per object and property, kept when the user switches property or object. The action bar lists them, each with a reset button; a control dragged back to where it started removes its edit. Cancel clears everything.
- **Apply** captures what is on screen (the photo with the warped previews, without outlines or controls) and sends it with the typed edits and the areas they affect. Editing is locked while it runs.

## Applying and reviewing

1. **Progress.** A pill names the stage (preparing, describing, generating, blending), counts seconds and offers Cancel, which abandons the model request.
2. **Review.** The result opens over the photo with a before/after divider (drag it, or use the arrow keys, Home and End), a short change log, and the model, time and cost. The activity panel carries the full log, including the instruction sent to the model.
3. **Choose.** Accept stores the result as a new image and re-analyses it so it can be edited further. Retry runs the same edits again; Faster draft retries with the configured draft model. Discard goes back to editing with every change intact.
4. **Failures** (refusals, a missing key, credits, a dropped connection) appear as a message, keep the edits, and a missing key opens the key prompt.

## The live preview

Dragging shows an approximate preview straight away, with no network call, and says so on screen.

1. When an object is first edited, its mask is fetched and used to cut the object out of the photo (a texture), and a low-resolution, smoothly filled patch is built to cover the spot it leaves behind.
2. The cutout is mapped onto a 28 x 28 grid mesh. Each drag recomputes the vertex positions: sliders stretch along the axis, a tilt rotates, a resize scales evenly and a move translates (all four move the object's own pixels, so they show the object itself); points use affine Moving Least Squares (Schaefer et al., 2006), which passes exactly through every dragged point and is smooth in between. Eight pins around the object keep the warp local.
3. PixiJS draws the mesh. The selection outline is deformed with the same function, so it follows the object.

Several edits on one object are applied in order; several edited objects each get their own preview layer.

The maths is in `lib/warp.ts` (pure, in image pixels so aspect ratio never skews it), the background fill in `lib/fill.ts`, and the slider, rotate and constraint helpers in `lib/widgetMath.ts`.

**Performance.** Deforming the whole grid with 24 control points takes about 0.15 ms (`warp.perf.test.ts`), roughly a hundredth of a 60 fps frame, so no Web Worker is used; the GPU draws the mesh. The fill runs once per object, at 96 px.

**Limits.** It is a preview, not the result: the patch is a blur of nearby colours, not inpainting, and the stretch moves existing pixels rather than inventing new ones. Real quality comes from the image model in Phase 7.

## Tests

| Layer | Tool | Covers |
|---|---|---|
| Unit | Vitest | geometry, viewport and warp maths (including property checks such as exact interpolation), fill, widget maths, previews, stores, API helpers, SSE, theme |
| Component | Vitest + Testing Library | every screen and dialog, all six widgets (drag, keyboard, constraints), the edit flow and previews, the API key flow, error states |
| Accessibility | vitest-axe | landing, editor with a selection, both dialogs |
| End to end | Playwright (real Chromium, mocked API) | upload, analysis, selection, AI suggestions, key prompt, dragging every widget kind, keyboard control, reset and cancel, zoom and pan, theme |
| Visual | Playwright screenshots | landing, selected object and mid-edit with live preview, in dark and light (local only: baselines depend on the platform's fonts) |

The canvas pixels themselves are only checked by the end-to-end and visual tests.

## Not yet built

Undo history, project persistence and cleanup of discarded results come in Phase 8.

## Reshaping

Applying a reshape sends the original photo with the new outline drawn over it in magenta (`lib/guide.ts`), not a warped render; see ADR 0009. Sliders and handles still send the warped preview.

Control points sit on the object's outline (the server snaps model-chosen points onto it). Dragging one moves only the border; the photo is not warped on screen. The warped reference image is still rendered off-screen when you press Apply so the model has geometry to follow. Sliders keep their live stretch.

## History, projects and first run

- **Versions.** The editor store holds every version of the open project and a cursor. Undo and redo (buttons, Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z) move the cursor; accepting an edit appends a version and drops any redo tail. Moving is disabled while an edit is applying or under review.
- **History panel.** Lists versions newest first with their change summaries; selecting one jumps to it.
- **Projects.** The project name in the top bar renames on click. The landing screen lists recent projects (open, or delete after confirmation). The address `#/p/<id>` reopens the project after a reload.
- **Export.** Download the current image, or a JSON edit log built from the version history.
- **First run.** A sample photo (credited in `public/sample/ATTRIBUTION.md`) is one click away. Two short tips (pick an object, drag the handle) appear once each and are remembered. Empty or failed analysis offers Try again or Choose another photo.

## Control style

Modelled on Adobe's Project Hands On: quiet, white, translucent. A slider is a soft bar laid across the object with its name in small capitals, a fixed end and a draggable handle (no dashed track). A reshape control is the object's outline reduced to its corners, drawn as straight thin lines with a small disc at each corner. A handle gets a square frame when you hover or grab it, and turns orange once moved. While a control is showing, the orange selection outline is hidden so only one thing is drawn. Reshape corners come from the detected outline (`corner_points`), not from a model, so they are instant and always on the edge.
