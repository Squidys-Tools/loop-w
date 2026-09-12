# Window actions

Window actions apply a placement or state change to the currently focused
target window and report the result through LoopW's local command interface.

## Sub-features

- `actions-list` discovers the current action names.
- `action-maximize` fills the target monitor work area.
- `action-left-half` places the target in the left half of the work area.
- `action-reply` returns a human-readable applied-action response.

## How to get to it (user POV)

- Focus a normal disposable window.
- Choose an action from the radial menu.
- Use a configured named action or run the equivalent `LoopW.exe action/...`
  command.

## Driving it with loopw-agent.ps1

Preconditions:

- Run on Windows with no pre-existing LoopW instance.
- The release executable and integration scripts are present.
- The harness can create a disposable WinForms target.

- **Discover.** Run `& .\target\release\LoopW.exe list/actions`. The output
  contains action entries beginning with `action/`.
- **Maximize.** Run `run --suite all --profile release --skip-radial
  --evidence-dir <evidence> --json`. Require `Win32 maximize action replies`
  and `Win32 maximize effect is visible`.
- **Left half.** Require `Win32 geometry action replies` and `Win32 left-half
  effect is bounded to monitor work area`; the log records target frame and
  work-area geometry.
- **Proof.** Preserve the JSON summary and raw log. The reply alone is not
  sufficient without the target geometry assertion.

## Gotchas

- The target is created and owned by the harness; never substitute an
  arbitrary user window.
- Work-area coordinates vary with monitor layout and taskbar placement, so use
  the harness assertions rather than hard-coded screen dimensions.
- Actions operate on the foreground target. Do not change focus during an
  action check.
