# Settings entry point

Settings is the dark iced configuration window opened from the tray or through
the local activation command, and it closes without ending the resident app.

## Sub-features

- `settings-tray` opens Settings from the user tray menu.
- `settings-activate` opens Settings through `LoopW.exe activate`.
- `settings-close` closes the window while the tray process remains resident.
- `settings-rendering` requires a manual desktop pass for canvas appearance and
  scrolling responsiveness.

## How to get to it (user POV)

- Choose Settings from the LoopW tray menu.
- Start a second `LoopW.exe` with the `activate` argument.
- Close the Settings window with its window control.
- Scroll the Window Preview and other settings tabs.

## Driving it with loopw-agent.ps1

Preconditions:

- Start a tracked release process with `launch --profile release --build
  --json`, or let the integration suite own the process.
- No unrelated LoopW instance is running.

- **Activate.** Run `& .\target\release\LoopW.exe activate`. The resulting
  IPC reply is `LoopW activated.` and a visible `LoopW Settings` window appears
  in the harness's app-owned window inventory.
- **Close.** Run `run --suite all --profile release --skip-radial
  --evidence-dir <evidence> --json`. Require `settings window closes without
  ending resident process` and `forwarded activation can be closed`.
- **Rendering.** Open the Settings window from the tray, select Window Preview,
  scroll several increments, and observe frame rate and input latency. Record
  this as a manual result; the CLI has no stable desktop accessibility surface
  for iced canvas pixels.
- **Proof.** Keep the harness JSON/log for lifecycle proof and separately record
  the manual tab, scroll action, and observed rendering result.

## Gotchas

- IPC activation proves the entry point and lifecycle, not every Settings
  control, canvas pixel, or scroll-performance claim.
- Closing Settings should leave the resident tray process alive.
- Do not report the settings UI as fully verified from a successful build or
  an activation reply alone.
