# Radial menu and preview

Hold the trigger. The radial opens at the cursor. Move toward a direction,
release to place the foreground window. The overlay itself never shows text —
settings labels describe actions, the live radial stays geometry-only.

- Eight wedges (halves, quarters, thirds) plus a center slot.
- Moving into the center clears the selection and hides the preview.
- Release, left-click, or arrow-key commits the highlighted wedge exactly
  once. Esc cancels without moving the window.
- Repeating a cardinal action cycles half → third → two-thirds. Quarters do
  not cycle unless configured as a cycle.
- The preview frame shows the target before commit. It never steals focus
  and never leaves an orphaned overlay after cancel.

## Customizing

Settings → Radial menu: enable/disable, radius, center size, colors, and
per-wedge targets. Settings → Preview: padding, corner radius, border.
Changes apply without restarting the resident process. Disabling the radial
leaves keybinds and `LoopW.exe` commands working.

Custom wedge and center assignments may point to built-in actions, cycles,
or keybinds. A slot that points at a deleted keybind does nothing rather
than guessing.
