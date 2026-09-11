# LoopW

LoopW is a Windows window manager built around a radial menu. Hold the trigger,
move toward a direction, and release to place the active window. It is a native
Rust + [iced](https://iced.rs) and Win32 app inspired by [Loop for macOS](https://github.com/MrKai77/Loop).

This tree is the Rust port (iced 0.14 UI, self-contained `win-x64` EXE, no
.NET runtime). The C# / WPF implementation has been replaced at the repo root;
`git log` retains its history.

The core product path, settings model, radial/preview surfaces, settings UI,
and native Win32 backend are implemented. The project still needs code-level
hardening and a full manual desktop QA pass across different window types,
display scales, and Windows configurations. Desktop checks are not complete.

The current test inventory is 98 binary tests plus 72 UI/settings contract
tests, for 170 tests under `cargo test --all-targets --locked`. The locked
build, formatting check, tests, and warnings-denied Clippy pass. A bounded
Windows integration harness lives under `tools/` and covers resident lifecycle,
IPC, disposable-window placement, and overlay HWND styles; broad desktop QA
remains separate.

## What it does

- Opens a text-free radial menu at the cursor and previews the target frame.
- Places windows into halves, quarters, thirds, two-thirds, centered zones,
  maximized zones, fullscreen, and custom resize areas.
- Moves windows between monitors and can focus nearby windows by direction or
  z-order.
- Supports resize, grow, shrink, fill-available-space, minimize, hide, undo,
  and stash/reveal actions.
- Lets you create keybinds, cycles, trigger-bypass bindings, and custom radial
  wedge or center assignments.
- Can snap title-bar drags to monitor edges and corners.
- Handles mixed-DPI monitor layouts, screen padding, application exclusions,
  launch at login, tray operation, and single-instance activation.
- Keeps the Winit event-target helper out of the taskbar while preserving the
  paint messages needed by the resident runtime.
- Provides a small local command interface for scripts and automation.

The default trigger is Caps Lock. LoopW captures the configured trigger while
it is active, so Caps Lock will not toggle its normal state unless you change
the trigger in Settings.

## Install

When a GitHub Release is available, download the Windows ZIP and checksum from
the [Releases page](https://github.com/Squidys-Tools/loop-w/releases), extract
the archive, and run `LoopW.exe`. The publish workflow produces a
self-contained `win-x64` build, so no separate runtime is needed for
that package.

For now, building from source is the most reliable way to try the current code.

## Build from source

You need:

- Windows
- The Rust stable toolchain (`rustup`)

From PowerShell:

```powershell
git clone https://github.com/Squidys-Tools/loop-w.git
cd loop-w
cargo build
cargo run
```

The app starts with the Settings window. Tray-resident operation, global hooks,
and overlays are wired through the Win32 backend; live desktop validation is
tracked separately in the roadmap and QA checklist.

Run the automated checks with:

```powershell
cargo fmt -- --check
cargo build --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
```

Ship a self-contained release binary with:

```powershell
cargo build --release
```

The EXE at `target\release\LoopW.exe` needs no
separate runtime.

## Configuration

LoopW saves settings here (same path and keys as the previous C# build, so
existing files load without loss):

```text
%LOCALAPPDATA%\LoopW\settings.json
```

Settings changes save automatically. The app normalizes invalid or older values
when it loads them, and radial slots that point to missing keybinds fall back to
no action. Saves go through a temp file + rename so a crash mid-write keeps the
last complete file.

Global keyboard and mouse hooks are part of the product. Review the trigger and
exclusion settings before using LoopW on a machine where those hooks are not
appropriate.

## Command interface

The running app accepts one command argument. Starting a second `LoopW.exe`
passes the command to the existing tray instance through a same-user named pipe.

```powershell
LoopW.exe activate
LoopW.exe list/actions
LoopW.exe list/keybinds
LoopW.exe list/all
LoopW.exe direction/right
LoopW.exe direction/next
LoopW.exe action/maximize
```

Use `list/actions` to see the action names supported by the current build.

For agent-friendly Windows driving, use the wrapper around the integration
harness. It never prompts, supports JSON output, and tracks only processes it
started:

```powershell
.\tools\loopw-agent.ps1 doctor --json
.\tools\loopw-agent.ps1 run --suite all --profile release --build --json
.\tools\loopw-agent.ps1 launch --profile release --json
.\tools\loopw-agent.ps1 status --run-id <run-id> --json
.\tools\loopw-agent.ps1 stop --run-id <run-id> --yes --json
```

Use `run --evidence-dir <path>` to save the raw harness log and its JSON
summary. Use `stop --dry-run` when inspecting cleanup before allowing it.

## Project docs

- [`docs/ROADMAP.md`](docs/ROADMAP.md) tracks what is implemented and what is
  next.
- [`docs/QA.md`](docs/QA.md) contains the manual Windows test checklist.
- [`docs/BUGS.md`](docs/BUGS.md) records deferred minor issues with revisit
  triggers, so they are tracked without blocking the release gate.
- [`specs/settings-ui-redesign-spec.md`](specs/settings-ui-redesign-spec.md)
  records the settings UI requirements and acceptance scenarios.

## Port notes

- `src/core` is platform-independent and fully unit tested (`cargo test`).
- `src/settings` keeps the exact C# JSON contract (PascalCase keys, integer
  enums) with a tolerant loader.
- `src/ui` is the iced 0.14 settings surface plus the shared text-free radial
  and preview canvas renderers. The settings canvases use fixed dimensions and
  cached geometry; desktop rendering still needs manual verification.
- `src/win` owns all Win32 side effects (hooks, window actions, snap, stash,
  IPC, tray). The backend is implemented; desktop validation and runtime
  hardening remain. Pure logic it depends on is tested in `src/core`.
- The former `src/ui/app.rs` monolith is split into focused settings, runtime,
  and window modules. `src/win/stash_service.rs`, `src/win/hooks.rs`,
  `src/core/frame_math.rs`, and `src/win/ipc.rs` remain larger follow-up
  candidates; new code should stay modular.

## License

LoopW is released under the [MIT License](MIT%20License).
