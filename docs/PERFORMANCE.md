# Performance measurements

This page records measurements from the optimized Windows builds. It is a
measurement log, not a performance guarantee for every monitor, GPU, or
desktop composition state.

## Release comparison

Measured on 2026-09-11 from this checkout on one Windows desktop. Each build
was started three times, allowed to become resident, sampled after one second
of idle time, and stopped before the next sample.

| Metric | Rust release | WPF release | Comparison |
| --- | ---: | ---: | --- |
| Process launch to `Local\\LoopW.Instance` mutex, median | 19.02 ms | 388.41 ms | Rust was 20.4x faster |
| Mutex time, range | 11.56-19.34 ms | 365.55-452.04 ms | Rust was faster in all samples |
| Idle working set, median | 14.39 MiB | 190.32 MiB | Rust used 92.4% less |
| Idle private memory, median | 2.62 MiB | 111.59 MiB | Rust used 97.7% less |

The Rust build was `target\\release\\LoopW.exe` at commit `1fd77c7`. The WPF
comparison used the existing self-contained artifact at
`C:\\Users\\chris\\.t3\\worktrees\\loop-w\\t3code-73d1d19e\\bin\\Release\\net8.0-windows\\win-x64\\LoopW.exe`,
from WPF checkout commit `1cfa9b2`. The WPF artifact is available locally, so
this comparison is more useful than a source-only estimate, but it is still
one machine and one build of each implementation.

The Rust IPC readiness probe succeeded in all three samples. The WPF artifact
did not return the same `action/...` wire reply within the probe, so its IPC
timing is not presented as a fair comparison. The comparable startup number
above uses mutex acquisition instead.

## Preview and wedge-crossing latency

No valid live interaction numbers were captured in this run.

The measurement harness focused a disposable WinForms target, sent the
configured trigger, and waited for the real `LoopW Radial` and `LoopW Preview`
windows. All three Rust attempts and all three WPF attempts timed out before
the radial window appeared. IPC and non-interactive Win32 checks still passed,
but the synthetic Caps Lock path was not accepted in this session. The
preview-open and wedge-crossing fields are therefore recorded as unavailable,
not zero.

The existing integration suite has previously proven overlay creation, but it
did not timestamp those transitions. A future pass needs a working physical or
synthetic trigger path before it can claim preview or wedge-crossing latency.

## Reproducing the measurement

From the repository root:

```powershell
$wpf = 'C:\\Users\\chris\\.t3\\worktrees\\loop-w\\t3code-73d1d19e\\bin\\Release\\net8.0-windows\\win-x64\\LoopW.exe'
powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File .\\tools\\performance-measurement.ps1 `
  -Target both `
  -Samples 3 `
  -WpfExe $wpf `
  -EvidenceDir "$env:TEMP\\LoopW\\verification\\performance-rerun"
```

The raw sample JSON from this run is preserved at
`C:\\Users\\chris\\AppData\\Local\\Temp\\LoopW\\verification\\performance-final2\\performance-samples.json`.

## Final release verification

The cleanup pass was verified on 2026-09-11 with the same release artifact:

- `cargo fmt --all -- --check`: passed.
- `cargo test --locked`: 177 passed (105 binary tests and 72 UI/settings
  contract tests).
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo build --release --locked`: passed.
- `tools/performance-measurement.ps1`: parsed successfully.
- `tools/loopw-agent.ps1 doctor --profile release --json`: passed with no
  resident instance detected.
- The full Windows harness was attempted without `--skip-radial`, but the
  synthetic trigger timed out waiting for `LoopW Radial`. The harness cleaned
  up the process; no radial/preview result is claimed from that run.
- The remaining release harness checks passed with the radial probe disabled:
  17 passed and 1 skipped in 6.3 seconds. Evidence is preserved at
  `C:\\Users\\chris\\AppData\\Local\\Temp\\LoopW\\verification\\final-cleanup-skip-radial-84ef629e76104b02beacdf42820c7bd0`.

The radial timeout is an environment-sensitive trigger-input limitation, not
evidence that the overlay path is correct. Physical trigger/release behavior
and settings-canvas visuals still require the manual checks in `docs/QA.md`.
