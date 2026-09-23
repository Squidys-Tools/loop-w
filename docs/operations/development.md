# Development

Fresh checkout, Windows, Rust stable via `rustup`:

```powershell
cargo fmt -- --check
cargo build --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo run
cargo build --release --locked
```

## Iterate narrow, gate wide

During iteration run the narrowest check that covers the change
(`cargo test <name>`, one module suite). Before calling work done run the
full gate above. The full suite (180 tests) is cheap here — it *is* the
gate, plus Win32 behavior below.

## Win32 verification

Agent-driven desktop checks go through the `verify-loopw` skill and
`tools/loopw-agent.ps1`:

```powershell
.\tools\loopw-agent.ps1 doctor --json
.\tools\loopw-agent.ps1 run --suite all --profile release --build --json
.\tools\loopw-agent.ps1 launch --profile release --json
.\tools\loopw-agent.ps1 status --run-id <run-id> --json
.\tools\loopw-agent.ps1 stop --run-id <run-id> --yes --json
```

- `doctor` must report `ok: true` and `resident_instance_detected: false`
  before a fresh run.
- `run --evidence-dir <path>` preserves the raw harness log plus its JSON
  summary (pass/skip counts, duration, exit code). Evidence is never deleted.
- `stop` takes `--dry-run` first, then `--yes`. Never `taskkill /IM`.
- Never run two verification sessions at once (mutex + pipe singletons).
- An IPC reply is not proof by itself — assert target geometry and overlay
  ownership. A skipped radial probe is a skip; the trigger path stays manual
  in `docs/QA.md`.

## Tests worth writing

Unit-test decisions in `core` and contract behavior in `settings`. For
`win`, test the decision the wrapper makes, not Win32 itself. Do not assert
rendered markup, callback wiring, or implementation mirrors — assert the
frame, the fallback, or the visible error.
