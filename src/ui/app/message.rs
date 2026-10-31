use iced::{keyboard, window};

/// Left-nav sections (General-first hierarchy from the spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Radial,
    Preview,
    Appearance,
    Advanced,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::General,
        Section::Radial,
        Section::Preview,
        Section::Appearance,
        Section::Advanced,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Radial => "Radial menu",
            Section::Preview => "Preview",
            Section::Appearance => "Appearance",
            Section::Advanced => "Advanced",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Section::General => "Trigger and launch",
            Section::Radial => "Menu shape and slots",
            Section::Preview => "Target preview and snapping",
            Section::Appearance => "Theme, motion, and colors",
            Section::Advanced => "Keybinds and recovery",
        }
    }
}

/// All UI events.
#[derive(Debug, Clone)]
pub enum Message {
    SelectSection(Section),
    BeginTriggerCapture,
    CancelTriggerCapture,
    BeginKeybindCapture(String),
    SetModifierSide(String),
    ToggleKeybindCycle(String),
    ToggleKeybindBypass(String),
    SetLaunchAtLogin(bool),
    NudgeDelay(i32),
    NudgeTimeout(i32),
    SetDoubleClick(bool),
    SetMiddleClick(bool),
    SetRadialEnabled(bool),
    SetCursorInteraction(bool),
    SetOuterRadius(f32),
    SetInnerRadius(f32),
    SetRadialTarget(Option<usize>, String),
    ToggleRadialCycle(Option<usize>),
    ClearWedge(usize),
    SetPreviewEnabled(bool),
    SetDragSnap(bool),
    SetPreviewPadding(f32),
    SetPreviewRadius(f32),
    SetPreviewBorder(f32),
    SetSnapThreshold(f32),
    SetRestoreOnCancel(bool),
    SetStashPersistence(bool),
    NudgeStashPeek(i32),
    NudgeStashHitZone(i32),
    NudgeStashDelay(i32),
    SetAppearanceMode(String),
    SetMotionPreference(String),
    ApplyPreset(String),
    EditAccent(String),
    EditSectorFill(String),
    EditSectorStroke(String),
    EditRingFill(String),
    EditPreviewBorderColor(String),
    AddKeybind,
    DeleteKeybind(String),
    SetKeybindAction(String, String),
    SetMonitorPolicy(String),
    NudgeGlobalPadding(i32),
    NudgePaddingLeft(i32),
    NudgePaddingTop(i32),
    NudgePaddingRight(i32),
    NudgePaddingBottom(i32),
    EditExcludedExecutables(String),
    EditExcludedProcesses(String),
    ResetSection(Section),
    ConfirmResetAll,
    CancelResetAll,
    ResetAll,
    ClearDiagnostics,
    // Resident-runtime messages.
    FrameTick,
    MainOpened(window::Id),
    RadialOpened(window::Id),
    PreviewOpened(window::Id),
    TrayMenuOpened(window::Id),
    WindowClosed(window::Id),
    WindowUnfocused(window::Id),
    TrayMenuSettings,
    TrayMenuQuit,
    OverlayKey(keyboard::Key),
    OverlayCommit,
    RunStartupCommand,
}
