# Contributing to LoopW

Contributions are welcome, kept small on purpose. LoopW is a live-desktop
utility — a bad hook, overlay, or placement change is felt immediately — so
review bandwidth goes to focused, well-evidenced PRs.

## Most likely to merge

- Small, focused bug fixes with a regression test.
- Small reliability fixes (save/load fallbacks, stale-state cleanup, safe
  no-ops for unsupported windows).
- Small, measured performance improvements (hook path, overlay redraw,
  startup/memory — with numbers from `tools/performance-measurement.ps1`).
- Tightly scoped maintenance that makes the codebase smaller or clearer
  without changing behavior.

## Less likely without prior discussion

- Large PRs (1000+ lines), drive-by features, rewrites, new settings, new
  surfaces. Open an issue first and get a go-ahead — otherwise it will
  probably be closed or asked to shrink.
- Anything expanding product scope (installer, updater, new integrations).
  Propose it as an issue; do not build it uninvited.

## Before opening a PR

1. Read `AGENTS.md` (safety + entry-point parity) and the relevant page
   under `docs/` — `user/` if behavior changes, `internals/` if a decision
   changes, `operations/development.md` for commands.
2. Keep one concern per PR. If the description says "also", split it.
3. Run the gate from `AGENTS.md`: `cargo fmt -- --check`,
   `cargo build --locked`, `cargo test --all-targets --locked`,
   `cargo clippy --all-targets --locked -- -D warnings`. For Win32 behavior
   add `verify-loopw` evidence; for trigger/overlay/DPI add `docs/QA.md`
   coverage or record it as a skip.
4. Follow the docs rule: most code needs no docs change. Update `docs/user/`
   when how to use it changes; update `docs/internals/` only when a
   maintainer would get it wrong from code alone. Update `ROADMAP.md`/`QA.md`
   status lines when a check actually ran.
5. UI changes need before/after screenshots; motion or timing needs a short
   video. Never commit PR-only screenshots or videos.

Titles are conventional (`fix(win): ...`, `feat(ui): ...`). The body states
the problem in a sentence or two, then the fix. Be realistic: a PR may be
closed, shrunk, or reimplemented. Small and evidenced is the fastest path
to merged.
