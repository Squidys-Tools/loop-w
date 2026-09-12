# LoopW verification map

This directory is the maintained source for verifying LoopW's user-facing
Windows behavior. Read the baseline and conventions before choosing one
feature file to drive.

## Baseline preconditions

- Run on Windows from the repository root.
- Build or select `target\release\LoopW.exe`.
- Run `doctor --profile release --json` and require no resident LoopW instance.
- Do not drive an instance that this verification run did not start.
- Keep one verification run active at a time because LoopW's mutex and named
  pipe are user-global.
- Save evidence under `%TEMP%\LoopW\verification\<run-id>` and never remove it
  during cleanup.

## Driving conventions

- Use `tools\loopw-agent.ps1` through `powershell.exe -NoProfile`.
- Prefer `--json` for machine-readable results.
- Use `run --evidence-dir <path>` for the bounded suite.
- Pair each user action with its observable result and a side-effect check.
- Treat `SKIP` as unverified, not as a pass.
- Use the manual checklist in `docs\QA.md` for physical input paths that the
  harness explicitly marks environment-sensitive.

## Proof and skip reporting

- Capture the action, the response, and the resulting window or process state.
- Preserve the wrapper's JSON summary and raw log.
- For placement actions, verify both the IPC reply and target geometry.
- For overlays, verify ownership, styles, visibility, and cleanup.
- Do not report Settings as fully UI-verified from an IPC activation check alone.
- Record a skipped radial trigger-release check as skipped and name the manual
  scenario still required.

## Features

- [Resident lifecycle](./resident-lifecycle.md) covers launch, activation,
  settings close, second-instance forwarding, tray popup lifecycle, and
  cleanup.
- [Window actions](./window-actions.md) covers command discovery, maximize,
  half-window placement, and observable target geometry.
- [Radial and preview overlays](./radial-overlays.md) covers overlay creation,
  ownership, styles, preview lifecycle, and trigger-path limits.
- [Settings entry point](./settings-entry.md) covers the user-visible Settings
  activation and close lifecycle, including the custom tray popup, without
  overstating desktop rendering proof.
