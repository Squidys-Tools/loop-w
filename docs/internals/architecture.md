# Architecture

LoopW is one tray-resident process: a global hook decides, Win32 moves the
window, iced draws settings and overlays. The split is deliberate —
`src/main.rs` lays it out:

- `src/core/` — pure decisions. Frame math, monitor selection, radial
  geometry, cycles, directional navigation, drag-snap math, stash math,
  command parsing. No `windows` crate imports. Everything here is unit
  testable without a desktop.
- `src/settings/` — the JSON contract plus its loader. Tolerant read,
  normalize, temp-file + rename write, revert-on-failure sync. Details in
  `settings-contract.md`.
- `src/win/` — every side effect. Hooks (`hooks.rs`), placement
  (`placement.rs`), snap (`snap_service.rs`), stash (`stash_service.rs`),
  IPC pipe (`ipc.rs`), tray, startup, instance mutex, diagnostics. Each file
  is a thin wrapper: resolve the decision in `core`, then call Win32 once.
- `src/ui/` — the iced daemon. Settings views, radial/preview canvas
  renderers, runtime wiring. Renderers draw the frame they are given; they
  never hit-test, hook input, or touch the pipe. Details in
  `hooks-overlays.md`.
- `src/cli.rs` — entry behavior. With args: forward to the resident via the
  pipe and print the reply; if no resident exists, become resident and run
  the command once. With no args: activate the resident or become resident
  hidden.

## Why this split

Hit-testing and drawing have opposite constraints. The hook thread must
never block on input; the renderer must never decide placement. The one
place they meet is a resolved target frame passed by value. That is why
`FillAvailableSpace` is the only preview that re-resolves per tick — it
depends on live neighbor windows — while every other wedge result is cached
per monitor generation (see `BUGS.md` #6).

## What the code cannot carry

- UIPI: a standard-user LoopW cannot move an elevated window. Fail safe with
  a friendly reason, never move a neighbor instead.
- Per-monitor V2 pixels: all rects are physical pixels
  (`SetProcessDpiAwarenessContext ..._PER_MONITOR_AWARE_V2`, see `main.rs`).
  Logical-size math belongs in `core`; DPI scaling at the call site is a bug.
- GUI-subsystem stdout: PowerShell capture into a windowed process has no
  pipe to write to, so `print_cli_line` writes directly only when stdout is
  a valid handle and otherwise falls back to `AttachConsole`. Same mechanics
  as the old C# build — parity, not a regression (`BUGS.md` #3).
- WGPU backend: forced to `dx11` unless the user overrides, because default
  DX12 presents blank on some Intel drivers. A 2D utility picks the reliable
  path (`main.rs`).
