# Trigger

The trigger opens the radial menu. The default is Caps Lock. While the
trigger is held, Caps Lock does not toggle its normal state.

## Change it

Settings → General → Trigger. Click the capture field, press the new
combination, Esc cancels. Reserved or conflicting combinations are rejected
inline and never reach the hook.

## Timing options

- **Activation delay** — hold must persist this long before the radial opens.
- **Release timeout** — how long the radial waits for release/commit.
- **Double-click trigger** — optional second press to open.
- **Middle-mouse** — optional radial on middle button. When off, middle-click
  behaves normally everywhere.
- **Left/right distinction** — on keyboards that expose both sides, only the
  configured side activates.

## Bypass keybinds

A bypass keybind fires its action without opening the radial. It fires once
per press and must not conflict with normal typing. Removing it restores
normal behavior — there is no leftover suppression.

If the keyboard or mouse drops mid-gesture, LoopW cancels the gesture and
the hook stays usable. A normal key press with no recognized gesture is
passed through untouched.
