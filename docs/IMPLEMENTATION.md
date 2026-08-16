# LoopW implementation roadmap

LoopW is a Windows-native port of [Loop for macOS](https://github.com/MrKai77/Loop).
This document answers one question: what is already working, and what still
needs to be built before the Windows port is finished?

The app uses .NET 8, WPF, and native Windows APIs through `user32`, `dwmapi`,
`shcore`, and `gdi32`.

`[x]` means implemented. `[ ]` means remaining work.

## Already implemented

### Radial interaction

- [x] Global hold-to-activate keyboard trigger using `WH_KEYBOARD_LL`.
- [x] Trigger rebinding with persistence in `%LOCALAPPDATA%\\LoopW\\settings.json`.
- [x] Eight-sector radial menu at the cursor: four halves and four quarters.
- [x] Cursor-based selection with a center area that clears the selection.
- [x] Live target-frame preview.
- [x] Commit on trigger release, left click, or arrow key.
- [x] Escape to cancel.
- [x] Per-monitor DPI handling.
- [x] The active radial menu contains no text. This remains a permanent UI rule.

### Window actions

- [x] Halves, quarters, thirds, two-thirds, center, almost maximize, maximize,
  fullscreen, minimize, and hide.
- [x] Next, previous, left, right, above, and below monitor actions.
- [x] Larger, smaller, grow, shrink, and move actions for each edge.
- [x] Maximize height/width, center halves, horizontal fourths, and three-fourths.
- [x] Scale up/down and grow/shrink horizontally or vertically.
- [x] Fill available space around non-overlapping neighboring windows.
- [x] Minimize other eligible windows and focus the nearest window by direction
  or the next window in z-order.
- [x] Restore the initial frame and undo recent moves.
- [x] Restore maximized/minimized windows before positioning them.
- [x] Respect minimum and maximum window sizes.
- [x] Keep resulting frames inside the monitor work area.

### Keybinds, cycles, and stash

- [x] Persisted keybinds with add, rebind, delete, and duplicate detection.
- [x] Trigger-plus-key actions.
- [x] Directional cycles: half, third, then two-thirds.
- [x] Stash windows at the nearest screen edge with a visible peek.
- [x] Reveal stashed windows by hovering the edge or using a keybind.
- [x] Restore the stashed window's original placement during the current session.

### Settings and resident app

- [x] Behavior, Radial, Preview, Appearance, and Advanced settings.
- [x] Persisted trigger, keybind, radial, preview, and appearance settings.
- [x] Tray icon, hidden startup, close-to-tray, and single-instance behavior.
- [x] Launch at login through the current user's Windows Run key.

### IPC and verification

- [x] Same-user named-pipe server: `LoopW-Commands`.
- [x] CLI commands for listing actions, listing keybinds, directional actions,
  and named actions.
- [x] Pure tests for frame math, radial geometry, cycles, navigation, drag snap
  geometry, settings, and command
  parsing.
- [x] Last recorded verification: warning-free build and 31 passing tests. The
  live pipe request and real Notepad action remain previously recorded checks
  and should be rerun with the drag-snap QA pass.
- [ ] Complete the interactive checks in [`QA.md`](QA.md).

## Remaining work

Implement these in order. Every new action must be available through the same
action service used by radial selections, keybinds, and IPC.

### 1. Validate the Windows action library on real windows

- [x] Add the missing frame actions and expose them through the action catalog.
- [x] Add fill-available-space obstacle selection with a safe current-frame
  fallback.
- [x] Add minimize-others and directional/z-order focus actions.
- [x] Add parser and pure-geometry/navigation test coverage.
- [ ] Run the complete action matrix against real windows, including elevated,
  non-resizable, borderless, and multi-monitor windows.

Windows implementation:

- `SetWindowPos`, `GetWindowPlacement`, and the existing `WindowFrameMath` for
  frame changes.
- `EnumWindows`, `GetWindowRect`, and `GetWindowLongPtr` for finding eligible
  windows.
- `GetForegroundWindow`, `SetForegroundWindow`, and `ShowWindow` for focus and
  minimize behavior.
- `MonitorFromWindow` and `GetMonitorInfo` for monitor work areas.

Acceptance criteria:

- Every action appears in the action catalog and can be invoked by keybind and
  IPC before it is added to the radial settings UI. (Implemented.)
- Actions ignore LoopW's settings, preview, radial, and tray windows.
- Unsupported or restricted windows produce a safe no-op rather than moving a
  different window.

### 2. Finish trigger and keybind behavior

- [x] Add persisted activation delay and release-timeout settings.
- [x] Add optional double-click trigger activation.
- [x] Add optional middle-mouse activation through `WH_MOUSE_LL`.
- [x] Support left/right modifier distinction where the keyboard exposes it.
- [x] Support explicitly configured keybinds that do not require the radial
  trigger.
- [x] Cancel cleanly when the trigger, mouse source, capture flow, or settings
  change interrupts an active gesture.

Implemented Windows design:

- The existing `WH_KEYBOARD_LL` state machine now tracks keyboard and mouse
  trigger sources, delayed activation, timeout, double-click arming, and
  cancellation versions so stale timer callbacks cannot reopen a gesture.
- `WH_MOUSE_LL` is installed only while middle-mouse activation is enabled and
  the keyboard hook is active.
- Trigger settings and bypass-keybind flags persist through `AppSettings`; the
  settings UI exposes all of them and the command listing identifies bypass
  bindings.
- The hook consumes only the configured trigger and matching keybind gestures.
  Manual QA must still verify normal typing, middle-click behavior when the
  option is disabled, left/right modifier hardware, and keyboard reconnects.

### 3. Make the radial menu configurable

- [x] Replace the fixed action assignments with eight persisted radial slots.
- [x] Allow each slot to reference a built-in action, a cycle, or a keybind.
- [x] Make the center action configurable, including no-op, maximize, center, or
  a selected keybind.
- [x] Add settings controls and a live main-window preview for assigning slots.
- [x] Keep release, click, arrow-key, preview, and cancel behavior on one target
  resolution path.

Implementation rules:

- Keep `RadialGeometry` pure and data-independent.
- Keep `RadialActionCatalog` responsible for fixed geometry and load validated
  target data through the normalized settings boundary.
- Replace missing or invalid saved actions with a safe no-op.
- Labels can appear in Settings, but never in the active radial overlay.

Implemented Windows design:

- `RadialTargetSettings` persists action, cycle, keybind, and no-op targets.
- Keybind targets use stable IDs, so deleting or reordering keybind rows cannot
  redirect a radial slot to another binding.
- The active overlay resolves wedge and center targets once, then sends the same
  typed target through release, click, arrow-key, preview, and commit handling.
- The visible main-window ring reflects the saved assignments. The active
  overlay remains geometry-only and contains no labels.

### 4. Add drag snapping

- [x] Snap a title-bar dragged window to screen edges and supported half/quarter zones.
- [x] Show the target frame before committing the snap.
- [x] Add a configurable snap threshold.
- [x] Optionally restore the pre-drag frame when a snap is canceled.
- [x] Handle capture loss, canceled moves, monitor changes, fullscreen windows,
  and applications that reject the requested frame.
- [x] Do not interfere with Windows Snap Assist or snap LoopW's own windows.

Windows implementation:

- `DragSnapService` observes a narrowly scoped `WH_MOUSE_LL` title-bar drag
  lifecycle without swallowing input or taking mouse capture. It polls the
  left-button state so capture loss and canceled moves close the gesture.
- `DragSnapGeometry` resolves half and quarter zones from the physical monitor
  edge while `WindowFrameMath` computes the work-area frame.
- The existing `PreviewOverlayWindow` renders the resolved frame before release.
- `WindowActionService.TryApplySnap` restores and places the frame on the
  cursor's monitor, reports min/max-size or placement rejection, and records a
  successful snap in undo history.
- Fullscreen, maximized, borderless, tool, owned, hidden, and LoopW windows are
  ignored at the drag boundary. Mouse messages continue to Windows so native
  resize and Snap Assist behavior remains available.

### 5. Harden stash and window identity

- [x] Remove stash entries when a window is destroyed or its process exits.
- [x] Restore maximized/minimized state and original monitor/DPI information.
- [x] Add configurable edge peek size, hit-zone size, and reveal delay.
- [x] Persist stash metadata across restart using executable path, process ID,
  window class, and title as matching hints.
- [x] Restore only an unambiguous match. Never trust a reused HWND by itself.

Stash records are stored with the application settings. Cross-restart restoration
requires the executable path plus at least two independent metadata hints, and
never treats a persisted HWND as an identity.

### 6. Finish monitor, DPI, and app policies

- [ ] Recompute monitor work areas after display, DPI, taskbar, and auto-hide
  changes.
- [ ] Define whether monitor moves preserve pixel size or logical size, then
  verify that behavior on mixed-DPI displays.
- [ ] Add global screen padding and per-edge padding.
- [ ] Add an exclusion list by executable path and process name.
- [ ] Apply exclusions consistently to actions, focus, fill, minimize-others,
  snapping, stash, and IPC.
- [ ] Define behavior for fullscreen, borderless fullscreen, UWP/WinUI, elevated,
  protected, and non-resizable windows.
- [ ] Show a useful diagnostic when Windows denies access or a window cannot be
  resized or moved.

Windows implementation:

- `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, `GetDpiForWindow`,
  `MonitorFromWindow`, and `GetMonitorInfo` for display changes.
- `DwmGetWindowAttribute` for reliable visible bounds and window state.
- Process-path lookup for the exclusion list.

### 7. Improve recovery and diagnostics

- [ ] Show hook installation failures, denied access, failed frame applications,
  stale stash records, and unavailable monitor data in a diagnostic view.
- [ ] Make settings writes atomic so an interrupted save cannot leave invalid
  JSON.
- [ ] Add tests for every exposed action, every action parser name, settings
  normalization, and invalid saved radial references.
- [ ] Add a desktop integration pass using Notepad, Explorer, Calculator, a
  non-resizable window, an elevated app, and a multi-monitor window.

## Definition of done for the port

The Windows port is ready for packaging work when:

1. The remaining action, input, radial, snapping, stash, monitor, and recovery
   items above are implemented or intentionally removed from the product scope.
2. Every exposed action works through the action service, keybinds, and IPC.
3. The active radial overlay remains text-free and does not leak input or steal
   focus.
4. The desktop QA checklist passes on a local multi-monitor setup, including
   mixed DPI, taskbar positions, elevated apps, and RDP limitations.
5. The build is warning-free and the automated test suite passes.

Packaging, signing, installer choice, update channels, and distribution are a
separate project after this definition of done is met.

## Build and run

```powershell
$dotnet = "$env:USERPROFILE\.dotnet\dotnet.exe"
& $dotnet build LoopW.csproj
& $dotnet run --project LoopW.csproj
& $dotnet run --project LoopW.Tests/LoopW.Tests.csproj
& $dotnet publish LoopW.csproj -c Release -r win-x64 --self-contained -p:PublishSingleFile=true
```

Run the desktop checks in [`QA.md`](QA.md) on a test machine where global
keyboard and mouse hooks are acceptable.
