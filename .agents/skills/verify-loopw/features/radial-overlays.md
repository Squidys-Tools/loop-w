# Radial and preview overlays

Holding the configured trigger opens the radial menu near the cursor, and an
active wedge can create a preview overlay before the gesture is released.

## Sub-features

- `radial-open` creates an app-owned radial overlay.
- `preview-open` creates an app-owned preview overlay for an active wedge.
- `overlay-style` applies tool-window and click-through styles.
- `overlay-cleanup` removes the radial and preview overlays together.
- `trigger-release` verifies the physical release path manually when synthetic
  input is skipped.

## How to get to it (user POV)

- Hold the configured trigger over a disposable foreground window.
- Move across radial wedges and outside the overlay boundary.
- Release over a wedge, outside the outer edge, or in the center.
- Observe the preview position and resulting window action.

## Driving it with loopw-agent.ps1

Preconditions:

- The configured trigger is known and is not physically held.
- A disposable target can become the foreground window.
- `RadialEnabled` and, for preview assertions, `PreviewEnabled` are enabled.

- **Open and inspect.** Run `run --suite all --profile release
  --evidence-dir <evidence> --json`. When synthetic trigger input is accepted,
  require `radial overlay opens`, `radial overlay is app-owned`, and the radial
  tool-window style check.
- **Preview.** Require `preview overlay opens for an active wedge`, app-owned
  preview, and `preview click-through style` when PreviewEnabled is true.
- **Cleanup.** Require `radial close also removes preview` in the raw result.
- **Release.** If the result contains `SKIP radial trigger-release path`, use
  the physical trigger scenarios in `docs\QA.md` and report the automated path
  as unverified.
- **Proof.** Preserve the JSON summary, raw log, configured trigger values, and
  the exact skip reason if any input check is skipped.

## Gotchas

- Synthetic Caps Lock release is environment-sensitive and is deliberately
  skipped by the current harness path.
- Do not run this feature while the configured trigger or modifier is already
  held; the harness refuses to synthesize input in that case.
- A preview style or overlay HWND assertion does not prove visual rendering or
  wedge hit-testing outside the overlay. Those remain manual desktop checks.
