# Keybinds and cycles

Settings → Advanced → Keybinds.

- **Add / rebind / delete.** Capture works like the trigger: press, Esc
  cancels, duplicates are rejected before they reach the hook.
- **Cycles.** Bind one key to step half → third → two-thirds on repeated
  presses. Radial wedge targets can point at a cycle by its stable ID.
- **Bypass.** A keybind flagged as bypass fires even when it would otherwise
  open the radial.

Keybinds, radial slots, and `LoopW.exe action/<name>` resolve through the
same action catalog, so all three do the same thing for the same name.

## Stash

Stash parks a window at a screen edge, leaving a small visible strip. Hover
the strip (or use the reveal keybind / `action/reveal`) to restore the
exact frame, monitor, DPI, and minimized/maximized state. Closing a stashed
window externally clears its entry — no stuck strip.
