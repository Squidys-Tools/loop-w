using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Brush = System.Windows.Media.Brush;
using Brushes = System.Windows.Media.Brushes;
using MessageBox = System.Windows.MessageBox;
using Button = System.Windows.Controls.Button;
using CheckBox = System.Windows.Controls.CheckBox;
using ComboBox = System.Windows.Controls.ComboBox;
using RadioButton = System.Windows.Controls.RadioButton;
using UserControl = System.Windows.Controls.UserControl;

namespace LoopW;

public partial class SettingsWindow : UserControl
{
    private static readonly ThemePreset[] ThemePresets =
    {
        new("LoopWBlue", "#3D9BFF", "#7A3D9BFF", "#F03D9BFF", "#B61B212B", "#B83D9BFF"),
        new("Cobalt", "#6AAEFF", "#7A6AAEFF", "#F06AAEFF", "#B61B2534", "#B86AAEFF"),
        new("Violet", "#B39AFF", "#7AB39AFF", "#F0B39AFF", "#B6252333", "#B8B39AFF")
    };

    public static IReadOnlyList<KeyValuePair<WindowAction, string>> ActionChoices { get; } =
        Enum.GetValues<WindowAction>()
            .Select(a => new KeyValuePair<WindowAction, string>(a, WindowActionService.ActionName(a)))
            .ToArray();

    private readonly GlobalHotkey _hotkey;
    private readonly AppSettings _settings;
    private readonly ObservableCollection<KeybindRow> _rows = new();
    private readonly ObservableCollection<RadialSlotRow> _radialRows = new();
    private bool _capturingUi;
    private bool _loading = true;
    private bool _initialized;

    public SettingsWindow(GlobalHotkey hotkey, AppSettings settings)
    {
        InitializeComponent();
        _hotkey = hotkey;
        _settings = settings;
        KeybindList.ItemsSource = _rows;
        RadialSlotList.ItemsSource = _radialRows;
    }

    public event Action<AppSettings>? SettingsChanged;

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        if (_initialized)
        {
            return;
        }

        _initialized = true;
        _loading = true;
        foreach (var keybind in _settings.Keybinds)
        {
            _rows.Add(new KeybindRow(keybind));
        }

        RefreshControlsFromSettings();
        _loading = false;
        ApplyThemeResources();
        ShowSection("Behavior");
    }

    private void SectionNav_Checked(object sender, RoutedEventArgs e)
    {
        if (sender is RadioButton { Tag: string section })
        {
            ShowSection(section);
        }
    }

    private void ShowSection(string section)
    {
        if (BehaviorSection == null || ContentScroller == null)
        {
            return;
        }

        _activeSection = section;
        BehaviorSection.Visibility = section == "Behavior" ? Visibility.Visible : Visibility.Collapsed;
        RadialSection.Visibility = section == "Radial" ? Visibility.Visible : Visibility.Collapsed;
        PreviewSection.Visibility = section == "Preview" ? Visibility.Visible : Visibility.Collapsed;
        AppearanceSection.Visibility = section == "Appearance" ? Visibility.Visible : Visibility.Collapsed;
        AdvancedSection.Visibility = section == "Advanced" ? Visibility.Visible : Visibility.Collapsed;
        ContentScroller.ScrollToTop();
    }

    private string _activeSection = "Behavior";

    private void Trigger_Click(object sender, RoutedEventArgs e) => BeginTriggerCapture();

    private void BeginTriggerCapture()
    {
        if (_capturingUi)
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                _settings.TriggerModifiers = mods;
                _settings.TriggerVk = vk;
                RefreshTriggerLabel();
                SaveSettings("Trigger updated");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("Trigger capture cancelled");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("That key is reserved by the OS — try another");
            });
    }

    private void Behavior_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading || LaunchAtLoginCheck.IsChecked is not bool enabled)
        {
            return;
        }

        if (!StartupManager.TrySetEnabled(enabled))
        {
            _loading = true;
            LaunchAtLoginCheck.IsChecked = _settings.LaunchAtLogin;
            _loading = false;
            SetStatus("Could not update launch-at-login", isError: true);
            return;
        }

        _settings.LaunchAtLogin = enabled;
        SaveSettings(enabled ? "Launch at login enabled" : "Launch at login disabled");
    }

    private void TriggerBehavior_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.DoubleClickToTrigger = DoubleClickTriggerCheck.IsChecked == true;
        _settings.MiddleClickToTrigger = MiddleClickTriggerCheck.IsChecked == true;
        SaveSettings("Trigger behavior saved");
    }

    private void TriggerBehavior_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading || TriggerDelaySlider is null || TriggerTimeoutSlider is null)
        {
            return;
        }

        _settings.TriggerDelayMilliseconds = (int)Math.Round(TriggerDelaySlider.Value);
        _settings.TriggerTimeoutMilliseconds = (int)Math.Round(TriggerTimeoutSlider.Value);
        UpdateTriggerBehaviorLabels();
        SaveSettings("Trigger timing saved");
    }

    private void TriggerSide_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loading || TriggerSideCombo.SelectedValue is not string value ||
            !Enum.TryParse<TriggerModifierSide>(value, out var side))
        {
            return;
        }

        _settings.TriggerModifierSide = side;
        SaveSettings("Trigger side saved");
    }

    private void Radial_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.RadialEnabled = RadialEnabledCheck.IsChecked == true;
        _settings.CursorInteractionEnabled = CursorInteractionCheck.IsChecked == true;
        SaveSettings("Radial settings saved");
    }

    private void Radial_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading || OuterRadiusSlider is null || InnerRadiusSlider is null)
        {
            return;
        }

        _settings.RadialOuterRadius = OuterRadiusSlider.Value;
        var innerRadius = Math.Min(InnerRadiusSlider.Value, _settings.RadialOuterRadius - 8);
        if (Math.Abs(InnerRadiusSlider.Value - innerRadius) > 0.001)
        {
            InnerRadiusSlider.Value = innerRadius;
        }

        _settings.RadialInnerRadius = innerRadius;
        UpdateValueLabels();
        SaveSettings("Radial size saved");
    }

    private void RadialSlot_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loading || sender is not ComboBox { DataContext: RadialSlotRow row, SelectedItem: RadialChoice choice })
        {
            return;
        }

        row.Select(choice);
        SaveSettings("Wedge assignment saved");
    }

    private void RadialSlotCycle_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading || sender is not CheckBox { DataContext: RadialSlotRow row, IsChecked: bool enabled })
        {
            return;
        }

        row.CycleEnabled = enabled;
        SaveSettings("Wedge cycle setting saved");
    }

    private void CenterAction_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loading || CenterActionCombo.SelectedItem is not RadialChoice choice)
        {
            return;
        }

        ApplyChoice(_settings.CenterTarget, choice);
        CenterCycleCheck.IsChecked = _settings.CenterTarget.CycleEnabled;
        CenterCycleCheck.IsEnabled = CanCycle(choice);
        SaveSettings("Center action saved");
    }

    private void CenterCycle_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading || CenterActionCombo.SelectedItem is not RadialChoice choice ||
            choice.Kind == RadialTargetKind.None)
        {
            return;
        }

        _settings.CenterTarget.CycleEnabled = CenterCycleCheck.IsChecked == true;
        CenterCycleCheck.IsEnabled = CanCycle(choice);
        SaveSettings("Center cycle setting saved");
    }

    private void Preview_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.PreviewEnabled = PreviewEnabledCheck.IsChecked == true;
        SaveSettings("Preview settings saved");
    }

    private void Preview_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading)
        {
            return;
        }

        _settings.PreviewPadding = PreviewPaddingSlider.Value;
        _settings.PreviewCornerRadius = PreviewCornerSlider.Value;
        _settings.PreviewBorderWidth = PreviewBorderWidthSlider.Value;
        UpdateValueLabels();
        SaveSettings("Preview size saved");
    }

    private void DragSnap_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.DragSnapEnabled = DragSnapEnabledCheck.IsChecked == true;
        _settings.RestorePreDragFrameOnSnapCancel = RestorePreDragFrameCheck.IsChecked == true;
        SaveSettings("Drag snapping settings saved");
    }

    private void DragSnap_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading)
        {
            return;
        }

        _settings.DragSnapThreshold = (int)Math.Round(DragSnapThresholdSlider.Value);
        UpdateValueLabels();
        SaveSettings("Snap threshold saved");
    }

    private void Stash_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.StashPersistenceEnabled = StashPersistenceCheck.IsChecked == true;
        SaveSettings("Stash settings saved");
    }

    private void Stash_ValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_loading)
        {
            return;
        }

        _settings.StashEdgePeek = (int)Math.Round(StashEdgePeekSlider.Value);
        _settings.StashHitZone = (int)Math.Round(StashHitZoneSlider.Value);
        _settings.StashRevealDelayMilliseconds = (int)Math.Round(StashRevealDelaySlider.Value);
        UpdateValueLabels();
        SaveSettings("Stash timing saved");
    }

    private void AppearanceMode_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loading || AppearanceModeCombo.SelectedValue is not string mode)
        {
            return;
        }

        _settings.AppearanceMode = mode;
        ApplyThemeResources();
        SaveSettings("Appearance saved");
    }

    private void Preset_Click(object sender, RoutedEventArgs e)
    {
        if (_loading || sender is not Button { Tag: string presetName })
        {
            return;
        }

        var preset = ThemePresets.FirstOrDefault(p => p.Name == presetName);
        if (preset.Name is null)
        {
            return;
        }

        _settings.AccentColor = preset.Accent;
        _settings.RadialSectorFill = preset.SectorFill;
        _settings.RadialSectorStroke = preset.SectorStroke;
        _settings.RadialRingFill = preset.RingFill;
        _settings.PreviewBorderColor = preset.PreviewBorder;
        RefreshThemeFields();
        ApplyThemeResources();
        SaveSettings($"{presetName} style applied");
    }

    private void Theme_TextChanged(object sender, TextChangedEventArgs e)
    {
        if (!_loading)
        {
            UpdateColorSwatches();
        }
    }

    private void Theme_LostFocus(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        _settings.AccentColor = AccentColorText.Text;
        _settings.RadialSectorFill = SectorFillText.Text;
        _settings.RadialSectorStroke = SectorStrokeText.Text;
        _settings.RadialRingFill = RingFillText.Text;
        _settings.PreviewBorderColor = PreviewBorderText.Text;
        _settings.Save();
        RefreshThemeFields();
        ApplyThemeResources();
        NotifySettingsChanged();
        SetStatus("Custom colors saved");
    }

    private void Add_Click(object sender, RoutedEventArgs e)
    {
        if (_capturingUi)
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                if (HasKeybindConflict(mods, vk))
                {
                    SetStatus("That key is already assigned — choose another", isError: true);
                    return;
                }

                _rows.Add(new KeybindRow(new Keybind(mods, vk, WindowAction.RightHalf)));
                SaveSettings("Keybind added");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("Keybind capture cancelled");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("That key is reserved by the OS — try another");
            });
    }

    private void KeyChip_Click(object sender, RoutedEventArgs e)
    {
        if (_capturingUi || sender is not Button { DataContext: KeybindRow row })
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                if (HasKeybindConflict(mods, vk, row.Keybind))
                {
                    SetStatus("That key is already assigned — choose another", isError: true);
                    return;
                }

                row.Keybind.Modifiers = mods;
                row.Keybind.Vk = vk;
                row.Refresh();
                SaveSettings("Keybind rebound");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("Rebind cancelled");
            },
            () =>
            {
                SetCapturingUi(false);
                SetStatus("That key is reserved by the OS — try another");
            });
    }

    private bool HasKeybindConflict(uint modifiers, uint vk, Keybind? ignored = null)
    {
        if (modifiers == _settings.TriggerModifiers && vk == _settings.TriggerVk)
        {
            return true;
        }

        return _rows.Any(row => row.Keybind != ignored &&
            row.Keybind.Modifiers == modifiers &&
            row.Keybind.Vk == vk);
    }

    private void Action_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        if (sender is ComboBox { DataContext: KeybindRow row } combo && combo.SelectedValue is WindowAction action)
        {
            row.Keybind.Action = action;
            row.Refresh();
            SaveSettings("Keybind action saved");
        }
    }

    private void Cycle_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        if (sender is CheckBox { DataContext: KeybindRow row, IsChecked: bool enabled })
        {
            row.Keybind.CycleEnabled = enabled;
            SaveSettings("Cycle setting saved");
        }
    }

    private void Bypass_Changed(object sender, RoutedEventArgs e)
    {
        if (_loading)
        {
            return;
        }

        if (sender is CheckBox { DataContext: KeybindRow row, IsChecked: bool enabled })
        {
            row.Keybind.BypassTrigger = enabled;
            SaveSettings("Trigger bypass saved");
        }
    }

    private void Delete_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: KeybindRow row })
        {
            _rows.Remove(row);
            SaveSettings("Keybind deleted");
        }
    }

    private void ResetSection_Click(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string section })
        {
            return;
        }

        var defaults = new AppSettings();
        switch (section)
        {
            case "Behavior":
                if (!StartupManager.TrySetEnabled(defaults.LaunchAtLogin))
                {
                    SetStatus("Could not reset launch-at-login", isError: true);
                    return;
                }

                _settings.TriggerModifiers = defaults.TriggerModifiers;
                _settings.TriggerVk = defaults.TriggerVk;
                _settings.TriggerModifierSide = defaults.TriggerModifierSide;
                _settings.TriggerDelayMilliseconds = defaults.TriggerDelayMilliseconds;
                _settings.TriggerTimeoutMilliseconds = defaults.TriggerTimeoutMilliseconds;
                _settings.DoubleClickToTrigger = defaults.DoubleClickToTrigger;
                _settings.MiddleClickToTrigger = defaults.MiddleClickToTrigger;
                _settings.LaunchAtLogin = defaults.LaunchAtLogin;
                break;
            case "Radial":
                _settings.RadialEnabled = defaults.RadialEnabled;
                _settings.CursorInteractionEnabled = defaults.CursorInteractionEnabled;
                _settings.RadialOuterRadius = defaults.RadialOuterRadius;
                _settings.RadialInnerRadius = defaults.RadialInnerRadius;
                _settings.RadialSlots = RadialConfiguration.CreateDefaultSlots();
                _settings.CenterTarget = RadialConfiguration.CreateDefaultCenter();
                break;
            case "Preview":
                _settings.PreviewEnabled = defaults.PreviewEnabled;
                _settings.PreviewPadding = defaults.PreviewPadding;
                _settings.PreviewCornerRadius = defaults.PreviewCornerRadius;
                _settings.PreviewBorderWidth = defaults.PreviewBorderWidth;
                _settings.DragSnapEnabled = defaults.DragSnapEnabled;
                _settings.DragSnapThreshold = defaults.DragSnapThreshold;
                _settings.RestorePreDragFrameOnSnapCancel = defaults.RestorePreDragFrameOnSnapCancel;
                _settings.StashPersistenceEnabled = defaults.StashPersistenceEnabled;
                _settings.StashEdgePeek = defaults.StashEdgePeek;
                _settings.StashHitZone = defaults.StashHitZone;
                _settings.StashRevealDelayMilliseconds = defaults.StashRevealDelayMilliseconds;
                break;
            case "Appearance":
                _settings.AppearanceMode = defaults.AppearanceMode;
                _settings.AccentColor = defaults.AccentColor;
                _settings.RadialSectorFill = defaults.RadialSectorFill;
                _settings.RadialSectorStroke = defaults.RadialSectorStroke;
                _settings.RadialRingFill = defaults.RadialRingFill;
                _settings.PreviewBorderColor = defaults.PreviewBorderColor;
                break;
            case "Advanced":
                _settings.Keybinds = new List<Keybind>();
                _rows.Clear();
                break;
            default:
                return;
        }

        RefreshControlsFromSettings();
        SaveSettings($"{section} reset");
    }

    private void ResetAll_Click(object sender, RoutedEventArgs e)
    {
        var result = MessageBox.Show(
            Window.GetWindow(this),
            "Reset all LoopW settings to their defaults? This will remove custom keybinds and visual choices.",
            "Reset all settings",
            MessageBoxButton.YesNo,
            MessageBoxImage.Warning,
            MessageBoxResult.No);

        if (result != MessageBoxResult.Yes)
        {
            return;
        }

        if (!StartupManager.TrySetEnabled(false))
        {
            SetStatus("Could not reset launch-at-login", isError: true);
            return;
        }

        _settings.ResetToDefaults();
        _rows.Clear();
        RefreshControlsFromSettings();
        SaveSettings("All settings reset");
    }

    private void SetCapturingUi(bool capturing)
    {
        _capturingUi = capturing;
        AddButton.IsEnabled = !capturing;
        TriggerButton.IsEnabled = !capturing;
        CaptureHint.Text = capturing ? "Press a key or key combination — Esc cancels." : string.Empty;
        TriggerLabel.Content = capturing
            ? "Press a key…"
            : HotkeyNames.For(_hotkey.TriggerModifiers, _hotkey.TriggerVk, _settings.TriggerModifierSide);
        SetStatus(capturing ? "Listening for a key…" : "Saved");
    }

    private void SaveSettings(string status)
    {
        _settings.Keybinds = _rows.Select(row => row.Keybind).ToList();
        _settings.Save();
        _hotkey.SetBinding(_settings.TriggerModifiers, _settings.TriggerVk);
        _hotkey.SetTriggerBehavior(
            _settings.TriggerModifierSide,
            _settings.TriggerDelayMilliseconds,
            _settings.TriggerTimeoutMilliseconds,
            _settings.DoubleClickToTrigger,
            _settings.MiddleClickToTrigger);
        _hotkey.SetKeybinds(_settings.Keybinds);
        ApplyThemeResources();
        RefreshRadialControls();
        NotifySettingsChanged();
        RefreshTriggerLabel();
        RefreshThemeFields();
        UpdateKeybindEmptyState();
        SetStatus(status);
    }

    private void NotifySettingsChanged() => SettingsChanged?.Invoke(_settings);

    private void RefreshControlsFromSettings()
    {
        _loading = true;
        RefreshTriggerLabel();
        DoubleClickTriggerCheck.IsChecked = _settings.DoubleClickToTrigger;
        MiddleClickTriggerCheck.IsChecked = _settings.MiddleClickToTrigger;
        TriggerSideCombo.SelectedValue = _settings.TriggerModifierSide.ToString();
        TriggerDelaySlider.Value = _settings.TriggerDelayMilliseconds;
        TriggerTimeoutSlider.Value = _settings.TriggerTimeoutMilliseconds;
        UpdateTriggerBehaviorLabels();
        LaunchAtLoginCheck.IsChecked = _settings.LaunchAtLogin;
        RadialEnabledCheck.IsChecked = _settings.RadialEnabled;
        CursorInteractionCheck.IsChecked = _settings.CursorInteractionEnabled;
        OuterRadiusSlider.Value = _settings.RadialOuterRadius;
        InnerRadiusSlider.Value = _settings.RadialInnerRadius;
        PreviewEnabledCheck.IsChecked = _settings.PreviewEnabled;
        PreviewPaddingSlider.Value = _settings.PreviewPadding;
        PreviewCornerSlider.Value = _settings.PreviewCornerRadius;
        PreviewBorderWidthSlider.Value = _settings.PreviewBorderWidth;
        DragSnapEnabledCheck.IsChecked = _settings.DragSnapEnabled;
        RestorePreDragFrameCheck.IsChecked = _settings.RestorePreDragFrameOnSnapCancel;
        DragSnapThresholdSlider.Value = _settings.DragSnapThreshold;
        StashPersistenceCheck.IsChecked = _settings.StashPersistenceEnabled;
        StashEdgePeekSlider.Value = _settings.StashEdgePeek;
        StashHitZoneSlider.Value = _settings.StashHitZone;
        StashRevealDelaySlider.Value = _settings.StashRevealDelayMilliseconds;
        AppearanceModeCombo.SelectedValue = _settings.AppearanceMode;
        RefreshRadialControls();
        RefreshThemeFields();
        UpdateValueLabels();
        UpdateKeybindEmptyState();
        _loading = false;
        ApplyThemeResources();
    }

    private void RefreshRadialControls()
    {
        var wasLoading = _loading;
        _loading = true;

        var choices = BuildRadialChoices();
        _radialRows.Clear();
        for (var i = 0; i < RadialConfiguration.SlotCount; i++)
        {
            var row = new RadialSlotRow(RadialActionCatalog.Geometry[i].Label, _settings.RadialSlots[i]);
            row.RefreshChoices(choices);
            _radialRows.Add(row);
        }

        CenterActionCombo.ItemsSource = choices;
        CenterActionCombo.SelectedItem = choices.First(choice => Matches(choice, _settings.CenterTarget));
        CenterCycleCheck.IsChecked = _settings.CenterTarget.CycleEnabled;
        CenterCycleCheck.IsEnabled = CenterActionCombo.SelectedItem is RadialChoice centerChoice && CanCycle(centerChoice);
        _loading = wasLoading;
    }

    private List<RadialChoice> BuildRadialChoices()
    {
        var choices = new List<RadialChoice>
        {
            new(RadialTargetKind.None, default, string.Empty, "No action", false)
        };
        choices.AddRange(ActionChoices.Select(choice =>
            new RadialChoice(
                RadialTargetKind.Action,
                choice.Key,
                string.Empty,
                choice.Value,
                WindowCycleService.CanCycle(choice.Key))));
        choices.AddRange(_rows.Select(row => new RadialChoice(
            RadialTargetKind.Keybind,
            row.Keybind.Action,
            row.Keybind.Id,
            $"{row.Display} · {WindowActionService.ActionName(row.Keybind.Action)}",
            row.Keybind.CycleEnabled)));
        return choices;
    }

    private static bool Matches(RadialChoice choice, RadialTargetSettings target) =>
        choice.Kind == target.Kind &&
        (choice.Kind == RadialTargetKind.None ||
         (choice.Kind == RadialTargetKind.Action && choice.Action == target.Action) ||
         (choice.Kind == RadialTargetKind.Keybind && choice.KeybindId == target.KeybindId));

    private static bool CanCycle(RadialChoice choice) =>
        choice.Kind != RadialTargetKind.None && WindowCycleService.CanCycle(choice.Action);

    private static void ApplyChoice(RadialTargetSettings target, RadialChoice choice)
    {
        target.Kind = choice.Kind;
        target.Action = choice.Action;
        target.KeybindId = choice.KeybindId;
        target.CycleEnabled = choice.Kind != RadialTargetKind.None && choice.DefaultCycle;
    }

    private void RefreshTriggerLabel()
    {
        if (TriggerLabel != null)
        {
            TriggerLabel.Content = HotkeyNames.For(
                _settings.TriggerModifiers,
                _settings.TriggerVk,
                _settings.TriggerModifierSide);
        }
    }

    private void UpdateTriggerBehaviorLabels()
    {
        if (TriggerDelayValue == null || TriggerTimeoutValue == null)
        {
            return;
        }

        TriggerDelayValue.Text = $"{TriggerDelaySlider.Value:0} ms";
        TriggerTimeoutValue.Text = TriggerTimeoutSlider.Value <= 0
            ? "Off"
            : $"{TriggerTimeoutSlider.Value:0} ms";
    }

    private void RefreshThemeFields()
    {
        if (AccentColorText == null)
        {
            return;
        }

        AccentColorText.Text = _settings.AccentColor;
        SectorFillText.Text = _settings.RadialSectorFill;
        SectorStrokeText.Text = _settings.RadialSectorStroke;
        RingFillText.Text = _settings.RadialRingFill;
        PreviewBorderText.Text = _settings.PreviewBorderColor;
        UpdateColorSwatches();
        UpdatePresetSelection();
    }

    private void UpdateValueLabels()
    {
        if (OuterRadiusSlider == null)
        {
            return;
        }

        OuterRadiusValue.Text = $"{OuterRadiusSlider.Value:0} px";
        InnerRadiusValue.Text = $"{InnerRadiusSlider.Value:0} px";
        PreviewPaddingValue.Text = $"{PreviewPaddingSlider.Value:0} px";
        PreviewCornerValue.Text = $"{PreviewCornerSlider.Value:0} px";
        PreviewBorderWidthValue.Text = $"{PreviewBorderWidthSlider.Value:0} px";
        DragSnapThresholdValue.Text = $"{DragSnapThresholdSlider.Value:0} px";
        StashEdgePeekValue.Text = $"{StashEdgePeekSlider.Value:0} px";
        StashHitZoneValue.Text = $"{StashHitZoneSlider.Value:0} px";
        StashRevealDelayValue.Text = StashRevealDelaySlider.Value <= 0
            ? "Instant"
            : $"{StashRevealDelaySlider.Value:0} ms";
    }

    private void UpdateKeybindEmptyState()
    {
        if (KeybindEmptyState == null)
        {
            return;
        }

        KeybindEmptyState.Visibility = _rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    private void UpdateColorSwatches()
    {
        SetSwatch(AccentSwatch, AccentColorText.Text);
        SetSwatch(SectorFillSwatch, SectorFillText.Text);
        SetSwatch(SectorStrokeSwatch, SectorStrokeText.Text);
        SetSwatch(RingFillSwatch, RingFillText.Text);
        SetSwatch(PreviewBorderSwatch, PreviewBorderText.Text);
    }

    private void SetSwatch(Border swatch, string value)
    {
        swatch.Background = TryCreateBrush(value) ?? (Brush)FindResource("BorderBrush");
    }

    private void UpdatePresetSelection()
    {
        if (BluePresetButton == null)
        {
            return;
        }

        var selected = ThemePresets.FirstOrDefault(p =>
            string.Equals(p.Accent, _settings.AccentColor, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(p.SectorFill, _settings.RadialSectorFill, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(p.SectorStroke, _settings.RadialSectorStroke, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(p.RingFill, _settings.RadialRingFill, StringComparison.OrdinalIgnoreCase) &&
            string.Equals(p.PreviewBorder, _settings.PreviewBorderColor, StringComparison.OrdinalIgnoreCase));

        var selectedName = selected.Name;
        var defaultBorder = (Brush)FindResource("BorderBrush");
        var accent = (Brush)FindResource("AccentBrush");
        foreach (var button in new[] { BluePresetButton, CobaltPresetButton, VioletPresetButton })
        {
            var isSelected = button.Tag is string name && name == selectedName;
            button.BorderBrush = isSelected ? accent : defaultBorder;
            button.BorderThickness = isSelected ? new Thickness(1.5) : new Thickness(1);
        }
    }

    private void ApplyThemeResources()
    {
        var light = _settings.IsLightAppearance;
        var accent = TryCreateBrush(_settings.AccentColor) ?? CreateBrush(light ? "#1769D1" : "#3D9BFF");
        Resources["PageBrush"] = CreateBrush(light ? "#F4F7FB" : "#101216");
        Resources["NavigationBrush"] = CreateBrush(light ? "#EAF0F7" : "#151922");
        Resources["PanelBrush"] = CreateBrush(light ? "#FFFFFF" : "#191E27");
        Resources["CardBrush"] = CreateBrush(light ? "#FFFFFF" : "#202632");
        Resources["CardHoverBrush"] = CreateBrush(light ? "#EEF4FA" : "#293342");
        Resources["InputBrush"] = CreateBrush(light ? "#F6F8FB" : "#151A22");
        Resources["BorderBrush"] = CreateBrush(light ? "#D7DFEA" : "#303A48");
        Resources["PrimaryTextBrush"] = CreateBrush(light ? "#16202D" : "#F0F4FA");
        Resources["SecondaryTextBrush"] = CreateBrush(light ? "#5E6B7C" : "#9DA9B9");
        Resources["QuietTextBrush"] = CreateBrush(light ? "#748092" : "#6F7B8A");
        Resources["AccentBrush"] = accent;
        Resources["AccentSoftBrush"] = CreateBrush(light ? "#1A3D9BFF" : "#263D9BFF");
        Resources["SuccessBrush"] = CreateBrush(light ? "#16734A" : "#6AD5A0");
        Resources["WarningBrush"] = CreateBrush(light ? "#9A5B00" : "#F1B86A");
        Resources["DangerBrush"] = CreateBrush(light ? "#B4232C" : "#FF8B91");
        Resources["FocusBrush"] = CreateBrush(light ? "#1769D1" : "#78B9FF");
        UpdatePresetSelection();
        UpdateColorSwatches();
    }

    private static Brush CreateBrush(string value)
    {
        return TryCreateBrush(value) ?? Brushes.Transparent;
    }

    private static Brush? TryCreateBrush(string? value)
    {
        try
        {
            if (new BrushConverter().ConvertFromString(value ?? string.Empty) is Brush brush)
            {
                brush.Freeze();
                return brush;
            }
        }
        catch
        {
            // Invalid values are normalized by AppSettings.Save().
        }

        return null;
    }

    private void SetStatus(string text, bool isError = false)
    {
        StatusText.Text = text;
        StatusText.Foreground = (Brush)FindResource(isError ? "DangerBrush" : "SecondaryTextBrush");
    }

    private readonly record struct ThemePreset(
        string Name,
        string Accent,
        string SectorFill,
        string SectorStroke,
        string RingFill,
        string PreviewBorder);
}

public sealed class KeybindRow : INotifyPropertyChanged
{
    public KeybindRow(Keybind keybind)
    {
        Keybind = keybind;
    }

    public Keybind Keybind { get; }

    public string Display => HotkeyNames.For(Keybind.Modifiers, Keybind.Vk);

    public bool CanCycle => WindowCycleService.CanCycle(Keybind.Action);

    public void Refresh()
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Display)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CanCycle)));
    }

    public event PropertyChangedEventHandler? PropertyChanged;
}

public sealed class RadialChoice
{
    public RadialChoice(
        RadialTargetKind kind,
        WindowAction action,
        string keybindId,
        string display,
        bool defaultCycle)
    {
        Kind = kind;
        Action = action;
        KeybindId = keybindId;
        Display = display;
        DefaultCycle = defaultCycle;
    }

    public RadialTargetKind Kind { get; }

    public WindowAction Action { get; }

    public string KeybindId { get; }

    public string Display { get; }

    public bool DefaultCycle { get; }
}

public sealed class RadialSlotRow : INotifyPropertyChanged
{
    private RadialChoice? _selectedChoice;

    public RadialSlotRow(string label, RadialTargetSettings target)
    {
        Label = label;
        Target = target;
    }

    public string Label { get; }

    public RadialTargetSettings Target { get; }

    public ObservableCollection<RadialChoice> Choices { get; } = new();

    public RadialChoice? SelectedChoice
    {
        get => _selectedChoice;
        set
        {
            if (ReferenceEquals(_selectedChoice, value))
            {
                return;
            }

            _selectedChoice = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(SelectedChoice)));
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CanCycle)));
        }
    }

    public bool CycleEnabled
    {
        get => Target.CycleEnabled;
        set
        {
            if (Target.CycleEnabled == value)
            {
                return;
            }

            Target.CycleEnabled = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CycleEnabled)));
        }
    }

    public bool CanCycle => SelectedChoice is { Kind: not RadialTargetKind.None } choice &&
        WindowCycleService.CanCycle(choice.Action);

    public void RefreshChoices(IEnumerable<RadialChoice> choices)
    {
        Choices.Clear();
        foreach (var choice in choices)
        {
            Choices.Add(choice);
        }

        SelectedChoice = Choices.FirstOrDefault(choice =>
            choice.Kind == Target.Kind &&
            (choice.Kind == RadialTargetKind.None ||
             (choice.Kind == RadialTargetKind.Action && choice.Action == Target.Action) ||
             (choice.Kind == RadialTargetKind.Keybind && choice.KeybindId == Target.KeybindId)));
        SelectedChoice ??= Choices[0];
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CycleEnabled)));
    }

    public void Select(RadialChoice choice)
    {
        Target.Kind = choice.Kind;
        Target.Action = choice.Action;
        Target.KeybindId = choice.KeybindId;
        Target.CycleEnabled = choice.Kind != RadialTargetKind.None && choice.DefaultCycle;
        SelectedChoice = choice;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(CycleEnabled)));
    }

    public event PropertyChangedEventHandler? PropertyChanged;
}
