# LoopW Settings UI Redesign Specification

**Status:** WPF UI implementation direction recorded; functional and visual QA scenarios retained
**Date:** 2026-08-16
**Scope:** Settings surface redesign plus integration of the advanced trigger/radial settings already introduced by `origin/main`; preserve all existing behavior and persistence contracts.

## 1. Summary

Redesign the LoopW settings experience into a modern, polished, premium, and coherent native Windows settings surface. The redesign should feel like a carefully crafted desktop utility rather than a collection of styled controls: quiet, confident, highly scannable, and precise.

The product should retain its existing dark visual identity and blue-accent lineage, but let WPF UI 4.3.0 provide the Fluent palette, control templates, navigation states, title-bar controls, and density foundation. The redesign should reorganize the current settings lightly rather than replace the information architecture wholesale. It should preserve all currently supported settings and their existing persistence semantics while improving hierarchy, navigation, feedback, accessibility, and visual coherence. LoopW-specific XAML is limited to the shared, text-free radial and window-preview surfaces plus the persisted appearance values and geometry behavior those surfaces consume.

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
- The settings control is hosted as a `UserControl` inside `MainWindow.SettingsHost`; the redesign uses a persistent WPF UI navigation pane for General, Radial menu, Preview, Appearance, and Advanced. General is the user-facing name for the existing behavior group. The visible settings surface does not add a resident-status badge.
- Trigger rebinding uses `GlobalHotkey.BeginCapture`; Esc cancels and reserved OS keys are rejected.
- Keybinds remain part of the persisted model and are honored by the resident runtime; the Advanced section exposes add, rebind, action, cycle, bypass, and delete controls.
- Radial settings include enabled state, cursor interaction, outer radius, inner radius, per-wedge targets, center target, cycle behavior, and color tokens.
- Trigger behavior includes modifier-side selection, activation delay, activation timeout, optional double-click activation, and optional middle-mouse activation.
- The Radial menu page puts the large, text-free shared radial surface first; Interaction and Ring geometry follow below it. The surface uses the same `RadialMenuSurface` component as the hotkey overlay, so it is quiet at rest and only the hovered sector uses the configured sector fill/stroke.
- Preview settings include enabled state, padding, corner radius, border width, and border color.
- The Preview page puts the large, text-free shared window-preview surface first; its geometry settings follow below it.
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

- Do not change the radial overlay’s interaction model or add settings-only visual behavior as part of this work; its renderer may be shared with the settings surface.
- Do not add text labels to the radial overlay.
- Do not regress the action catalog, placement algorithms, global hook, tray lifecycle, IPC, or keybind execution semantics. New trigger and radial configuration options documented in section 17 are explicitly supported integration scope.
- Do not turn settings into a dashboard or marketing page.
- Do not introduce a web-based UI framework. The explicit WPF UI adoption decision in the implementation guidance is the approved native control-library exception for this redesign.
- Do not require a full light theme as the default design deliverable; the standard product appearance remains dark.
- Do not remove raw theme customization; it should be available as an advanced option.
- Do not bury trigger rebinding, launch-at-login, keybinds, or reset/recovery behind unexplained overflow menus.

## 6. Proposed information architecture

Use a persistent left sidebar and a single active content pane. The sidebar should remain visible while editing and should make the current location obvious.

Recommended primary sections:

1. **General**
   - Trigger and launch behavior
   - Trigger key
   - Launch at login
   - Any existing resident/runtime behavior that is already represented by settings
2. **Radial menu**
   - Enable/disable radial menu
   - Cursor-direction interaction
   - Outer radius
   - Inner radius
   - Wedge and center action assignments
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
   - Keybind list: add, rebind, action selection, cycle behavior, bypass-trigger behavior, delete
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

- The sidebar is the primary section label. The active content pane begins with a concise one-sentence explanation and section reset action, without repeating the selected navigation label as a second page title.
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

Prefer WPF UI’s application resources and semantic brushes rather than introducing a second token system or scattering literal colors through XAML. Persisted custom colors are data for the shared overlay surfaces, not a replacement for WPF UI’s control palette. The relationships should follow this model:

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

Use an inline capture state in the General section:

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
- Optional Bypass toggle for bindings that should run without holding the trigger.
- Delete action with a clear accessible name.
- Add keybind action at the end of the list.

Empty state should explain what keybinds do and provide a clear Add keybind action. Duplicate or conflicting combinations should be detected and explained before replacing or saving. Capture behavior should match trigger capture: inline, cancellable with Esc, and explicit about reserved keys.

### Trigger behavior options

- Modifier side selection applies to the configured trigger only; `Either` preserves the legacy behavior.
- Activation delay postpones opening the radial surface until the trigger remains held for the selected interval.
- Activation timeout closes an active trigger without committing an action; `Off` preserves an unlimited hold.
- Double-click activation requires two trigger presses within the normal Windows double-click interval.
- Middle-mouse activation uses the middle button as an alternate hold gesture when enabled.
- Changing any trigger behavior cancels active trigger state so the global hook cannot remain logically stuck.

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

- Continue using the native WPF `UserControl` architecture inside a WPF UI `FluentWindow`; avoid an independent settings window.
- Keep the settings surface inside the existing main window and preserve tray activation behavior.
- Prefer a desktop-first minimum size that preserves the intended left-sidebar/content composition.
- Do not force the user into a narrow single-column layout. If the window is resized near its minimum, active content should scroll rather than become cramped.
- Support maximized or larger desktop use gracefully by allowing content to breathe without stretching controls excessively.
- Keep the navigation stable while the content pane scrolls.
- Avoid a second independent settings window unless a later product decision explicitly asks for it.

## 12. Implementation guidance

This is a visual and interaction redesign, not a data-model rewrite.

Likely implementation areas:

- `SettingsWindow.xaml`: use WPF UI navigation, cards, Fluent buttons, toggles, icons, number inputs, snackbar feedback, and a content-dialog host while preserving the existing named control/event contract where practical. Place the shared `RadialMenuSurface` and `WindowPreviewSurface` in full-bleed WPF UI cards before their related settings.
- `SettingsWindow.xaml.cs`: preserve existing event wiring and persistence; add section navigation, reset flows, appearance mode/preset state, and improved capture/status states as needed.
- `MainWindow.xaml`: retain `SettingsHost` hosting, the blank same-surface WPF UI title bar, and dimensions/minimums that support the new composition.
- `MainWindow.xaml.cs`: preserve tray/show/hide behavior and settings propagation.
- `Settings.cs`: extend only if appearance mode/preset metadata needs persistence; preserve backward compatibility and normalization for existing settings files.
- `App.xaml`: load WPF UI's theme and control dictionaries once at application scope. Do not create a second application-wide theme dictionary unless a concrete control gap requires it.

### WPF UI adoption decision

LoopW intentionally adopts `WPF-UI` 4.3.0 and `WPF-UI.Tray` 4.3.0. NuGet lists both packages as compatible with the project’s `net8.0-windows` target. WPF UI is the foundation for the Fluent control templates, theme dictionaries, navigation, cards, buttons, toggles, icons, number inputs, snackbar feedback, content dialogs, and tray icon/menu integration.

Use WPF UI directly for:

- `ThemesDictionary` and `ControlsDictionary` loaded from `App.xaml`.
- `NavigationView` and `NavigationViewItem` for the persistent left navigation.
- `TitleBar`, `Card`, `Button`, `ToggleSwitch`, `NumberBox`, `SymbolIcon`, `SnackbarPresenter`, and `ContentDialogHost` in the settings surface.
- `ContentDialogService` for reset-all confirmation and `SnackbarService` for higher-impact change feedback.
- `Wpf.Ui.Tray.Controls.NotifyIcon` for the resident tray icon and its WPF context menu.

LoopW-specific behavior remains intentionally small and is implemented in the existing settings code-behind and shared surface controls rather than through a parallel theme system:

- WPF UI owns the palette, Fluent control templates, navigation states, title-bar buttons, focus treatment, semantic feedback colors, and normal spacing/density. No application-wide LoopW style dictionary is required.
- `MainWindow` is a WPF UI `FluentWindow` with a WPF UI `TitleBar`; its title and icon are intentionally blank and its background uses the same WPF UI application surface as the content. Existing closing/hide-to-tray behavior remains in code-behind.
- WPF UI 4.3.0 `NavigationView` does not expose an arbitrary XAML `Content` property. The pane therefore occupies a dedicated left grid column and the existing settings content occupies a sibling right column. This keeps the Fluent navigation pane stable instead of allowing its frame surface to cover the settings content.
- Persisted LoopW appearance presets and custom colors remain data values because they drive the product's radial and preview overlays; the settings surface applies them through WPF UI's accent API and passes the same values to the shared text-free surfaces.
- `RadialMenuSurface` owns the runtime/settings radial geometry, backdrop blur, sector hover animation, and ring appearance. `WindowPreviewSurface` owns the runtime/settings glass surface, blur, corner radius, border, tint, and appearance treatment. They are reused directly instead of maintaining settings-only mockups.
- The radial and window-preview surfaces are intentionally placed before their related settings controls and remain text-free. Their settings instances may use different host sizing or pointer input, but not a different rendering implementation.

Keep the radial overlay, preview overlay, global hotkey capture, and window-management services outside WPF UI. WPF UI supplies the window/settings foundation without requiring an MVVM rewrite or a replacement of LoopW’s distinctive overlays.

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
- [ ] A persistent left sidebar clearly identifies General, Radial menu, Preview, Appearance, and Advanced.
- [ ] General is the default active section and trigger/launch settings are immediately prominent.
- [ ] Surface hierarchy uses WPF UI's semantic brushes and Fluent density with only the small persisted accent/preview color treatment LoopW needs.
- [ ] User-facing copy is concise, direct, and primarily sentence case.
- [ ] No oversized hero, decorative dashboard content, excessive glow, or dense technical wall of controls is present.

### Functional

- [ ] All existing settings remain available and continue to save correctly.
- [ ] Trigger rebinding works inline, supports Esc cancellation, and reports reserved-key failures without changing the old value.
- [ ] Keybind add/rebind/delete/action/cycle/bypass workflows remain available in Advanced.
- [ ] Trigger side, delay, timeout, double-click, and middle-mouse options persist and update the global hook.
- [ ] Radial wedge and center assignments persist, resolve safely, and drive the runtime overlay.
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

1. Open settings from a hidden resident/tray state and confirm General is the default active section.
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

## 15. Resolved implementation decisions

The implementation resolves the former open questions as follows:

- The existing `MainWindow` remains the settings host at an initial approximately 1166x779 size with a firm approximately 1100x760 minimum. The content pane scrolls at that minimum rather than collapsing the sidebar.
- Raw theme token fields remain in the Appearance section inside a clearly labeled advanced expander, keeping customization in the same visual-settings area.
- Dark is the standard LoopW mode. Follow Windows and explicit Light use WPF UI’s theme resources; the effective theme and persisted accent are applied through WPF UI’s appearance APIs whenever settings are applied.
- The existing LoopWBlue, LoopWCobalt, and LoopWViolet curated presets remain the named, persisted choices.
- WPF UI `NumberBox` controls sit beside the existing sliders for precise radius and preview-value entry; both paths share the same validation and persistence helpers.
- Global reset uses a WPF UI `ContentDialog` confirmation. Per-section reset remains an immediate, explicit action with status feedback.
- Custom colors continue through `Settings.Normalize()` as persisted data for the shared overlay surfaces. Invalid values fall back safely, while the WPF UI accent and settings surfaces remain synchronized.
- The WPF UI `TitleBar` is intentionally blank and uses the application background, while its standard caption buttons preserve normal utility-window behavior.
- The Radial menu and Preview pages lead with their large shared surfaces. Radial preview highlighting is pointer-driven: no sector is emphasized at rest and only the hovered sector receives the configured sector fill/stroke. The settings and hotkey activation paths use the same rendering controls, so appearance and geometry changes are visible consistently in both places.
- The WPF UI `NavigationView` remains in a dedicated left column beside the scrollable settings content so its pane cannot disappear behind or cover the content surface.
- Motion stays restrained and does not add a load animation or alter the radial/preview overlay transitions. Native WPF UI focus, hover, and selected states provide the interaction feedback needed for this utility surface.

## 16. Definition of done

The redesign is ready for implementation sign-off when the UI has a clear visual system, the left-sidebar navigation and General-first hierarchy are established, all existing settings remain available and persistent, the advanced workflows are not hidden or degraded, reset/error/capture states are specified and testable, and the layout passes keyboard, contrast, DPI, and tray/runtime regression checks without changing LoopW’s core window-management behavior.

## 17. Post-pull integration: advanced trigger and radial configuration

The current `origin/main` native-port work is integrated into the WPF UI direction rather than treated as a separate settings surface. These additions are now part of the supported settings contract:

- General exposes modifier side (`Either`, `Left`, or `Right`), activation delay, activation timeout, double-click activation, and middle-mouse activation.
- Advanced keybind rows expose the existing cycle behavior plus `Bypass`, which allows a binding to run without holding the trigger.
- Radial menu exposes per-wedge assignments, optional keybind targets, per-target cycle behavior, and a center action. Missing or invalid targets normalize to `No action` without invalidating the rest of the settings file.
- Existing JSON files remain compatible. New properties have safe defaults, stable keybind IDs are generated for older entries, and radial targets are normalized against the current action catalog and keybind list.

WPF UI remains the direct foundation for the settings presentation:

- `NavigationView`, `NavigationViewItem`, `Card`, `Button`, `ToggleSwitch`, `NumberBox`, `SymbolIcon`, `SnackbarPresenter`, `ContentDialogHost`, and the application theme/control dictionaries are used directly from WPF UI.
- `SettingsWindow.xaml.cs` remains code-behind based. It owns the small amount of synchronization needed to preserve the current settings model, auto-save behavior, capture workflows, reset behavior, and WPF UI feedback services.
- `RadialMenuSurface` is the one shared radial renderer for the settings preview and hotkey overlay. It owns geometry, backdrop treatment, hover state, selected-slot state, and center highlighting; the overlay supplies the resolved target mapping without duplicating the visual implementation.
- `WindowPreviewSurface` is the one shared window-preview renderer for the settings preview and runtime preview overlay. Both paths consume the same persisted padding, corner radius, border, color, and appearance values.
- LoopW-specific code is limited to target resolution, persisted appearance data, native trigger/window behavior, and the shared text-free surfaces. No second application-wide theme system or gallery-style control library layer is added.

The new trigger options preserve the existing safety requirements: capture still handles Esc cancellation and reserved keys, trigger delay/timeout state is reset when bindings change, and a timed-out or cancelled trigger dismisses the overlay without committing an action. The radial overlay remains text-free and uses resolved settings targets for both pointer and arrow-key selection.

Additional QA scenarios for this integration:

1. Set each modifier-side option and confirm only the selected side activates the trigger.
2. Adjust activation delay and timeout, confirm the status/overlay behavior, and verify timeout never commits an action.
3. Enable double-click and middle-mouse activation, then confirm release and cancellation leave no stuck trigger state.
4. Mark a keybind as Bypass and verify it works without the trigger while ordinary bindings still require the trigger.
5. Assign built-in actions, keybinds, and No action to radial wedges and the center; verify assignments persist and invalid/missing targets normalize safely.
6. Confirm the same radial hover/selection treatment and appearance values are visible in Settings and during actual hotkey use.
