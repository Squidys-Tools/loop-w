# Release

There is one workflow, `.github/workflows/publish.yml`. It runs on manual
dispatch or a `v*` tag push. It checks formatting, tests, and warning-denied
Clippy, builds the release EXE, and uploads `LoopW-<label>.zip` plus its
`.sha256`. Tag runs additionally create a GitHub Release. Nothing runs
automatically on every pull request.

## Ship when

From `docs/ROADMAP.md`, the release gate is:

1. Formatting, locked build, all-targets tests, warning-denied Clippy pass.
2. The `docs/QA.md` desktop checklist passes on the supported Windows and
   display setups. Each exception gets a documented reason.
3. Trigger, radial, keybind, snapping, stash, monitor, exclusion, tray, and
   IPC behavior agree across entry points.
4. Existing settings files load without loss; save failures are visible and
   recoverable.
5. Unsupported windows fail safe with a clear message.

Packaging (installer, signing, update path) is a separate decision after
this gate. The current artifact is a self-contained `win-x64` ZIP needing
no runtime.
