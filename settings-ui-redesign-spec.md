# LoopW Settings UI Redesign Specification

**Status:** Ready for implementation planning  
**Date:** 2026-08-15  
**Scope:** Settings surface redesign only; no behavior or persistence changes unless explicitly required to support the new presentation.

## 1. Summary

Redesign the LoopW settings experience into a modern, polished, premium, and coherent native Windows settings surface. The redesign should feel like a carefully crafted desktop utility rather than a collection of styled controls: quiet, confident, highly scannable, and precise.

The product should retain its existing dark visual identity and blue-accent lineage, but evolve the palette and component system into a more sophisticated tonal system. The redesign should reorganize the current settings lightly rather than replace the information architecture wholesale. It should preserve all currently supported settings and their existing persistence semantics while improving hierarchy, navigation, feedback, accessibility, and visual coherence.

This is an **Operate** surface: users come here to configure LoopW quickly, understand what each option does, and leave without friction. Utility, scanability, and native Windows expectations outrank decoration.

## 2. User-confirmed direction

These decisions were gathered during the design interview and are requirements for the spec:

- Overall direction: **quiet premium**.
- Surface language: **subtle depth**, not heavy glass, glow, or decorative effects.
- Brand continuity: evolve the existing dark/blue identity rather than discard it.
- Information architecture: retain the current feature groupings, but reorganize and rename lightly where it improves clarity.
- Navigation: persistent **left sidebar** with a content pane.
- Primary context: a desktop-first WPF settings window.
- Initial priority: make **trigger and launch behavior** the most prominent content.
- Advanced controls: use a **dedicated advanced section** rather than hiding important capabilities behind ambiguous menus.
- Theme editing: provide **curated presets** plus a **full visual editor as an advanced option in the same theme area**.
- Change behavior: use a **mixed-by-risk** model: harmless visual changes save immediately; higher-impact behavior changes may require confirmation or explicit apply treatment when needed.
- Narrow-window behavior: preserve a spacious composition with a firm practical minimum size; do not optimize for very narrow layouts.
- Motion: use polished, restrained transitions with reduced-motion support.
- Content tone: crisp utility—short labels, direct explanations, and highly scannable copy.
- Landing state: pure settings; do not add a dashboard, status hero, recent-changes feed, or prominent quick-action panel.
- Recovery: provide both per-section reset and a clearly separated global reset-all action.
- Trigger/keybind capture: use inline capture mode with visible listening, Esc cancellation, and inline reserved-key errors.
- App appearance: dark mode remains the standard product choice, but users can choose to follow Windows light/dark mode or explicitly choose the app mode.
- Explicit avoidances: avoid over-decoration, excessive glow/gradients, oversized hero content, dense technical UI, tiny controls, and raw jargon.
- Quality emphasis: visual polish first, with keyboard usability, visible focus, readable contrast, and Windows scaling treated as required quality concerns.

## 3. Existing product context

LoopW is a native .NET 8 WPF/Win32 Windows utility for window management. The app runs resident in the system tray and uses a global trigger key to open a radial window-management surface. The settings surface is hosted directly inside `MainWindow`; the former `SettingsWindow` is currently a `UserControl` placed into `MainWindow.SettingsHost`.

Current relevant behavior and constraints:

- `MainWindow` starts hidden for resident/tray operation and is shown or activated from the tray.
- Closing the main window hides it to the tray rather than exiting.
- Settings changes are currently persisted automatically to `%LOCALAPPDATA%\\LoopW\\settings.json`.
- The settings control currently uses left-positioned WPF tabs for Behavior, Radial menu, Preview, a hidden Keybinds tab, and Theme.
- Trigger rebinding uses `GlobalHotkey.BeginCapture`; Esc cancels and reserved OS keys are rejected.
- Keybinds remain part of the persisted model and are honored by the resident runtime, even though their existing UI is currently hidden.
- Radial settings include enabled state, cursor interaction, outer radius, inner radius, and color tokens.
- Preview settings include enabled state, padding, corner radius, border width, and border color.
- Theme settings currently expose accent, radial sector fill/stroke, ring fill, and preview border values as raw text fields.
- Existing visual defaults are dark surfaces with blue accent values and semi-transparent color tokens.
- The radial overlay itself must never contain text. This invariant applies to the global-hotkey activation path and all other activation paths; the settings UI may explain radial actions in its own surface.
- The app must remain multi-DPI aware and usable on mixed-DPI Windows displays.

The redesign must not accidentally change window action behavior, global hotkey behavior, tray lifecycle, radial overlay behavior, or settings persistence beyond what is needed to present the settings more clearly.

## 4. Goals

1. Make the settings UI feel intentionally designed as one coherent product surface.
2. Improve first-glance hierarchy so users immediately understand where to configure the trigger and launch behavior.
3. Make every existing setting discoverable without forcing users through awkward or ambiguous navigation.
4. Preserve fast editing and auto-save for ordinary settings.
5. Give advanced users a capable home for keybinds and visual customization without making the everyday path dense.
6. Establish reusable visual tokens and control states so future settings can be added consistently.
7. Make the window feel native to Windows while still carrying a distinct LoopW identity.
8. Support dark, Windows-following, and explicit user-selected appearance modes.
9. Provide clear inline feedback for saving, validation, capture, reset, and failure states.
10. Keep the surface practical at the current desktop window scale and across Windows display scaling.

## 5. Non-goals

- Do not redesign the radial overlay itself as part of this work.
- Do not add text labels to the radial overlay.
- Do not change the action catalog, placement algorithms, global hook, tray lifecycle, IPC, or keybind execution semantics.
- Do not turn settings into a dashboard or marketing page.
- Do not introduce a web-based UI framework or third-party UI dependency without a separate decision.
- Do not require a full light theme as the default design deliverable; the standard product appearance remains dark.
- Do not remove raw theme customization; it should be available as an advanced option.
- Do not bury trigger rebinding, launch-at-login, keybinds, or reset/recovery behind unexplained overflow menus.

## 6. Proposed information architecture

Use a persistent left sidebar and a single active content pane. The sidebar should remain visible while editing and should make the current location obvious.

Recommended primary sections:

1. **Behavior**
   - Trigger key
   - Launch at login
   - Any existing resident/runtime behavior that is already represented by settings
2. **Radial menu**
   - Enable/disable radial menu
   - Cursor-direction interaction
   - Outer radius
   - Inner radius
   - Brief explanation of how the radial surface works
3. **Preview**
   - Enable/disable target preview
   - Preview padding
   - Corner radius
   - Border width
   - Preview border appearance, if the design makes it useful to expose here as a linked visual setting
4. **Appearance**
   - App appearance mode: LoopW dark, follow Windows, or explicit light/dark choice as supported by the implementation
   - Curated visual presets
   - Accent and radial/preview visual treatment
   - Advanced custom color editor in the same section
5. **Advanced**
   - Keybind list: add, rebind, action selection, cycle behavior, delete
   - Raw theme token editing if it is too dense for the default Appearance view; the entry point must remain clear and remain within the broader appearance/advanced model
   - Per-section reset controls where they are most useful
   - Global reset-all action, visually separated from routine controls

The exact labels may be adjusted during implementation if the final wording remains crisp and unambiguous. The intent is to make the everyday sections easy to understand while keeping power-user controls available in a clearly named Advanced area.

### Sidebar requirements

- Use a compact LoopW identity/header treatment, not a large hero.
- Show section name and optionally a restrained one-line descriptor.
- Use one clear active indicator: a filled tonal pill, slim accent rail, or similarly quiet treatment—not multiple competing indicators.
- Keep active, hover, keyboard-focus, and disabled states distinct.
- Sidebar labels should remain readable at normal Windows text scaling.
- The sidebar should not behave like the current tab strip with nested all-caps labels that make scanning harder.
- If the available width becomes insufficient, preserve the firm minimum window width rather than compressing labels into an unusable state.

### Content pane requirements

- Every section begins with a concise title and one-sentence explanation.
- Group related settings into cards or rows with consistent alignment, spacing, and control placement.
- Prefer a standard row pattern: label and short description on the left, control on the right where space allows.
- Use full-width rows for sliders, keybind tables, and custom color editing where the control needs room.
- Avoid nesting cards inside cards without a clear hierarchy.
- Keep section-level reset near the section heading or footer; never mix it visually with destructive global reset.
- Allow the active content area to scroll when needed; keep navigation and global footer treatment stable.

## 7. Visual direction

### Overall mood

Quiet, premium, and technical without being cold. The surface should feel composed and deliberate: dark foundation, layered charcoal/slate surfaces, a more refined blue accent, and restrained use of color for meaning.

### Color system

Create a small, reusable token system rather than scattering literal colors through XAML. The final values can be tuned during implementation, but the relationships should follow this model:

- **App background:** deepest neutral blue-black; should not read as pure black.
- **Navigation surface:** subtly distinct from the page background.
- **Content surface/card:** one or two tonal steps above the background.
- **Input surface:** slightly recessed, with enough contrast from cards to show editability.
- **Primary text:** high-contrast cool off-white.
- **Secondary text:** readable slate/blue-gray; not low-contrast gray.
- **Tertiary/quiet text:** reserved for metadata and supporting hints only.
- **Border/divider:** low-contrast cool neutral used sparingly.
- **Accent:** evolved LoopW blue; use it for active navigation, focus, primary actions, selected controls, and meaningful emphasis.
- **Accent soft:** translucent or low-contrast accent wash for active backgrounds and selected states.
- **Success/saved:** distinct but quiet positive color; do not rely on color alone.
- **Warning/error:** readable semantic colors with text/status support.

Color tokens need to work in dark mode, Windows-following mode, and explicit light mode if supported. User-customized theme colors must not make labels, focus rings, or controls unreadable; apply validation/contrast safeguards or a safe fallback.

### Depth and shape

- Use subtle elevation through tonal contrast, not dramatic shadows.
- Use consistent corner radii across cards, inputs, buttons, and navigation states.
- Prefer moderate radii; avoid both sharp legacy rectangles and excessive pill-shaped UI.
- Use borders only where they clarify structure or focus.
- Avoid glass blur, ornamental gradients, oversized glow, and decorative background art.
- Keep the visual rhythm spacious but not wasteful.

### Typography

- Use the existing system/native font stack unless a font decision is made separately.
- Establish a clear hierarchy: page/section title, supporting description, group label, setting label, and helper/status copy.
- Favor sentence case over all-caps for user-facing labels. Small uppercase eyebrow labels may be used sparingly for metadata, but should not carry essential meaning.
- Keep copy direct: explain what the setting does and what will happen, not implementation details.
- Avoid jargon such as “token” in the default user-facing UI; use “color” or “visual style.” If raw token terminology is needed for advanced users, explain it once.

## 8. Core interaction specifications

### Auto-save and change feedback

Retain automatic saving for ordinary changes. The footer or section-level status should communicate one of a small number of clear states:

- Saved
- Saving…
- Could not save
- Trigger updated
- Reset complete

Status should not flash excessively after every slider tick. For sliders, use a quiet value update while dragging and save with a debounced or coalesced update where practical. The user should always have confidence that the final value is persisted.

Use the mixed-by-risk rule:

- **Immediate auto-save:** visual settings, radial sizing, preview sizing, toggles that do not disrupt the current editing flow.
- **Inline confirmation or stronger feedback:** launch-at-login changes, trigger changes, keybind changes, reset-all, and any setting that may affect system integration or cause surprising global behavior.
- Avoid confirmation dialogs for every harmless interaction.
- If a system operation fails, restore the prior control value where possible and show a concise actionable error inline.

### Trigger rebinding

Use an inline capture state in the Behavior section:

1. User activates Rebind.
2. The trigger control changes to a clear listening state such as “Press a key…” and receives visible focus.
3. Supporting text explains: “Press a key or key combination. Esc cancels.”
4. Other conflicting edit actions are disabled or safely ignored while capture is active.
5. Successful capture updates the key chip, saves the setting, and shows a concise success state.
6. Esc cancels without changing the current trigger.
7. Reserved/invalid input leaves the current trigger unchanged and reports the reason inline.
8. The capture state must be visually obvious without relying on color alone.

### Keybind editor

Keybinds should be available in Advanced, not removed. The editor should be a readable list/table with consistent rows:

- Key combination chip or button.
- Action name in plain language.
- Cycle toggle where the selected action supports cycling.
- Delete action with a clear accessible name.
- Add keybind action at the end of the list.

Empty state should explain what keybinds do and provide a clear Add keybind action. Duplicate or conflicting combinations should be detected and explained before replacing or saving. Capture behavior should match trigger capture: inline, cancellable with Esc, and explicit about reserved keys.

### Sliders and numeric controls

- Show the current value with units, e.g. `91 px`.
- Keep labels and values aligned consistently.
- Preserve current valid ranges and constraints unless a separate product decision changes them.
- Make the inner-radius constraint legible when the outer radius changes; avoid silently producing an invalid or confusing state.
- Support keyboard adjustment and visible focus.
- Do not use a slider alone when precise entry is materially useful; consider a compact numeric value/editor for power users if it remains visually calm.

### Theme and appearance

The Appearance section should lead with curated presets rather than raw hex fields. Presets should be presented as small, coherent visual previews showing background, accent, selection, and preview treatment.

Required behavior:

- Selecting a preset applies it immediately and saves it.
- The current preset is clearly selected.
- A Custom option appears when values diverge from a preset.
- Advanced custom editing remains available in the same broad area, with color swatches and editable values.
- The custom editor should show a visual swatch next to each color value.
- Values should accept supported `#RRGGBB` and `#AARRGGBB` formats, with inline validation and safe fallback behavior.
- Invalid values must not make the UI unusable or silently corrupt other settings.
- If the implementation adds color pickers, the raw value must remain available for precision.

### Appearance mode

Provide an explicit setting with these conceptual options:

- LoopW dark (default/product standard)
- Follow Windows
- LoopW light or system light, depending on implementation support

If the design supports both explicit dark and explicit light, make the distinction clear. The dark mode must receive the primary craft and remain the reference visual system. Windows-following mode should update appropriately without breaking contrast or control states.

### Reset and recovery

Provide:

- **Reset section:** restores only the active section’s defaults.
- **Reset all:** restores all settings to defaults and is placed in a clearly separated recovery area in Advanced or the page footer.
- Reset-all requires a confirmation step that names the consequence.
- After reset, controls and dependent visual previews update immediately, persistence completes, and the status is explicit.
- Reset should not affect unrelated application files, tray state, or external windows.

## 9. Motion and feedback

Use polished transitions, not spectacle:

- Short, restrained transitions when changing sidebar sections or revealing content.
- Subtle hover/focus/selected transitions for navigation and controls.
- Clear but quiet status transitions for saving and errors.
- Avoid animating every card or control on every load.
- Avoid motion that delays access to settings.
- Honor Windows reduced-motion preferences where available; provide an implementation path that disables or minimizes nonessential transitions.
- Focus movement must remain immediate and predictable even when visual transitions are active.

## 10. Accessibility and quality requirements

The selected quality bar prioritizes visual polish plus keyboard usability and contrast. The following are release requirements:

### Keyboard

- All navigation items and controls are reachable by keyboard.
- Tab order follows the visual order within the sidebar and active content pane.
- Enter/Space activates buttons, toggles, navigation, and selection controls correctly.
- Sliders support standard keyboard adjustments.
- Rebind/keybind capture clearly handles Esc cancellation.
- Focus never becomes visually lost when switching sections.

### Focus and contrast

- Every interactive element has a clear, non-color-only focus treatment.
- Text and controls remain readable against their backgrounds in dark and supported alternate modes.
- Disabled controls remain distinguishable without becoming unreadably faint.
- User-selected accent/custom colors must not remove focus visibility.
- Do not communicate saved/error/selected state through color alone; pair it with text, iconography, shape, or position.

### Windows scaling

- Layout remains usable at common 100%, 125%, 150%, and 200% scaling.
- Text should not clip or overlap at common Windows font-size settings.
- The window remains practical on a normal laptop/desktop display.
- Keep the firm minimum size sufficient for sidebar labels and the primary content rows.

### Baseline semantics

- Use meaningful control labels and tooltips where a visual-only affordance would be ambiguous.
- Delete, reset, rebind, and status controls need explicit accessible names.
- Screen-reader support should not be regressed from standard WPF control behavior, even though the primary acceptance emphasis is keyboard and contrast.

## 11. Window and layout constraints

- Continue using the native WPF `Window`/`UserControl` architecture unless implementation experience shows a small structural change is necessary.
- Keep the settings surface inside the existing main window and preserve tray activation behavior.
- Prefer a desktop-first minimum size that preserves the intended left-sidebar/content composition.
- Do not force the user into a narrow single-column layout. If the window is resized near its minimum, active content should scroll rather than become cramped.
- Support maximized or larger desktop use gracefully by allowing content to breathe without stretching controls excessively.
- Keep the navigation stable while the content pane scrolls.
- Avoid a second independent settings window unless a later product decision explicitly asks for it.

## 12. Implementation guidance

This is a visual and interaction redesign, not a data-model rewrite.

Likely implementation areas:

- `SettingsWindow.xaml`: replace the current tab-control presentation with the sidebar/content layout and shared styles/templates.
- `SettingsWindow.xaml.cs`: preserve existing event wiring and persistence; add section navigation, reset flows, appearance mode/preset state, and improved capture/status states as needed.
- `MainWindow.xaml`: retain `SettingsHost` hosting and ensure the window dimensions/minimums support the new composition.
- `MainWindow.xaml.cs`: preserve tray/show/hide behavior and settings propagation.
- `Settings.cs`: extend only if appearance mode/preset metadata needs persistence; preserve backward compatibility and normalization for existing settings files.
- `App.xaml` or a shared resource location: consider moving stable visual tokens/templates to a reusable resource dictionary if that fits current project conventions.

Do not add a new UI library unless the repository first adopts it intentionally. Match the project’s existing .NET 8/WPF conventions and keep changes focused.

Important preservation rules:

- Existing settings files must continue loading safely.
- Existing values must not reset simply because the new UI is opened.
- Existing keybinds must remain active even if the Advanced section is not visited.
- Existing trigger capture and global hook safety behavior must remain intact.
- Existing settings-change propagation to the main/radial/preview surfaces must remain intact.
- The radial overlay’s no-text invariant must remain untouched.

## 13. Acceptance criteria

### Visual and structural

- [ ] The settings surface reads as one coherent premium Windows utility UI rather than a set of unrelated tabs.
- [ ] A persistent left sidebar clearly identifies Behavior, Radial menu, Preview, Appearance, and Advanced.
- [ ] Behavior is the default active section and trigger/launch settings are immediately prominent.
- [ ] Surface hierarchy uses subtle depth, restrained borders, consistent radii, and evolved dark/blue tokens.
- [ ] User-facing copy is concise, direct, and primarily sentence case.
- [ ] No oversized hero, decorative dashboard content, excessive glow, or dense technical wall of controls is present.

### Functional

- [ ] All existing settings remain available and continue to save correctly.
- [ ] Trigger rebinding works inline, supports Esc cancellation, and reports reserved-key failures without changing the old value.
- [ ] Keybind add/rebind/delete/action/cycle workflows remain available in Advanced.
- [ ] Radial and Preview settings retain their existing ranges and dependency behavior.
- [ ] Appearance presets apply coherently and expose a custom editing path.
- [ ] Raw color editing has swatches, validation, and safe fallback behavior.
- [ ] Dark mode is the default; Windows-following and explicit user mode choices behave as specified by the final implementation.
- [ ] Each section supports reset, and reset-all is separated and confirmed.
- [ ] Auto-save feedback is clear and does not produce noisy status flicker.
- [ ] Failure to update launch-at-login or save settings leaves the UI in a truthful, recoverable state.

### Accessibility and scaling

- [ ] All interactive controls are keyboard reachable and have a visible focus state.
- [ ] Keyboard order is logical across navigation and content.
- [ ] Contrast remains readable in supported appearance modes and with safe/custom accent values.
- [ ] The UI is usable at 100%, 125%, 150%, and 200% Windows scaling.
- [ ] Important state is not conveyed by color alone.
- [ ] Nonessential motion can be reduced or disabled.

### Regression protection

- [ ] Tray open/settings activation still shows the same main settings surface.
- [ ] Closing the window still hides to tray rather than exiting.
- [ ] The radial overlay remains text-free.
- [ ] Global trigger and keybind runtime behavior are unchanged.
- [ ] Existing settings JSON files load without data loss.

## 14. QA scenarios

Before considering the redesign complete, manually verify at minimum:

1. Open settings from a hidden resident/tray state and confirm Behavior is the default active section.
2. Navigate every sidebar section with mouse and keyboard.
3. Resize to the minimum supported size; confirm no labels clip and content scrolls correctly.
4. Test at 100%, 125%, 150%, and 200% Windows scaling.
5. Rebind the trigger successfully, cancel with Esc, and attempt a reserved key.
6. Add, edit, cycle, and delete a keybind; verify it still works after leaving settings.
7. Change radial and preview values and confirm the live app surfaces update.
8. Select each appearance preset, switch to custom editing, enter valid values, and enter invalid values.
9. Switch between LoopW dark, Follow Windows, and supported explicit mode choices; inspect contrast and focus states.
10. Reset one section and verify unrelated sections remain unchanged.
11. Use reset-all and verify all settings return to defaults after confirmation.
12. Force or simulate a launch-at-login/save failure and verify the control reverts or reports the failure truthfully.
13. Verify keyboard-only operation, focus visibility, Esc behavior, and slider adjustment.
14. Confirm settings changes persist across restart.
15. Open the radial overlay after all appearance changes and confirm it remains text-free and functional.

## 15. Open implementation decisions

These should be resolved during implementation, not guessed silently:

- Exact final window minimum size and default size after the sidebar layout is implemented.
- Whether Appearance or Advanced owns raw color fields when both are present; the user requirement is that custom editing remains in the same broad visual-settings area and is not removed.
- Whether explicit light mode is implemented as a complete bespoke theme or as a Windows-following/system-light variant.
- Exact curated preset names, values, and number of presets.
- Whether numeric text entry is needed alongside each slider after testing precision and layout.
- Whether reset confirmation uses a native dialog or an inline confirmation region.
- Exact semantic colors and contrast-safe fallback rules for custom accents.
- Exact transition durations and reduced-motion detection approach.

## 16. Definition of done

The redesign is ready for implementation sign-off when the UI has a clear visual system, the left-sidebar navigation and Behavior-first hierarchy are established, all existing settings remain available and persistent, the advanced workflows are not hidden or degraded, reset/error/capture states are specified and testable, and the layout passes keyboard, contrast, DPI, and tray/runtime regression checks without changing LoopW’s core window-management behavior.
