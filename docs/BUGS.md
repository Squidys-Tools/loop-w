# LoopW known issues (deferred)

Minor defects and accepted deviations found during the C# → Rust port
audits. None blocks the release gate; each entry records where it lives,
why it was deferred, and what would trigger revisiting it. Local quality
gates (`cargo fmt --check`, `cargo test --locked`,
`cargo clippy --all-targets --locked`, via lefthook) stay green regardless.

## 1. Hook timer threads pile up under rapid churn — accepted

- **Where:** `src/win/hooks.rs` (`schedule_activation_locked`, timeout arm in
  `activate_locked`).
- **What:** every trigger press spawns a sleeping thread for the activation
  delay, and every activation spawns another for the release timeout. Rapid
  press/release churn accumulates short-lived threads instead of cancelling
  a shared timer (C# disposes its `System.Threading.Timer`).
- **Why deferred:** correctness is preserved by the `_stateVersion` guard —
  stale timers wake, see a version mismatch, and exit without effect. Threads
  live at most ~10 s (the timeout clamp) and only spawn on real trigger
  presses, so the pile-up is bounded and self-reaping.
- **Revisit if:** profiling ever shows thread-count spikes, or delays above
  ~1 s are introduced. Fix direction: single timer thread + cancel channel.

## 2. Pipe 256 limit counts bytes, C# counts chars — accepted

- **Where:** `src/win/ipc.rs` (`read_command`).
- **What:** over-limit input is dropped at 256 UTF-8 bytes; C# drops at 256
  `char`s. A non-ASCII command near the limit can be cut mid-codepoint and
  decodes with a replacement character instead of being measured precisely.
- **Why deferred:** byte-identical for ASCII, and every real command
  (`direction/*`, `action/*`, `list/*`, `activate`) is pure ASCII and far
  below the limit.
- **Revisit if:** commands ever accept free-form text (window titles, paths).
  Fix direction: decode incrementally and enforce the cap in chars.

## 3. CLI output invisible under PowerShell capture — platform reality

- **Where:** `src/win/native.rs` (`print_cli_line`).
- **What:** PowerShell `$()` and `>` redirection do not propagate std handles
  into GUI-subsystem processes, so there is nothing to write to and the
  `AttachConsole` fallback paints the (invisible-to-the-pipe) screen buffer.
  `cmd.exe` console output works; `.NET`-client pipe traffic is unaffected.
- **Why deferred:** identical Win32 mechanics to the C# build (`AttachConsole`
  + `WriteLine` + `FreeConsole`), so this is parity, not a regression. When
  stdout IS a valid pipe/file handle, `print_cli_line` writes directly with
  no console API involved.
- **Revisit if:** scripted CLI use becomes common. Fix direction: a tiny
  console-subsystem helper binary, or long-lived JSON-over-pipe mode.

## 4. Minimized spin-wait is stricter than C# — intentional

- **Where:** `src/win/placement.rs` (`place_window`).
- **What:** if `GetWindowPlacement` fails during the 10×50 ms un-minimize
  wait, Rust returns false; C# ignores the failure and proceeds with the
  stale placement.
- **Why deferred:** failing closed is strictly safer (never places a window
  from garbage data); the only cost is a rarer retry by the caller.
- **Revisit if:** a real app reproducibly fails to restore-from-minimized
  where C# succeeded.

## 5. SnapBegin does hit-testing on the UI thread per click — parity

- **Where:** `src/win/snap_service.rs` (`begin_at_cursor`: `WindowFromPoint`
  + policy + `NCHITTEST` with 100 ms timeout).
- **What:** every left-button press anywhere runs the drag-start gates on
  the UI thread. A click storm (or a hung foreground app stalling the
  100 ms `SendMessageTimeout`) can jank settings-window interaction.
- **Why deferred:** the C# build does the same work on its dispatcher, so
  behavior and cost match; the early-outs (feature off, session active,
  ineligible window) keep the common path cheap.
- **Revisit if:** UI jank is traced here. Fix direction: move hit-testing to
  the hook thread's async margin (it must never block input) or debounce.

## 6. Hover preview recomputes every frame while open — accepted

- **Where:** `src/ui/app.rs` (`update_radial_hover`).
- **What:** while the radial overlay is open, each frame re-resolves the
  hovered wedge's target frame (`GetWindowRect` + monitor lookup) even when
  the cursor is still, so display/DPI changes move a live preview.
- **Why deferred:** the overlay lives for seconds at most and the calls are
  cheap; the alternative (stale preview after display change) is worse.
- **Revisit if:** profiling flags it. Fix direction: recompute only on
  cursor movement + display-change generation bump.

## 7. Single pipe instance serializes all clients — parity

- **Where:** `src/win/ipc.rs` (`server_loop`, `maxInstances = 1`).
- **What:** a second client that arrives while the server waits up to 2 s
  for the UI to answer a first command retries instead of queueing (same as
  C#). A hung UI frame stalls every CLI caller for the same window.
- **Why deferred:** byte-for-byte C# semantics, including the retry timing.
- **Revisit if:** CLI-driven automation needs concurrency. Fix direction:
  per-connection thread + reply routing by id (the `set_reply` map already
  keys by id, so this is mostly the accept loop).

## 8. Server SA descriptor intentionally leaks — accepted

- **Where:** `src/win/ipc.rs` (`current_user_sa`, `server_loop`).
- **What:** the `SECURITY_ATTRIBUTES` + SID buffer are boxed and never freed
  for process lifetime (one small allocation).
- **Why deferred:** the descriptor must outlive every accepted pipe instance;
  freeing it would require reference counting for zero benefit.
- **Revisit if:** never — only noted so it isn't "fixed" into a
  use-after-free.
