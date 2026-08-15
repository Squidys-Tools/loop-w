# LoopW — Implementation Progress

Window management made elegant, for Windows. A feature-by-feature port of
[Loop (macOS)](https://github.com/mrkai77/loop) to a native Win32/WPF app.

**Stack:** .NET 8 (net8.0-windows) · WPF · Win32 interop (user32 / dwmapi / shcore / gdi32)

**Status legend:** `[ ]` not started · `[~]` in progress · `[x]` done

---

## Core interaction (the foundation)

- [x] Global trigger key (hold-to-activate) via `WH_KEYBOARD_LL` hook — swallows the
      key system-wide so Caps Lock never toggles while bound. `GlobalHotkey.cs`
- [x] Rebinding — click the trigger chip or "rebind" in the main window; Esc cancels;
      Win-combos rejected; persisted to `%LOCALAPPDATA%\LoopW\settings.json`. `Settings.cs`
- [x] Radial menu rendered at the cursor on trigger press. `RadialOverlayWindow.xaml(.cs)`
- [x] Wedge selection by cursor direction (polled 20 ms so the transparent center hole
      can clear a selection). Cardinal halves and diagonal quarters.
- [x] Live preview of the target frame (blurred backdrop, rounded, animated). `PreviewOverlayWindow.xaml(.cs)`
- [x] Commit on trigger release / left-click / arrow keys; Esc cancels.
- [x] Multi-DPI aware (per-monitor DPI via `shcore.dll`).
- [x] Window placement hardening: restores maximized/minimized, clamps to min/max
      track sizes, re-anchors to the zone edge so frames never overflow the screen,
      tolerates unresponsive apps. `WindowActionService.cs`

---

## 1. Action library

The generic action library contains ~40 actions. The radial surface now exposes
the four cardinal halves and four diagonal quarters; the remaining actions stay
available through keybinds.

- [x] Refactor to a generic `WindowAction` set: halves, quarters, thirds, center,
      maximize, almost-maximize, fullscreen, minimize, hide.
- [x] Screen switching: next / previous / left / right / top / bottom (multi-monitor).
- [x] Window manipulation: larger / smaller, grow & shrink per edge, nudge per edge.
- [x] Initial Frame — record a window's bounds on its first action, restore on demand.
- [x] Undo — stack of recent placements per window.

Notes: `WindowActionService.cs` exposes `TryApply(window, WindowAction, out msg)`
and `TryGetTargetFrame(...)`. The radial overlay uses those generic APIs directly;
the half wrappers remain for compatibility with older callers.
`FitFrame` clamps to min/max track sizes and re-anchors to zone edges (halves, thirds,
etc.) or re-centers (center, almost-maximize, center-thirds). Keybinds/settings
continue to expose actions that do not fit the radial surface.

---

## 2. Keyboard shortcuts (keybinds)

Loop: trigger + any key → immediate action, fully configurable.

- [x] Keybind registry (key → action, persisted like trigger binding). `Keybind.cs` +
      `AppSettings.Keybinds` (`settings.json`).
- [x] Capture flow to record new binds (reuse `GlobalHotkey.BeginCapture` UX; added a
      callback overload so the settings window captures without tripping trigger-rebind).
- [x] Apply action instantly on trigger+key (no radial menu) — `KeybindPressed` event
      fires while the trigger is held and the key is swallowed system-wide.
- [x] Keybinds UI (list, add, rebind, delete) — placeholder `SettingsWindow` with a
      keybinds tab only (full settings in #5). Opened via the "settings" link in the
      main window footer.

---

## 3. Cycles

Repeat the same trigger+key (or left-click) to step through action variations.

- [x] Cycle chains for left/right/top/bottom sizing (half → third → two-thirds → cycle).
- [x] Repeat detection for repeated trigger + key presses, with keyboard auto-repeat suppressed.
- [x] Configurable per keybind with a Cycle toggle; radial directional commits cycle by default.

Notes: Cycle.cs keeps the last successful position per target and requested action.
The first use keeps the configured action, then repeated uses advance through its
directional chain. A failed window placement does not consume a step.

---

## 4. Stash

Hide windows at the screen edge to declutter; reveal on hover or keybind.

- [x] Stash a target window off the closest screen edge with a visible edge peek.
- [x] Edge hit-zone: hover near edge reveals stashed windows (like a dock).
- [x] Keybind to reveal the next stashed window.
- [x] Per-window stash state + placement restore (pair with Initial Frame).

---

## 5. Settings UI

Loop has a Settings window with tabs (Behavior / Radial / Preview / Keybinds / Theming).
The settings window now exposes the stored behavior, radial, preview, keybind,
and theme options.

- [x] Settings window with tabbed layout.
- [x] Behavior: trigger key and launch at login. Run-in-tray stays with resident-app plumbing (#7).
- [x] Radial: enabled toggle, cursor-interaction toggle, radius controls, and colors.
- [x] Preview: padding, corner radius, border color, border width, enabled toggle.
- [x] Keybinds tab (wires into #2).
- [x] Persistence model for all of the above (extend `Settings.cs`).

---

## 6. Theming

- [x] Radial menu colors (accent, ring, sector fill/stroke).
- [x] Radial geometry controls (outer/inner radius) — `RadialGeometry.cs` receives
      settings input.
- [x] Toggle radial menu off entirely (keybinds still work).
- [x] Preview theming per #5.

---

## 7. Resident app plumbing

To be a real background utility (not a window you keep open):

- [x] System tray icon with menu (Open LoopW, Open Settings, Quit).
- [x] Single-instance guard (second launch signals and focuses the existing instance).
- [x] Launch at login (registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- [x] Run without showing the main window (start hidden in the tray on boot).

---

## 8. Scripting / IPC

Loop's `loop://` URL scheme. Windows equivalent:

- [x] Same-user named pipe server (`LoopW-Commands`) accepting one-line commands
      from the CLI.
- [x] CLI mode: `LoopW.exe direction/right`, `list/all`, `list/actions`, and
      `list/keybinds`, with `action/<name>` available for the full action library.
- [x] Same action library exposed through typed command parsing and the existing
      `WindowActionService`.

---

## 9. Packaging

- [ ] App icon (`.ico`), manifest metadata (already have `app.manifest`).
- [ ] Single-file publish profile (`dotnet publish -r win-x64 -p:PublishSingleFile=true`).
- [ ] Installer (e.g. Inno Setup / WiX / MSIX).
- [ ] Self-updater (Loop ships one; can defer).

---

## 10. Tests / verification

- [x] Pure unit runner for frame math and radial geometry in `LoopW.Tests`.
- [x] Manual QA checklist for lifecycle, radial actions, window states, DPI scaling,
      multi-monitor behavior, and RDP sessions in `QA.md`.

---

## Suggested build order

1. Packaging (#9), including a real app icon, publish profile, and installer.
2. Run the desktop QA checklist in `QA.md` across the supported Windows setups.

---

## Build / run

```pwsh
# NOTE: the user PATH already contains ~\.dotnet, but the machine PATH's
# runtime-only C:\Program Files\dotnet\ resolves first and has no SDK.
# Call the home SDK explicitly (SDK 8.0.424) unless that precedence is fixed.
$dotnet = "$HOME\.dotnet\dotnet.exe"
& $dotnet build LoopW.csproj
& $dotnet run --project LoopW.csproj
& $dotnet run --project LoopW.Tests/LoopW.Tests.csproj
& $dotnet publish LoopW.csproj -c Release -r win-x64 --self-contained -p:PublishSingleFile=true
```

## Session log

Track decisions and progress here as work happens.

- **2026-08-14** — Gap analysis vs upstream Loop completed. Doc created. Core
  interaction (trigger, radial, preview, halves) confirmed working.
- **2026-08-14** — Feature #1: generic action library landed in `WindowActionService.cs`.
  `WindowAction` enum (~45 actions) + `TryApply` + `TryGetTargetFrame`. Added halves,
  quarters, thirds, center/maximize/almost-maximize/fullscreen, minimize/hide,
  screen switching (next/prev/directional), manipulation (larger/smaller,
  grow/shrink/move per edge), Initial Frame restore, and a 20-deep undo stack.
  `ApplyHalf` kept as wrapper for the radial UI. Builds clean with `~\.dotnet`.
  Not yet wired into the UI (awaits keybinds/settings).
- **2026-08-14** — Feature #2: keybindings. `Keybind.cs` model persisted in
  `AppSettings.Keybinds`. `GlobalHotkey` gains `SetKeybinds` + `KeybindPressed`
  (fires while trigger is held, key swallowed) + a callback-based `BeginCapture`
  overload for window-safe capture. Placeholder `SettingsWindow` (keybinds editor:
  list/add/rebind/delete with action dropdown) opened via "settings" link in main
  window. `MainWindow.Hotkey_KeybindPressed` applies the action to the captured
  target. App launches clean; smoke-tested.
- **2026-08-15** — Feature #5: replaced the keybind-only screen with Behavior,
  Radial, Preview, Keybinds, and Theme tabs. Added validated persistence for the
  new settings, launch-at-login registration, live visual updates, and radial/
  preview options.
- **2026-08-15** — Feature #7: added a resident app lifecycle with a system tray,
  single-instance activation, hidden startup, and explicit tray quit. Main-window
  close now hides to the tray, while tray activation restores the window or opens
  settings. The lifecycle smoke test confirmed that a second launch leaves one
  LoopW process running.
- **2026-08-15** — Radial action surface: expanded the overlay and main preview to
  eight spatial sectors. Cardinal sectors apply halves, diagonal sectors apply
  quarters, and the live preview now calls the generic `WindowAction` frame API.
  Added `.gitignore` rules and removed tracked `bin/` and `obj/` build outputs from
  the repository index.
- **2026-08-15** — Verification: extracted pure frame math into
  `WindowFrameMath.cs` and added an eight-case test runner under `LoopW.Tests`.
  All 8 tests pass. Added `QA.md` for the Windows-only lifecycle, display, window
  state, and settings checks that need a real desktop session.
- **2026-08-15** — Feature #4: added edge stash state with nearest-edge placement,
  visible peeking, edge-hover reveal polling, and a Reveal stashed action that
  works from a keybind without a captured target window.
- **2026-08-15** — Automated QA: main build passed with zero warnings, all 9 pure
  tests passed, and the published executable passed the single-instance smoke
  test. The remaining QA checklist items need interactive desktop verification.
- **2026-08-15** — Feature #8: added typed named-pipe commands for direction and
  full action execution, action/keybind/all listings, same-user CLI routing to
  the resident process, and startup-command execution when no instance exists.
  Parser coverage now brings the pure test runner to 15 tests; a live pipe
  request returned the configured trigger successfully. Second-launch activation
  now uses the same resident pipe, with the named event retained as fallback.
- **2026-08-15** — Desktop-level smoke verification used a real Notepad HWND:
  `direction/right` moved it to the exact right-half frame. Lifecycle checks
  confirmed hidden startup, second-launch activation, close-to-tray hiding, and
  one remaining resident process. Visual tray and radial interaction remain for
  a connected desktop session.
