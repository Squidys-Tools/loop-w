<!--
Keep PRs small and focused. One concern per PR — if the description says
"also", split it. Large feature PRs without a prior issue will likely be
closed or asked to shrink. See CONTRIBUTING.md.
-->

## What Changed

<!-- Describe the change clearly and keep scope tight. -->

## Why

<!-- The problem in a sentence or two, then why this approach. -->

## Verification

<!-- Commands run + Win32 evidence. E.g. cargo test suite, loopw-agent run
     with evidence dir, QA.md sections covered or recorded as skipped. -->

## UI Changes

<!-- If this PR changes UI, include clear before/after screenshots.
     If the change involves motion or interaction, include a short video.
     Never commit PR-only screenshots or videos to the repo.
     Delete this section if not applicable. -->

## Checklist

- [ ] Gate passes: fmt, build --locked, test --all-targets --locked, clippy -D warnings
- [ ] Entry-point parity checked (radial / keybind / IPC) or noted as N/A
- [ ] Before/after screenshots for any UI changes
- [ ] Video for animation/interaction changes
