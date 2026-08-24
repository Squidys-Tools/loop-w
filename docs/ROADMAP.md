# LoopW roadmap

LoopW is a Windows window manager built around a hold-to-open radial menu. The
main product path is in the code now. This roadmap tracks the work that still
needs to be verified or hardened before a wider release.

LoopW targets .NET 8, WPF, and native Windows APIs. It runs as a tray resident
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
- A same-user named-pipe command server supports activation, action listing,
  keybind listing, directional actions, and named actions.
- Pure tests cover frame math, radial geometry, cycles, navigation, settings
  normalization, stash calculations, drag snapping, and command parsing.

The design document for the settings surface remains in
[`specs/settings-ui-redesign-spec.md`](../specs/settings-ui-redesign-spec.md).
It describes the intended behavior and the manual checks that still need to be
run. [`QA.md`](QA.md) is the desktop test checklist.

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
- [ ] Keep the edited control value and the saved value in sync when Windows or
  the filesystem rejects a change.
- [ ] Add tests for invalid JSON, partial writes, and settings migration.

### Add useful runtime diagnostics

- [ ] Report hook installation failures, denied window access, failed frame
  changes, unavailable monitor data, and stale stash records in a user-visible
  diagnostics view or log.
- [ ] Explain safe no-ops for unsupported or excluded windows without exposing
  native error codes as the only message.
- [ ] Add a small diagnostic path for reproducing IPC and settings issues.

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

LoopW is ready for a broader release when:

1. The automated build and pure test suite pass without warnings.
2. The desktop checklist passes on the supported Windows and display setups, or
   each exception has a documented reason.
3. Trigger, radial, keybind, snapping, stash, monitor, exclusion, tray, and IPC
   behavior are consistent across their supported entry points.
4. Existing settings files load without losing values, and save failures are
   visible and recoverable.
5. Unsupported windows fail safely and tell the user what happened.

Packaging is a separate decision after this gate. The current GitHub Actions
workflow builds a self-contained `win-x64` package for `v*` tags and stores the
archive and checksum for manual workflow runs.

## Development commands

Run these commands from the repository root in PowerShell:

```powershell
dotnet build LoopW.csproj
dotnet run --project LoopW.Tests/LoopW.Tests.csproj
dotnet run --project LoopW.csproj
dotnet publish LoopW.csproj -c Release -r win-x64 --self-contained -p:PublishSingleFile=true
```

The tests are a small executable rather than a test-framework project, so use
`dotnet run` for the test project.
