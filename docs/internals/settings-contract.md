# Settings contract

Settings live at `%LOCALAPPDATA%\LoopW\settings.json`, with the same path,
PascalCase keys, and integer enums as the old C# build. Existing files load
without loss; unknown or out-of-range values normalize to safe defaults.

## Load

`settings::load_report` reads and normalizes in one pass: invalid JSON,
missing keys, bad colors, out-of-range numbers, and radial slots pointing at
deleted keybinds all fall back to no-op defaults. The UI keeps editing while
the loader never panics on the desktop path.

## Save

`settings::persistence` writes to a temp file in the same directory and
renames over the original only after the write succeeds, so a crash
mid-write keeps the last complete file. `settings::sync` pushes the save
result back into the settings UI: on failure the control reverts to the
last saved value and surfaces a sticky `save_error` instead of leaving an
unsaved edit sitting in the shared mirror.

Covered by the `load_report` + `sync` suites (invalid JSON, partial writes,
migration). When the contract changes, update the normalizer and its tests
together — the tolerant loader is the compatibility story.
