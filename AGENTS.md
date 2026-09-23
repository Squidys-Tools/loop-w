# LoopW AGENTS.md

LoopW is a Windows tray-resident window manager. Hold the trigger, move toward
a direction, release to place the active window. Rust + iced 0.14 + Win32,
self-contained `win-x64` EXE, no .NET runtime.

## Words we use

- **you** — the agent changing this repo.
- **user** — the person running LoopW on their desktop.
- **target** — the foreground window the action should hit. Never our own
  settings, radial, preview, or tray surfaces.
- **resident** — the single tray process. One hook, one pipe server, one tray
  icon. Second launch forwards to it and exits.
- **trigger** — the held key/mouse button that opens the radial (default Caps Lock).
- **overlay** — the radial menu plus its preview frame. Radial stays
  geometry-only, no text.
- **wedge / slot** — one of eight radial directions plus the center action.
- **keybind** — a global hotkey bound to an action or cycle.
- **cycle** — repeated presses stepping half, third, two-thirds.
- **stash** — park a window at a screen edge, reveal restores it.

## Safety: Windows desktop is live

The machine you build on is the machine you test on. Hooks, overlays, and
window moves affect the real desktop.

1. **Never kill by name.** Never `taskkill /IM LoopW.exe`, `pkill -f`, or kill
   a PID found by matching. Stop only the exact tracked run:
   `loopw-agent.ps1 stop --run-id <run-id> --yes --json`, after a
   `--dry-run` look. The skill (`verify-loopw`) has the sequence.
2. **Never drive the developer's resident instance.** Check
   `doctor --json` reports `resident_instance_detected: false` before a fresh
   run. Never run two verification sessions at once — the
   `Local\LoopW.Instance` mutex and `LoopW-Commands` pipe are per-user singletons.
3. **Never delete evidence or user state.** Evidence dirs under `$env:TEMP`
   are never removed during cleanup. Never hand-edit, move, or delete
   `%LOCALAPPDATA%\LoopW\settings.json` except through a documented repro, and
   never commit a real one.
4. **Leave no input behind.** Quit removes hooks and overlays. If a run leaves
   a stuck overlay, suppressed key, or moved cursor, that is the bug to fix
   first. Save work before testing minimize, hide, stash, or focus actions.
5. **IPC stays local.** The named pipe is same-user only. No window titles,
   keystrokes, or app data leave the process. A cross-user pipe connection
   must be rejected.

## Where code lives

- `src/core/` — pure logic: frame math, monitors, radial geometry, cycles,
  navigation, snap, stash math, command parsing. No Win32 calls. Fully unit
  tested.
- `src/settings/` — the JSON contract (PascalCase keys, integer enums, same
  path and keys as the old C# build). Tolerant loader, temp-file + rename
  saves, revert-on-failure sync. See `docs/internals/settings-contract.md`.
- `src/win/` — all side effects: hooks, placement, snap service, stash
  service, IPC pipe, tray, startup, diagnostics. Thin wrappers over pure
  `core` decisions.
- `src/ui/` — iced daemon: settings window, radial/preview renderers, runtime.
  Renderers stay dumb; they draw frames `core`/`win` resolved.
- `src/cli.rs` + `src/main.rs` — entry: with args forward to resident and
  print its reply; without resident become resident and optionally run the
  command once.
- `tools/` — `loopw-agent.ps1` wrapper, `windows-integration.ps1` checks,
  `performance-measurement.ps1`. The wrapper is process control, not a second
  Win32 implementation.
- `docs/` — `user/` for how to use it, `internals/` for why it is built this
  way, `operations/` for how to build and ship it. `ROADMAP.md`, `QA.md`,
  `BUGS.md`, `PERFORMANCE.md` are status logs, not guides.
- `specs/` — frozen requirements and acceptance scenarios. Read before
  changing the settings surface.

## Entry-point parity

The most common LoopW defect is a fix that works on one path and is missing
on the others. An action must behave the same through radial, keybind, and
`LoopW.exe action/<name>` / `direction/<dir>`. Before calling action work done,
say which of those you exercised.

- If you add a way in, add the way out and the way to see it. Stash needs
  reveal. Bypass needs removal. A one-way door is a bug.
- Unsupported targets (elevated, UWP/WinUI, protected, fullscreen-exclusive,
  tool windows) fail safe with a friendly reason plus technical detail, never
  by moving the wrong window.
- Our own windows are never valid targets.

## Build and verify

Toolchain: Rust stable via `rustup`.

```powershell
cargo fmt -- --check
cargo build --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
```

During iteration run the narrowest check that covers the change
(`cargo test <name>`, one suite). Before calling work done run the full gate
above — this repo is small enough that the full suite (179 tests) *is* the
gate, plus `verify-loopw` for Win32 behavior and `docs/QA.md` for anything
involving trigger, overlay, DPI, or real windows. A passing IPC reply without
a target-geometry or overlay-ownership assertion is incomplete. A skipped
radial probe is a skip, not a pass.

## Taste

- Decisions live in `core`, effects live in `win`, drawing lives in `ui`.
  New code follows that split. Do not put hit-testing, hook logic, or pipe
  I/O into renderers, and do not put drawing math into the hook thread.
- Keep files small and modular. No 1000+ line files. The known split
  candidates are `src/win/stash_service.rs`, `src/win/hooks.rs`,
  `src/core/frame_math.rs`, and `src/win/ipc.rs` — shrink new code instead of
  growing them.
- Prefer total functions and explicit fallbacks: unknown actions, missing
  keybinds, stale stash records, and bad settings fall back to a safe no-op,
  never to garbage placement or a panic on the desktop path.
- `expect`/`unwrap` only where a failure is genuinely impossible; the hook,
  pipe, placement, and settings-load paths return and surface errors.
- Comments explain how a function is used and move with the code. Do not
  narrate every line. Hard Win32 constraints (UIPI, per-monitor V2 pixels,
  GUI-subsystem stdout limits) get one short comment at the call site.
- Users feel every dropped frame and stuck key. No per-tick frame resolution,
  no blocking work on the input thread, no continuously repainting animation.
  The 100 ms `NCHITTEST` budget and wedge frame cache exist for a reason —
  measure before changing them.

## Documentation

Most code changes do not need a docs change. Agents can read the code.

- Before adding a paragraph, ask what a maintainer would get wrong without it. If reading the relevant code answers the question, leave it out.
- `docs/user/` helps users get something done: what it does, how to start,
  what is unintuitive. No implementation details, no contributor tooling. A
  settings path is useful; a tour of every button is not.
- `docs/internals/` records decisions that cross modules, constraints the
  code cannot carry, and traps found the hard way. Link to source instead of
  copying it. When a decision changes, rewrite the text — do not append a
  second account.
- `docs/operations/` holds build, verify, and release procedures. Keep them
  runnable from a fresh checkout.
- `ROADMAP.md`, `QA.md`, `BUGS.md`, `PERFORMANCE.md` track state and evidence.
  Update the status line when a check actually ran; do not restate guides there.
- Do not commit plans, research notes, or agent scratch files. The merged PR
  is the record.

## Pull requests

Small and focused. One concern per PR — if the description says "also", split
it. See `CONTRIBUTING.md`. Conventional titles (`fix(win): ...`,
`feat(ui): ...`), problem then fix in the body, before/after screenshots for
UI, short video for motion or timing. Never commit PR-only screenshots.

## T3 Code instructions

Screenshots the user must SEE: tool-result images never render in chat —
always save via `screenshot_out_file` to
`C:\Users\$USER$\AppData\Local\Temp\opencode\chat-images\` (create it if
missing) and embed with `![what](C:\...\chat-images\name.png)`; only that
subfolder may ever be wiped.
