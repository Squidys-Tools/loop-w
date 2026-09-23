---
name: verify-loopw
description: >-
  Verify the LoopW Windows desktop app and its local command interface through
  the agent-friendly PowerShell harness. Use after changes to lifecycle, IPC,
  Win32 actions, overlays, settings activation, or radial input behavior.
---

# Verify LoopW

LoopW is a Windows tray-resident desktop window manager with an iced Settings
window, global trigger hooks, radial and preview overlays, Win32 window actions,
and a same-user named-pipe command interface. The primary surface is the real
Windows desktop app. The command interface is a secondary automation surface.

## Launch

Run from the repository root in a Windows PowerShell session:

```powershell
$launch = powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 launch --profile release --build --json | ConvertFrom-Json
$launch.run_id
```

The command starts one hidden, release-profile LoopW process and returns a
`run_id`, `pid`, and executable path. It is ready when the command succeeds and
`status --run-id <run-id> --json` reports `running: true` and
`process_path_matches: true`. The app remains resident in the tray; opening
Settings is a separate user action through `LoopW.exe activate` or the tray.

For the bounded integration suite, use `run` instead. It starts and cleans up
its own disposable LoopW process and WinForms target:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 run --suite all --profile release --build --json --evidence-dir "$env:TEMP\LoopW\verification\integration"
```

Do not run two LoopW verification sessions at once. The app uses the global
`Local\LoopW.Instance` mutex and `LoopW-Commands` named pipe, so concurrent
sessions would drive the same user-level instance.

## Doctor

Run this before driving an existing launch or when a verification run behaves
unexpectedly:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 doctor --profile release --json
```

Require `ok: true`, an existing executable, both integration scripts, a Cargo
executable, and `resident_instance_detected: false` before starting a fresh
run. For a tracked process, also require:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 status --run-id <run-id> --json
```

The status must identify the expected PID and executable path.

## Drive

Read [`features/README.md`](features/README.md), choose one mapped feature,
and start from its stated preconditions. Use `tools/loopw-agent.ps1` for
process control and the existing `tools/windows-integration.ps1` checks for
real Win32 behavior. Use LoopW's documented command interface for direct
actions, for example:

```powershell
& .\target\release\LoopW.exe list/actions
& .\target\release\LoopW.exe activate
& .\target\release\LoopW.exe action/maximize
```

Prefer `--json` for wrapper output. The integration harness uses real window
handles, a disposable WinForms target, the named pipe, and real keyboard input
when the radial path is enabled. It does not use internal setters or test-only
endpoints.

## Evidence

Every proof should capture the user action and resulting state. Run the suite
with `--evidence-dir <path>` so the wrapper preserves both the raw harness log
and a JSON summary containing pass/skip counts, duration, exit code, and full
output:

```powershell
$evidence = Join-Path $env:TEMP ('LoopW\verification\' + [Guid]::NewGuid().ToString('N'))
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 run --suite all --profile release --skip-radial --evidence-dir $evidence --json
Get-ChildItem -LiteralPath $evidence
```

Use the log to prove the command and visible Win32 result together. A passing
IPC reply without the target window geometry or overlay ownership assertion is
incomplete. A skipped radial release path is not proof of physical trigger
release behavior; record it as skipped and use the manual checklist in
`docs/QA.md` for that path.

## Cleanup

For a process started by `launch`, inspect first and then stop only its exact
tracked run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 stop --run-id <run-id> --dry-run --json
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 stop --run-id <run-id> --yes --json
```

The wrapper verifies the recorded PID and executable path before stopping it.
It removes only that run's state file. The integration suite cleans up the
LoopW process, disposable target, overlays, cursor position, and trigger state
it started. Evidence under the named evidence directory is never removed.
Never kill LoopW by process name and never delete the evidence directory during
cleanup.

## Helpers

The project-local helper is [`tools/loopw-agent.ps1`](../../../tools/loopw-agent.ps1).
Use its layered help when a command changes:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 --help
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\loopw-agent.ps1 run --help
```

The helper is intentionally a wrapper, not a second Win32 test implementation.
Update the wrapper and this skill together when the app's user-facing entry
points change.

For ongoing map maintenance, use `/maintain-verification-skill` after changes
to commands, settings entry points, overlays, or the integration harness.
