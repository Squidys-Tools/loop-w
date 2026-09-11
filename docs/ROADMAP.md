# LoopW roadmap

LoopW is a Windows window manager built around a hold-to-open radial menu. The
main product path is in the code now. This roadmap tracks the work that still
needs to be verified or hardened before a wider release.

LoopW targets Rust, iced 0.14, and native Windows APIs. It runs as a tray resident
app and saves settings to `%LOCALAPPDATA%\\LoopW\\settings.json`.

## Current state

The following product areas are implemented:

- The global trigger supports keyboard modifiers, left/right modifier choice,
  activation delay, release timeout, double-click activation, and optional
  middle-mouse activation.
- The radial menu opens at the cursor, selects by direction, supports eight
  saved wedge targets plus a center target, and keeps the active overlay free of
  text.
- The action catalog includes window placement, resizing, monitor movement,
  focus navigation, minimize/hide, undo, fill-available-space, and stash
  actions.
- Keybinds support add, rebind, delete, duplicate detection, cycling, and
  bypassing the trigger. Radial slots can point to built-in actions, cycles, or
  stable keybind IDs.
- The settings surface uses a persistent left navigation pane with General,
  Radial menu, Preview, Appearance, and Advanced sections. It includes inline
  trigger capture, presets, custom colors, auto-save feedback, section reset,
  and confirmed reset-all behavior.
- Drag snapping, target previews, stash and reveal, monitor move policies,
  screen padding, application exclusions, tray lifecycle, launch at login, and
  single-instance activation are wired into the runtime.
- The Winit event-target helper is kept out of the taskbar with a tool-window
  style and a taskbar-tab removal call; it remains visible to Winit for paint
  delivery.
- A same-user named-pipe command server supports activation, action listing,
  keybind listing, directional actions, and named actions.
- The automated suite currently contains 98 binary tests and 72 UI/settings
  contract tests, for 170 tests in total. It covers frame math, radial
  geometry, cycles, navigation, settings normalization, stash calculations,
  drag snapping, and command parsing.

The design document for the settings surface remains in
[`specs/settings-ui-redesign-spec.md`](../specs/settings-ui-redesign-spec.md).
It records the settings requirements and acceptance scenarios. The current Rust
and iced implementation status is summarized in this roadmap. [`QA.md`](QA.md)
is the desktop test checklist.

### Remaining implementation and hardening

- [x] Apply the settings-canvas rendering fix documented in
  [`BUGS.md`](BUGS.md#9-settings-canvas-rendering-fix-needs-desktop-verification).
  Both canvases now use local coordinates, fixed dimensions, and cached
  geometry. The desktop rendering result is still unverified.
- [x] Split the former `src/ui/app.rs` monolith into focused settings, runtime,
  and window modules. The remaining large Win32/core modules are follow-up
  refactors:
  `src/win/stash_service.rs`, `src/win/hooks.rs`, `src/core/frame_math.rs`, and
  `src/win/ipc.rs`.

## Next work

### Finish the desktop QA pass

- [ ] Run the complete checklist in [`QA.md`](QA.md) on a local Windows setup.
- [ ] Cover keyboard-only use, visible focus, reserved-key errors, and normal
  typing with trigger options enabled and disabled.
- [ ] Test the settings surface at 100%, 125%, 150%, and 200% scaling.
- [ ] Exercise real windows across single-monitor, mixed-DPI, taskbar, RDP,
  elevated, borderless, fullscreen, non-resizable, and minimum-size cases.
- [ ] Test snapping, stash persistence, exclusions, display changes, and named
  pipe commands after a restart.
- [ ] Confirm that the Winit helper event target does not appear in the taskbar
  or Alt+Tab while LoopW remains resident.
- [ ] Record the Windows version, display layout, DPI settings, and commit used
  for each manual pass.

This is the highest priority because native window behavior depends on the
desktop environment. Passing pure tests cannot prove that a hook, overlay, or
window action behaves correctly on every Windows setup.

### Make persistence failures visible

- [x] Write `settings.json` through a temporary file and replace the old file
  only after the write succeeds.
- [x] Return save results to the settings UI instead of treating persistence as
  best-effort and silent.
- [x] Keep the edited control value and the saved value in sync when Windows or
  the filesystem rejects a change (revert-on-failure via `settings::sync`,
  sticky `save_error` in the settings UI).
- [x] Add tests for invalid JSON, partial writes, and settings migration
  (`settings::load_report` + `settings::sync` suites).

### Add useful runtime diagnostics

- [x] Report hook installation failures, denied window access, failed frame
  changes, unavailable monitor data, and stale stash records in a user-visible
  diagnostics view or log (`win::diagnostics` ring buffer, Advanced section).
- [x] Explain safe no-ops for unsupported or excluded windows without exposing
  native error codes as the only message (friendly text + technical detail).
- [x] Add a small diagnostic path for reproducing IPC and settings issues
  (repro hints in the Diagnostics view; pipe-command listing deferred).

### Close the remaining UI quality gaps

- [ ] Verify contrast and focus visibility for dark, light, Windows-following,
  and custom accent settings.
- [ ] Honor the Windows reduced-motion preference for nonessential UI motion.
- [ ] Confirm that all section resets leave unrelated settings and external
  windows untouched.
- [ ] Keep the active radial overlay text-free while settings assignments and
  previews change.

## Later release work

- [ ] Decide whether the project needs an installer, code signing, and an update
  path in addition to the existing self-contained ZIP workflow.
- [ ] Define a supported Windows version range and document known limitations
  for elevated, protected, UWP/WinUI, console, and exclusive-fullscreen apps.
- [ ] Add a release checklist that includes a clean-machine install, upgrade,
  uninstall, settings preservation, and rollback checks.

## Release gate

The current automated gate is met: 155 tests pass, formatting is clean, the
locked build passes, and warning-denied Clippy passes. The broader release gate
is not met because the desktop checklist has not been run.

LoopW is ready for a broader release when:

1. Formatting, the locked build, the all-targets test suite, and
   warning-denied Clippy pass.
2. The desktop checklist passes on the supported Windows and display setups.
   Each exception must have a documented reason.
3. Trigger, radial, keybind, snapping, stash, monitor, exclusion, tray, and IPC
   behavior are consistent across their supported entry points.
4. Existing settings files load without losing values, and save failures are
   visible and recoverable.
5. Unsupported windows fail safely and tell the user what happened.

Packaging is a separate decision after this gate. The repository currently has
one GitHub Actions workflow, `publish.yml`; it runs when manually dispatched or
when a `v*` tag is pushed. It checks formatting, tests, and warning-denied
Clippy, builds the release EXE, and uploads the ZIP archive and checksum. Tag
runs additionally create a GitHub Release. There is currently no workflow that
runs automatically on every pull request.

## Development commands

Run these commands from the repository root in PowerShell:

```powershell
cargo fmt -- --check
cargo build --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run
cargo build --release --locked
```

The release binary is `target\release\LoopW.exe`.
