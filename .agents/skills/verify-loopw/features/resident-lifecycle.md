# Resident lifecycle

LoopW starts as one tray-resident process, opens Settings on activation, keeps
the process alive when Settings closes, and forwards a second launch to the
existing instance.

## Sub-features

- `resident-start` starts one tracked LoopW process.
- `resident-activate` opens the app-owned Settings window.
- `resident-close` closes Settings while the resident process remains alive.
- `resident-forward` forwards a second activation to the first instance.
- `resident-cleanup` stops the exact process created by the run.

## How to get to it (user POV)

- Start LoopW from `LoopW.exe`.
- Open Settings from the tray or run `LoopW.exe activate`.
- Close the Settings window with its window control.
- Start `LoopW.exe activate` a second time while LoopW is resident.
- Choose Quit from the tray for a real user shutdown check.

## Driving it with loopw-agent.ps1

Preconditions:

- `doctor --profile release --json` reports `ok: true` and no resident instance.
- Evidence directory is a disposable path under `%TEMP%\LoopW\verification`.

- **Start.** Run `launch --profile release --build --json`. The result contains
  a `run_id`; `status --run-id <run-id> --json` reports `running: true`.
- **Activate.** Run `& .\target\release\LoopW.exe activate`. The integration
  suite's `IPC activation replies` check returns `LoopW activated.` and finds
  an app-owned `LoopW Settings` window.
- **Close.** Run `run --suite all --profile release --evidence-dir <evidence>
  --json`. The result must include `settings window closes without ending
  resident process`.
- **Forward.** The same suite starts a second `LoopW.exe activate` and requires
  `second instance exits after forwarding` plus `first instance remains
  resident`.
- **Proof.** Preserve the suite JSON and raw log, then inspect the tracked
  launch with `status --run-id <run-id> --json` before cleanup.

## Gotchas

- The app uses a user-global mutex, so a pre-existing tray instance invalidates
  the isolation precondition.
- Closing Settings is not the same as quitting the resident process.
- A taskbar icon or hidden helper window requires the manual taskbar checks in
  `docs\QA.md`; the harness cannot prove visual taskbar appearance.
- The suite owns its own process. Do not stop it by process name.
