using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace LoopW;

public partial class SettingsWindow : Window
{
    public static IReadOnlyList<KeyValuePair<WindowAction, string>> ActionChoices { get; } =
        Enum.GetValues<WindowAction>()
            .Select(a => new KeyValuePair<WindowAction, string>(a, WindowActionService.ActionName(a)))
            .ToArray();

    private readonly GlobalHotkey _hotkey;
    private readonly AppSettings _settings;
    private readonly ObservableCollection<KeybindRow> _rows = new();
    private bool _capturingUi;
    private bool _loading;

    public SettingsWindow(GlobalHotkey hotkey, AppSettings settings)
    {
        InitializeComponent();
        _hotkey = hotkey;
        _settings = settings;
        KeybindList.ItemsSource = _rows;
    }

    public event Action<AppSettings>? SettingsChanged;

    private void Window_SourceInitialized(object? sender, EventArgs e)
    {
        var hwnd = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (hwnd == IntPtr.Zero)
        {
            return;
        }

        foreach (var attribute in new[] { NativeMethods.DwmwaUseImmersiveDarkMode, NativeMethods.DwmwaUseImmersiveDarkModeBefore20h1 })
        {
            var enabled = 1;
            if (NativeMethods.DwmSetWindowAttribute(hwnd, attribute, ref enabled, sizeof(int)) == 0)
            {
                break;
            }
        }
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        _loading = true;
        foreach (var keybind in _settings.Keybinds)
        {
            _rows.Add(new KeybindRow(keybind));
        }

        TriggerLabel.Text = HotkeyNames.For(_settings.TriggerModifiers, _settings.TriggerVk);
        LaunchAtLoginCheck.IsChecked = _settings.LaunchAtLogin;
        RadialEnabledCheck.IsChecked = _settings.RadialEnabled;
        CursorInteractionCheck.IsChecked = _settings.CursorInteractionEnabled;
        OuterRadiusSlider.Value = _settings.RadialOuterRadius;
        InnerRadiusSlider.Value = _settings.RadialInnerRadius;
        PreviewEnabledCheck.IsChecked = _settings.PreviewEnabled;
        PreviewPaddingSlider.Value = _settings.PreviewPadding;
        PreviewCornerSlider.Value = _settings.PreviewCornerRadius;
        PreviewBorderWidthSlider.Value = _settings.PreviewBorderWidth;
        AccentColorText.Text = _settings.AccentColor;
        SectorFillText.Text = _settings.RadialSectorFill;
        SectorStrokeText.Text = _settings.RadialSectorStroke;
        RingFillText.Text = _settings.RadialRingFill;
        PreviewBorderText.Text = _settings.PreviewBorderColor;
        UpdateValueLabels();
        _loading = false;
    }

    private void Trigger_MouseLeftButtonUp(object sender, MouseButtonEventArgs e) => BeginTriggerCapture();

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
                TriggerLabel.Text = HotkeyNames.For(mods, vk);
                SaveSettings("Trigger updated");
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "Trigger capture cancelled";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "That key is reserved by the OS — try another";
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
            StatusText.Text = "Could not update launch-at-login";
            return;
        }

        _settings.LaunchAtLogin = enabled;
        SaveSettings(enabled ? "Launch at login enabled" : "Launch at login disabled");
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
        if (_loading)
        {
            return;
        }

        _settings.RadialOuterRadius = OuterRadiusSlider.Value;
        _settings.RadialInnerRadius = Math.Min(InnerRadiusSlider.Value, _settings.RadialOuterRadius - 8);
        InnerRadiusSlider.Value = _settings.RadialInnerRadius;
        UpdateValueLabels();
        SaveSettings("Radial size saved");
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
        SettingsChanged?.Invoke(_settings);
        StatusText.Text = "Theme saved";
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
                _rows.Add(new KeybindRow(new Keybind(mods, vk, WindowAction.RightHalf)));
                SaveSettings("Keybind added");
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "Keybind capture cancelled";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "That key is reserved by the OS — try another";
            });
    }

    private void KeyChip_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (_capturingUi || sender is not TextBlock { DataContext: KeybindRow row })
        {
            return;
        }

        SetCapturingUi(true);
        _hotkey.BeginCapture(
            (mods, vk) =>
            {
                SetCapturingUi(false);
                row.Keybind.Modifiers = mods;
                row.Keybind.Vk = vk;
                row.Refresh();
                SaveSettings("Keybind rebound");
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "Rebind cancelled";
            },
            () =>
            {
                SetCapturingUi(false);
                StatusText.Text = "That key is reserved by the OS — try another";
            });
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

    private void Delete_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: KeybindRow row })
        {
            _rows.Remove(row);
            SaveSettings("Keybind deleted");
        }
    }

    private void SetCapturingUi(bool capturing)
    {
        _capturingUi = capturing;
        AddButton.IsEnabled = !capturing;
        TriggerButton.IsEnabled = !capturing;
        StatusText.Text = capturing ? "Press a key or combo — Esc to cancel" : "Saved";
    }

    private void SaveSettings(string status)
    {
        _settings.Keybinds = _rows.Select(r => r.Keybind).ToList();
        _settings.Save();
        _hotkey.SetBinding(_settings.TriggerModifiers, _settings.TriggerVk);
        _hotkey.SetKeybinds(_settings.Keybinds);
        SettingsChanged?.Invoke(_settings);
        StatusText.Text = status;
    }

    private void UpdateValueLabels()
    {
        OuterRadiusValue.Text = $"{OuterRadiusSlider.Value:0} px";
        InnerRadiusValue.Text = $"{InnerRadiusSlider.Value:0} px";
        PreviewPaddingValue.Text = $"{PreviewPaddingSlider.Value:0} px";
        PreviewCornerValue.Text = $"{PreviewCornerSlider.Value:0} px";
        PreviewBorderWidthValue.Text = $"{PreviewBorderWidthSlider.Value:0} px";
    }
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
