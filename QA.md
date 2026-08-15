# LoopW manual QA checklist

Run this checklist on a Windows machine with the built executable. Record the
Windows version, display layout, DPI scale, and app version before starting.

Automated checks already run: the main build passes with zero warnings, all 15
pure tests pass, the named-pipe server answered live listing and action
requests, a real Notepad window moved to the exact right-half frame, and the
published executable passed hidden startup, second-launch activation, close-to-
tray, and single-instance checks. The remaining boxes require visual tray and
radial-menu interaction.

## Resident lifecycle

- [ ] Launch the app. The main window stays hidden and a LoopW tray icon appears.
- [ ] Open LoopW from the tray. The main window appears and can be hidden again.
- [ ] Open settings from the tray. Settings opens even while the main window is hidden.
- [x] Start LoopW a second time. The existing instance is activated and no second tray icon or hook appears.
- [x] Close the main window with the title-bar close button. It hides to the tray instead of exiting.
- [ ] Choose Quit from the tray. The process and tray icon disappear.
- [ ] Enable launch at login, sign out, and sign back in. LoopW starts hidden in the tray.

## Radial actions

- [ ] Capture a normal window and verify all eight sectors select the expected half or quarter.
- [ ] Move into the center hole. Selection clears and the preview disappears.
- [ ] Release the trigger on each cardinal sector. The window lands in the correct half.
- [ ] Release the trigger on each diagonal sector. The window lands in the correct quarter.
- [ ] Click a selected sector and verify it commits immediately.
- [ ] Press the arrow keys while the radial menu is open. Each key commits its cardinal half.
- [ ] Press Escape. The radial menu closes without moving the window.
- [ ] Repeat a cardinal action. It cycles half, third, and two-thirds. Quarter actions do not cycle.
- [ ] Disable the radial menu in settings. Keybinds still apply actions directly.

## Window states and constraints

- [ ] Apply an action to a maximized window. It restores and lands in the requested frame.
- [ ] Apply an action to a minimized window. It restores and lands in the requested frame.
- [ ] Test a non-resizable window. The app reports the size constraint without overflowing the monitor.
- [ ] Test a window with a large minimum size. The frame stays on-screen and remains anchored to its zone edge.
- [ ] Test Undo and Restore original frame after several moves.
- [ ] Test Hide and Minimize, then recover the window through the normal Windows UI.

## Displays and DPI

- [ ] Test one 100% DPI monitor with the taskbar on each edge.
- [ ] Test mixed DPI monitors, including a layout with one monitor above or offset from another.
- [ ] Test next, previous, left, right, above, and below screen actions.
- [ ] Test a directional screen action where no monitor exists. It should report a no-op.
- [ ] Disconnect or reconnect a display while LoopW is running, then use the radial menu again.
- [ ] Test an RDP session if the product will be used over Remote Desktop.

## Stash

- [ ] Apply Stash to a window near the left, right, top, and bottom edges. It leaves a small visible strip.
- [ ] Move the pointer into the visible edge strip. The stashed window returns to its original frame.
- [ ] Stash multiple windows and use a Reveal stashed keybind. Windows reveal in stash order.
- [ ] Close a stashed window externally. Edge-hover polling continues without an error or stuck entry.
- [ ] Stash a maximized or minimized window, reveal it, and confirm its original state returns.

## Settings and recovery

- [ ] Change the trigger, radial size, preview settings, colors, and keybinds.
- [ ] Restart LoopW and confirm every changed setting persists.
- [ ] Enter invalid color text. Settings falls back safely and the UI remains usable.
- [ ] Verify the preview follows the selected action and does not remain visible after canceling.

## Scripting / IPC

- [ ] Run `LoopW.exe list/actions` and confirm the action catalog is returned.
- [ ] Run `LoopW.exe list/keybinds` and confirm the trigger and configured binds are returned.
- [x] Focus an external window, run `LoopW.exe direction/right`, and confirm it moves to the right half.
- [ ] Run a malformed command and confirm it returns an `ERROR:` response without exiting LoopW.
- [ ] Start LoopW with a command while it is not running, then confirm it starts resident and executes the command.
- [ ] Invoke commands repeatedly while the tray app is running; confirm there is still one LoopW process and one tray icon.

- [x] Rebuild the solution. `bin/` and `obj/` remain ignored by Git.
