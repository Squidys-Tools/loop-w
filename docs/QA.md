# LoopW manual QA checklist

Run this checklist with the built Windows executable. Record Windows edition and
build, LoopW commit/version, monitor layout, taskbar placement, DPI scale, and
whether the session is local or RDP. Use disposable test windows and save work
before testing minimize, hide, stash, or focus actions.

Status: `[ ]` not run, `[x]` passed, `[!]` blocked or not supported on this
Windows configuration.

The current automated baseline is a warning-free build and 39 pure tests. Live
named-pipe responses and desktop actions still require the manual checks below.
Re-run the automated baseline after every action-model change.

## 1. Build and resident lifecycle

- [ ] Build `LoopW.csproj` with zero warnings and run all `LoopW.Tests` tests.
- [ ] Launch LoopW. The main window stays hidden and exactly one tray icon appears.
- [x] Launch LoopW a second time. The existing instance activates; no second hook,
  process, or tray icon remains.
- [x] Close the settings window. It hides to the tray instead of exiting.
- [ ] Choose Quit from the tray. The process and icon disappear and hooks are
  removed.
- [ ] Enable launch at login, sign out, and sign back in. LoopW starts hidden.
- [ ] Kill and relaunch LoopW after an active radial menu, a pending preview, and
  a stashed window. No stuck overlay, hook, or stale tray state remains.

## 2. Trigger and input behavior

- [ ] Hold the default trigger. The radial menu opens at the cursor and the
  trigger does not leak to the active application.
- [ ] Rebind the trigger, including modifier combinations. Esc cancels capture;
  invalid or conflicting combinations are rejected.
- [ ] Release the trigger in each state: center, a sector, after moving across
  sectors, and after moving outside the overlay. State resets cleanly.
- [ ] If enabled, test trigger delay, release timeout, and double-click trigger.
  Verify a normal key press is unaffected when no gesture is recognized.
- [ ] If enabled, test middle-mouse activation in a text editor, browser, and
  file manager. Normal middle-click behavior remains intact when inactive.
- [ ] If enabled, test left/right modifier distinction on a keyboard that exposes
  both sides. The configured side alone activates.
- [ ] Add a trigger-bypass keybind and verify it fires once, does not conflict
  with normal typing, and is removed cleanly.
- [ ] Unplug/reconnect the keyboard or mouse during an active gesture. LoopW
  cancels safely and the input hook remains usable.

## 3. Radial menu and preview

- [ ] All eight default sectors select the expected cardinal half or diagonal
  quarter on a normal resizable window.
- [ ] Move beyond the visible radial ring in a wedge direction and release. The
  directional action commits without requiring the pointer to hover the painted
  wedge.
- [ ] Turn off cursor direction selection. Hover a visible wedge and release;
  the hovered action still commits and the overlay closes normally.
- [ ] Move into the center hole. Selection clears and the preview disappears.
- [ ] Commit by trigger release, left click, and arrow key. Each path applies the
  same target frame exactly once.
- [ ] Press Escape. The overlay and preview close without moving the window.
- [ ] Confirm the active radial overlay contains no text at any DPI or theme.
- [ ] Repeat each cardinal action. It cycles half, third, two-thirds; quarter
  actions do not cycle unless explicitly configured as cycles.
- [ ] Disable the radial menu. Keybinds and IPC actions still work.
- [ ] Change radial radius, preview padding, corner radius, border, colors, and
  appearance mode. Changes apply without restarting the resident process.
- [ ] Configure a custom radial slot and center action. The settings labels may
  describe actions, but the active overlay remains geometry-only.
- [ ] Test a preview over a dark, light, animated, and high-contrast window. It
  does not steal focus or leave an orphaned overlay after cancel.

## 4. Window action library

Use Notepad, Explorer, Calculator, a browser, and a window with a large minimum
size. For every action, verify the target is the foreground window and that LoopW
does not target its own settings, radial, preview, or tray surfaces.

- [ ] Halves, quarters, thirds, two-thirds, center, almost maximize, maximize,
  fullscreen, hide, minimize, restore initial frame, and undo.
- [ ] Maximize height and maximize width.
- [ ] Horizontal/vertical center halves, horizontal fourths, and three-fourths
  layouts, if exposed in this build.
- [ ] Larger/smaller, proportional scale up/down, grow/shrink per edge, and
  grow/shrink horizontally/vertically.
- [ ] Fill available space around a neighboring window. Verify the documented
  fallback when no neighbor is usable.
- [ ] Minimize others. Verify LoopW, the target, hidden windows, owned popups,
  and excluded applications are not incorrectly minimized.
- [ ] Focus nearest up/down/left/right and next in z-order. Focus never lands on
  an overlay, tool window, or excluded process.
- [ ] Apply actions to maximized, minimized, non-resizable, borderless, and
  large-minimum-size windows. Results stay in the work area or report a clear
  no-op reason.
- [ ] Test a window spanning two monitors. The selected monitor and resulting
  frame are deterministic.

## 5. Drag snapping

- [ ] Drag a normal window to each monitor edge and corner. The target preview
  appears at the configured threshold and the committed frame is correct.
- [ ] Cancel a drag after the preview appears. The original frame is restored if
  that setting is enabled.
- [ ] Resize while moving near an edge. LoopW does not fight native resize or
  Windows Snap Assist.
- [ ] Drag a maximized, fullscreen, minimized, elevated, excluded, and
  non-resizable window. Each follows its documented policy.
- [ ] Change monitors or DPI during a drag. No stale preview or off-screen frame
  remains.

## 6. Screens, DPI, and work areas

- [ ] On one 100% monitor, test the taskbar on the left, right, top, and bottom.
- [ ] Test mixed-DPI monitors at 100%/125% and 100%/150%, including monitors
  above, below, and offset from the primary display.
- [ ] Test next, previous, left, right, above, and below screen actions. If no
  directional monitor exists, the action is a no-op with a useful diagnostic.
- [ ] Move a window between monitors and confirm pixel/logical size behavior is
  consistent with the selected action.
- [ ] Change display scale, resolution, taskbar mode, and auto-hide while LoopW
  is running. Work areas and previews refresh without a restart.
- [ ] Disconnect and reconnect a display, then use the radial menu and keybinds.
- [ ] Repeat the supported checks in an RDP session and record any expected
  limitations.

## 7. Stash and identity recovery

- [ ] Stash a window at the left, right, top, and bottom edges. Each leaves the
  configured visible strip.
- [ ] Hover the strip and reveal the window. It returns to the original frame,
  monitor, DPI, and maximized/minimized state.
- [ ] Stash multiple windows and reveal them in order with the keybind.
- [ ] Close a stashed window externally. Polling removes the entry without an
  exception or stuck edge strip.
- [ ] Restart LoopW with stash persistence enabled. Only an unambiguous matching
  window is restored; a reused HWND is never trusted by itself.
- [ ] Change a stashed window's title or process state before restart. Confirm
  ambiguous metadata does not move an unrelated window.

## 8. Settings, persistence, and recovery

- [ ] Navigate Behavior, Radial, Preview, Appearance, and Advanced with mouse and
  keyboard. Focus indicators and keyboard navigation remain usable.
- [ ] Add, rebind, cycle, bypass, and delete keybinds. Duplicate combinations are
  rejected inline and do not reach the runtime hook.
- [ ] Add/remove radial actions and reset one section. Unrelated settings remain.
- [ ] Change trigger, timing, mouse activation, radial layout, preview, theme,
  padding, exclusion list, and stash settings; restart; confirm persistence.
- [ ] Enter invalid JSON, an invalid color, an out-of-range number, and an
  unknown action reference. Startup falls back safely and preserves a usable
  configuration.
- [ ] Interrupt or terminate during a settings save. The next launch either uses
  the previous complete file or a valid new file.
- [ ] Reset all settings with confirmation. Defaults are restored and the hook
  is rebound without a second process.
- [ ] Confirm the settings window itself is never moved, hidden, focused, or
  minimized by a target-window action.

## 9. IPC and CLI

- [ ] Run `LoopW.exe list/actions` and confirm every exposed action is listed.
- [ ] Run `LoopW.exe list/keybinds` and confirm the trigger, timing settings, and
  configured binds are returned without secrets or stale entries.
- [x] Focus an external window, run `LoopW.exe direction/right`, and confirm the
  exact right-half frame.
- [ ] Run each named action through `action/<name>` and compare it with the
  radial/keybind result.
- [ ] Run a malformed command. It returns `ERROR:` without terminating LoopW.
- [ ] Start LoopW with a command while it is not running. It starts resident and
  executes the command once.
- [ ] Invoke commands repeatedly while resident. There is still one process,
  one pipe server, one hook, and one tray icon.
- [ ] Attempt a pipe connection from another user account. It is rejected.

## 10. Security and unsupported-window checks

- [ ] Test a standard-user LoopW against a standard-user app and an elevated
  app. Record the expected UIPI/access result; no unrelated window moves.
- [ ] Test UWP/WinUI, protected, exclusive-fullscreen, console, and tool windows.
  Unsupported targets produce a safe no-op or clear diagnostic.
- [ ] Add an executable to the exclusion list. Radial, keybind, focus, fill,
  minimize-others, snapping, stash, and IPC all honor it.
- [ ] Verify hooks are removed on Quit and process exit. No stuck keyboard or
  mouse suppression remains.
- [ ] Verify no window title, keystroke, or unrelated application data is sent
  outside the local process. IPC remains same-user named-pipe traffic.

## 11. Final gate

- [ ] Run the automated build and test commands from this document's companion
  implementation plan.
- [ ] Complete sections 1 through 10 on at least one local multi-monitor setup.
- [ ] Complete mixed-DPI, RDP, elevated-app, and taskbar-layout checks or record
  explicit product limitations.
- [ ] Update the implementation plan with the verified status and any remaining
  Windows-specific limitations.

Packaging, signing, installer choice, update channels, and distribution are not
part of this port QA gate. They will be tested in a separate shipping plan.

