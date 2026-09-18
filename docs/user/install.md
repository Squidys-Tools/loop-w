# Install and first run

LoopW is a portable tray app. There is no installer.

## From a release ZIP

1. Download the Windows ZIP and its `.sha256` from the Releases page.
2. Extract it and run `LoopW.exe`.
3. LoopW stays in the tray. Open Settings from the tray popup, by
   double-clicking the tray icon, or with `LoopW.exe activate`.

## From source

You need Windows and the Rust stable toolchain (`rustup`).

```powershell
git clone https://github.com/Squidys-Tools/loop-w.git
cd loop-w
cargo build
cargo run
```

The app starts hidden in the tray. Global hooks are active while it runs —
review the trigger and exclusion settings first on machines where hooks are
sensitive.

## Where settings live

```text
%LOCALAPPDATA%\LoopW\settings.json
```

The file uses the same path and keys as the previous C# build, so existing
files load without loss. Edits in the settings window save automatically.
A failed save keeps the last complete file and shows the error in the UI
instead of silently dropping it.
