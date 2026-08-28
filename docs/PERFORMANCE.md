# Performance diagnostics

LoopW keeps aggregate runtime timings disabled by default so diagnostics do not
add work to global input hooks.

Enable them for one run with the `LOOPW_PERF` environment variable:

```powershell
$env:LOOPW_PERF = "1"
dotnet run --project LoopW.csproj
```

The summary is written to the process trace output when LoopW exits. It includes
count, average duration, and maximum duration for:

- startup
- keyboard and mouse hooks
- stash polling
- overlay capture
- window placement
- settings persistence
- settings runtime application

There is no per-mouse-event log. Use the existing manual checklist in
`docs/QA.md` while collecting timings. Compare idle, radial, preview, drag,
settings, stash, and placement flows across DPI and multi-monitor setups.
