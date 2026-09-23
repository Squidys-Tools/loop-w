# Command interface

The running resident owns one same-user named pipe. Starting a second
`LoopW.exe` with arguments forwards the command to the resident and prints
its reply. With no resident running, the new process becomes resident and
runs the command once.

```powershell
LoopW.exe activate
LoopW.exe list/actions
LoopW.exe list/keybinds
LoopW.exe list/all
LoopW.exe direction/right
LoopW.exe direction/next
LoopW.exe action/maximize
```

- `list/actions` is the source of truth for action names in this build.
- A malformed command returns `ERROR:` without stopping LoopW.
- Pipe traffic is same-user only. A connection from another Windows account
  is rejected.
- Repeated invocations never create a second hook, pipe server, or tray icon.

Note: GUI-subsystem stdout means PowerShell `$()` capture does not show
`LoopW.exe` replies; `cmd.exe` does. Pipe replies to a second `LoopW.exe`
are unaffected. See `docs/internals/architecture.md` for why.
