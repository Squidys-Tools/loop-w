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
      can clear a selection). N/S/E/W halves only.
- [x] Live preview of the target frame (blurred backdrop, rounded, animated). `PreviewOverlayWindow.xaml(.cs)`
- [x] Commit on trigger release / left-click / arrow keys; Esc cancels.
- [x] Multi-DPI aware (per-monitor DPI via `shcore.dll`).
- [x] Window placement hardening: restores maximized/minimized, clamps to min/max
      track sizes, re-anchors to the zone edge so frames never overflow the screen,
      tolerates unresponsive apps. `WindowActionService.cs`

---

## 1. Action library

Today only **halves** exist (`WindowActionService.ApplyHalf` / `WindowHalf`). Loop has
~40 actions. Plan:

- [x] Refactor to a generic `WindowAction` set: halves, quarters, thirds, center,
      maximize, almost-maximize, fullscreen, minimize, hide.
- [x] Screen switching: next / previous / left / right / top / bottom (multi-monitor).
- [x] Window manipulation: larger / smaller, grow & shrink per edge, nudge per edge.
- [x] Initial Frame — record a window's bounds on its first action, restore on demand.
- [x] Undo — stack of recent placements per window.

Notes: `WindowActionService.cs` now exposes `TryApply(window, WindowAction, out msg)`
and `TryGetTargetFrame(...)` (used by the preview). `ApplyHalf` / `TryGetHalfFrame`
remain as thin wrappers so the radial overlay and preview keep working unchanged.
`FitFrame` clamps to min/max track sizes and re-anchors to zone edges (halves, thirds,
etc.) or re-centers (center, almost-maximize, center-thirds). Not yet surfaced in UI —
keybinds/settings (#2/#5) will drive it.

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

- [ ] Cycle chains (e.g. right-half → right-third → right-two-thirds → cycle).
- [ ] Repeat detection on trigger re-press without full release (and with).
- [ ] Configurable per keybind; opt-out per action.

---

## 4. Stash

Hide windows at the screen edge to declutter; reveal on hover or keybind.

- [ ] Stash a target window (animate off the closest screen edge).
- [ ] Edge hit-zone: hover near edge reveals stashed windows (like a dock).
- [ ] Keybind to cycle/revive stashed windows.
- [ ] Per-window stash state + bounds restore (pair with Initial Frame).

---

## 5. Settings UI

Loop has a Settings window with tabs (Behavior / Radial / Preview / Keybinds / Theming).
Currently the only screen is the demo radial panel + trigger chip.

- [ ] Settings window with tabbed layout.
- [ ] Behavior: trigger key, launch at login, run in tray.
- [ ] Radial: enabled toggle, cursor-interaction toggle, width/shape/colors.
- [ ] Preview: padding, corner radius, border color, border width, enabled toggle.
- [ ] Keybinds tab (wires into #2).
- [ ] Persistence model for all of the above (extend `Settings.cs`).

---

## 6. Theming

- [ ] Radial menu colors (accent, ring, sector fill/stroke).
- [ ] Radial geometry controls (outer/inner radius, gap/width) — `RadialGeometry.cs` is
      parameterized already, just needs settings input.
- [ ] Toggle radial menu off entirely (keybinds still work).
- [ ] Preview theming per #5.

---

## 7. Resident app plumbing

To be a real background utility (not a window you keep open):

- [ ] System tray icon with menu (Open Settings, Quit).
- [ ] Single-instance guard (second launch focuses existing instance / tray).
- [ ] Launch at login (registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- [ ] Run without showing the main window (start minimized to tray on boot).

---

## 8. Scripting / IPC

Loop's `loop://` URL scheme. Windows equivalent:

- [ ] Named pipe server (e.g. `loopw-<user>`) accepting commands.
- [ ] CLI mode: `loopw.exe direction/right`, `list/all`, `list/actions`, `list/keybinds`.
- [ ] Same action library exposed (reuses #1).

---

## 9. Packaging

- [ ] App icon (`.ico`), manifest metadata (already have `app.manifest`).
- [ ] Single-file publish profile (`dotnet publish -r win-x64 -p:PublishSingleFile=true`).
- [ ] Installer (e.g. Inno Setup / WiX / MSIX).
- [ ] Self-updater (Loop ships one; can defer).

---

## 10. Tests / verification

- [ ] Unit tests for frame math (`WindowActionService`, geometry) with a fake window.
- [ ] Manual QA checklist for DPI scaling, multi-monitor, elevated apps, RDP sessions.

---

## Suggested build order

1. Generic action library (#1) — everything else depends on it.
2. Keybindings engine (#2) on top of the action library.
3. Cycles (#3) — reuse the repeat detection from keybinds.
4. Settings window (#5) to surface #1–#3 and hold theming (#6).
5. Tray/autostart/single-instance (#7) to make it a resident utility.
6. Stash (#4).
7. Scripting/IPC (#8), then packaging (#9).

---

## Build / run

```pwsh
# NOTE: the user PATH already contains ~\.dotnet, but the machine PATH's
# runtime-only C:\Program Files\dotnet\ resolves first and has no SDK.
# Call the home SDK explicitly (SDK 8.0.424) unless that precedence is fixed.
$dotnet = "$HOME\.dotnet\dotnet.exe"
& $dotnet build LoopW.csproj
& $dotnet run --project LoopW.csproj
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
