# Hooks and overlays

## Input hook

`src/win/hooks.rs` installs the global keyboard/mouse hook. The hook thread
does the cheapest possible gating (feature on, no session active, eligible
window) and never blocks on Win32. Activation delay and release timeout are
guarded by a state version: stale timer threads wake, see the mismatch, and
exit. That keeps rapid press/release churn correct at the cost of
short-lived threads (`BUGS.md` #1 documents the tradeoff and the fix
direction: single timer + cancel channel).

Drag-snap start (`snap_service.rs::begin_at_cursor`: `WindowFromPoint` +
policy + `NCHITTEST` with a 100 ms `SendMessageTimeout`) runs on the UI
thread per left-button press, matching the old C# behavior. The early-outs
keep the common path cheap; a hung foreground app stalling the timeout is
the known jank source (`BUGS.md` #5). Do not move this onto the hook thread
without measuring — blocking input is worse than a slow click.

## Overlays

`src/win/overlay.rs` + `src/ui/` own the `LoopW Radial` and `LoopW Preview`
windows. They are tool windows excluded from Alt+Tab and the taskbar, and
the Winit event-target helper gets the same treatment so only the tray icon
represents the app. The preview never takes focus; cancel always tears both
windows down together so no orphaned frame survives.

The settings-page canvases (`radial_canvas.rs`, `preview_canvas.rs`) render
in canvas-local coordinates at fixed sizes with cached geometry. The old
sliver bug was a doubled translation — absolute layout offset applied inside
iced's already-local space. The regression test pins the 2x symptom; the
remaining step is a Windows visual sign-off at real scale factors
(`BUGS.md` #9, `docs/QA.md` §3).
