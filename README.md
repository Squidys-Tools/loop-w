# LoopW

LoopW is a Windows window manager built around a radial menu. Hold the trigger,
move toward a direction, and release to place the active window. It is a native
WPF and Win32 app inspired by [Loop for macOS](https://github.com/MrKai77/Loop).

LoopW is still early-stage. The core features are implemented, but the project
needs a full manual desktop QA pass across different window types, display
scales, and Windows configurations. Expect rough edges.

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
- Provides a small local command interface for scripts and automation.

The default trigger is Caps Lock. LoopW captures the configured trigger while
it is active, so Caps Lock will not toggle its normal state unless you change
the trigger in Settings.

## Install

When a GitHub Release is available, download the Windows ZIP and checksum from
the [Releases page](https://github.com/Squidys-Tools/loop-w/releases), extract
the archive, and run `LoopW.exe`. The publish workflow produces a
self-contained `win-x64` build, so a separate .NET runtime is not needed for
that package.

For now, building from source is the most reliable way to try the current code.

## Build from source

You need:

- Windows
- The .NET 8 SDK

The live preview uses the prebuilt Win2D package restored by NuGet. An MSVC
compiler is not needed to build LoopW. The published folder carries the native
Canvas dependency and C++ forwarders needed at runtime.

From PowerShell:

```powershell
git clone https://github.com/Squidys-Tools/loop-w.git
cd loop-w
dotnet build LoopW.csproj
dotnet run --project LoopW.csproj
```

The app starts hidden in the system tray. Open Settings from the tray icon to
change the trigger, keybinds, radial assignments, previews, appearance, and
advanced window policies.

If the live preview falls back to the bitmap preview, inspect
`%LOCALAPPDATA%\LoopW\live-preview.log` after reproducing it. The log records
whether Windows rejected the backdrop, Win2D failed to load, or Composition
failed to attach.

To make a portable x64 folder, publish the app instead of copying the build
output:

```powershell
dotnet publish LoopW.csproj -c Release -r win-x64 --self-contained true `
  -p:PublishSingleFile=true -o publish\LoopW
```

Copy the whole `publish\LoopW` folder. The single-file executable contains the
.NET runtime, while the folder also carries the native Win2D Canvas DLL and its
app-local C++ forwarders. Do not distribute only `LoopW.exe`. The live preview
also needs Windows 11 build 22000 or newer. Older Windows versions use the
existing bitmap preview path.

Run the automated checks with:

```powershell
dotnet run --project LoopW.Tests/LoopW.Tests.csproj
```

## Configuration

LoopW saves settings here:

```text
%LOCALAPPDATA%\\LoopW\\settings.json
```

Settings changes save automatically. The app normalizes invalid or older values
when it loads them, and radial slots that point to missing keybinds fall back to
no action.

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

## Project docs

- [`docs/ROADMAP.md`](docs/ROADMAP.md) tracks what is implemented and what is
  next.
- [`docs/QA.md`](docs/QA.md) contains the manual Windows test checklist.
- [`specs/settings-ui-redesign-spec.md`](specs/settings-ui-redesign-spec.md)
  records the settings UI requirements and acceptance scenarios.

## License

LoopW is released under the [MIT License](MIT%20License).
