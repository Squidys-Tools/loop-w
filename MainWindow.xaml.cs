using System;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Shapes;

namespace LoopW;

public partial class MainWindow : Window
{
    private const double RadialCanvasPadding = 76;

    private readonly GlobalHotkey _hotkey = new();
    private readonly AppSettings _settings;
    private IntPtr _targetWindow;
    private RadialOverlayWindow? _activeOverlay;
    private SettingsWindow? _settingsWindow;
    private bool _capturing;

    public MainWindow()
    {
        InitializeComponent();
        _settings = AppSettings.Load();
        _hotkey.SetBinding(_settings.TriggerModifiers, _settings.TriggerVk);
        _hotkey.SetKeybinds(_settings.Keybinds);
        ApplyVisualSettings();
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        BuildRadialGeometry();
        AnimateRadialMenuIn();
        _hotkey.TriggerPressed += Hotkey_TriggerPressed;
        _hotkey.TriggerReleased += Hotkey_TriggerReleased;
        _hotkey.KeyCaptured += Hotkey_KeyCaptured;
        _hotkey.CaptureCancelled += Hotkey_CaptureCancelled;
        _hotkey.CaptureRejected += Hotkey_CaptureRejected;
        _hotkey.KeybindPressed += Hotkey_KeybindPressed;
        _hotkey.Start();

        UpdateTriggerLabel();
        TargetStatus.Text = $"  ·  No target captured — hold {TriggerLabel.Text} while another app is focused";

        if (!_hotkey.IsActive)
        {
            TargetStatus.Text = "  ·  Could not install the global keyboard hook";
            RebindHint.Text = "unavailable";
            RebindHint.Cursor = Cursors.Arrow;
        }
    }

    private void Window_Closed(object? sender, EventArgs e)
    {
        _hotkey.Dispose();
    }

    private void BuildRadialGeometry()
    {
        var outerRadius = _settings.RadialOuterRadius;
        var innerRadius = _settings.RadialInnerRadius;
        var diameter = outerRadius * 2;
        var panelSize = diameter + RadialCanvasPadding * 2;

        RadialPanel.Width = panelSize;
        RadialPanel.Height = panelSize;
        RadialPanelCanvas.Width = panelSize;
        RadialPanelCanvas.Height = panelSize;

        foreach (var path in new[] { RadialRing, TopWedge, RightWedge, BottomWedge, LeftWedge })
        {
            path.Width = diameter;
            path.Height = diameter;
            Canvas.SetLeft(path, RadialCanvasPadding);
            Canvas.SetTop(path, RadialCanvasPadding);
        }

        RadialRing.Data = RadialGeometry.BuildAnnulus(outerRadius, outerRadius, innerRadius);
        TopWedge.Data = RadialGeometry.BuildWedge(outerRadius, outerRadius, innerRadius, -135, -45);
        RightWedge.Data = RadialGeometry.BuildWedge(outerRadius, outerRadius, innerRadius, -45, 45);
        BottomWedge.Data = RadialGeometry.BuildWedge(outerRadius, outerRadius, innerRadius, 45, 135);
        LeftWedge.Data = RadialGeometry.BuildWedge(outerRadius, outerRadius, innerRadius, 135, 225);
    }

    private void AnimateRadialMenuIn()
    {
        RadialPanel.Opacity = 0;
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(260));
        RadialPanel.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(0, 1, duration) { EasingFunction = ease });
        RadialPanelScale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(0.96, 1, duration) { EasingFunction = ease });
        RadialPanelScale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(0.96, 1, duration) { EasingFunction = ease });
    }

    private void Window_SourceInitialized(object? sender, EventArgs e)
    {
        var hwnd = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (hwnd == IntPtr.Zero)
        {
            return;
        }

        // 20 is the immersive dark-mode attribute on Windows 11 / 10 2004+;
        // 19 covers Windows 10 1903–1909.
        foreach (var attribute in new[] { NativeMethods.DwmwaUseImmersiveDarkMode, NativeMethods.DwmwaUseImmersiveDarkModeBefore20h1 })
        {
            var enabled = 1;
            if (NativeMethods.DwmSetWindowAttribute(hwnd, attribute, ref enabled, sizeof(int)) == 0)
            {
                break;
            }
        }
    }

    private void Hotkey_TriggerPressed()
    {
        CaptureTargetWindow();
    }

    private void Hotkey_TriggerReleased()
    {
        if (_activeOverlay is { IsVisible: true } overlay)
        {
            overlay.CommitOrClose();
        }
    }

    private void Hotkey_KeyCaptured(uint modifiers, uint vk)
    {
        _capturing = false;
        _hotkey.SetBinding(modifiers, vk);

        // Load the existing settings and update only the trigger fields, so a
        // rebind never wipes out persisted keybinds.
        _settings.TriggerModifiers = modifiers;
        _settings.TriggerVk = vk;
        _settings.Save();

        SetCapturingUi(false);
        TargetStatus.Text = $"  ·  Trigger set to {HotkeyNames.For(modifiers, vk)}";
        Keyboard.ClearFocus();
    }

    private void Hotkey_CaptureCancelled()
    {
        _capturing = false;
        SetCapturingUi(false);
        TargetStatus.Text = "  ·  Rebind cancelled";
    }

    private void Hotkey_CaptureRejected()
    {
        _capturing = false;
        SetCapturingUi(false);
        TargetStatus.Text = "  ·  That key is reserved by the OS — try another (Caps Lock is a good one)";
    }

    private void TriggerCap_MouseUp(object sender, MouseButtonEventArgs e) => BeginRebind();

    private void RebindHint_MouseUp(object sender, MouseButtonEventArgs e) => BeginRebind();

    private void BeginRebind()
    {
        if (_capturing || !_hotkey.IsActive)
        {
            return;
        }

        _capturing = true;
        _hotkey.BeginCapture();
        SetCapturingUi(true);
        TargetStatus.Text = "  ·  Press any key, or a combo like Ctrl + B — Esc to cancel";
        Keyboard.ClearFocus();
    }

    private void SetCapturingUi(bool capturing)
    {
        TriggerLabel.Text = capturing ? "Press a key…" : HotkeyNames.For(_hotkey.TriggerModifiers, _hotkey.TriggerVk);
        RebindHint.Text = capturing ? "Esc to cancel" : "rebind";
        TriggerCap.BorderBrush = capturing
            ? (System.Windows.Media.Brush)FindResource("AccentBrush")
            : (System.Windows.Media.Brush)FindResource("CapBrush");
    }

    private void UpdateTriggerLabel()
    {
        TriggerLabel.Text = HotkeyNames.For(_hotkey.TriggerModifiers, _hotkey.TriggerVk);
    }

    private void CaptureTargetWindow()
    {
        if (_activeOverlay is { IsVisible: true })
        {
            return;
        }

        var foreground = NativeMethods.GetForegroundWindow();
        var ownWindow = new System.Windows.Interop.WindowInteropHelper(this).Handle;

        if (foreground == IntPtr.Zero || foreground == ownWindow)
        {
            TargetStatus.Text = $"  ·  Focus another app, then press {TriggerLabel.Text}";
            return;
        }

        _targetWindow = foreground;
        var title = new StringBuilder(256);
        NativeMethods.GetWindowText(_targetWindow, title, title.Capacity);
        TargetStatus.Text = $"  ·  Target: {(title.Length > 0 ? title.ToString() : "active window")}";
        if (_settings.RadialEnabled)
        {
            var overlay = new RadialOverlayWindow(_targetWindow, CommitOverlayAction, _settings);
            _activeOverlay = overlay;
            overlay.Closed += (_, _) => _activeOverlay = null;
            overlay.ShowAtCursor();
        }
    }

    private void Hotkey_KeybindPressed(Keybind keybind)
    {
        if (_targetWindow == IntPtr.Zero)
        {
            TargetStatus.Text = $"  ·  Capture a target first with {TriggerLabel.Text}";
            return;
        }

        // A keybind applies the action directly, so dismiss any open radial
        // overlay; otherwise releasing the trigger would commit a second wedge.
        _activeOverlay?.Dismiss();

        ApplyWindowAction(keybind.Action, keybind.CycleEnabled);
    }

    private void Settings_MouseUp(object sender, MouseButtonEventArgs e) => OpenSettings();

    private void OpenSettings()
    {
        if (_settingsWindow == null)
        {
            _settingsWindow = new SettingsWindow(_hotkey, _settings);
            _settingsWindow.SettingsChanged += ApplySettings;
            _settingsWindow.Closed += (_, _) => _settingsWindow = null;
        }

        _settingsWindow.Show();
        _settingsWindow.Activate();
    }

    private void CommitOverlayAction(WindowHalf action)
    {
        ApplyWindowAction(ToWindowAction(action), cycleEnabled: true);
    }

    private void Wedge_MouseUp(object sender, MouseButtonEventArgs e)
    {
        if (sender is Path path && path.Tag is string action)
        {
            ApplyAction(action);
        }
    }

    private void Window_KeyDown(object sender, KeyEventArgs e)
    {
        var action = e.Key switch
        {
            Key.Left => "Left",
            Key.Right => "Right",
            Key.Up => "Top",
            Key.Down => "Bottom",
            _ => string.Empty
        };

        if (action.Length > 0)
        {
            ApplyAction(action);
            e.Handled = true;
        }
    }

    private void ApplyAction(string action)
    {
        var half = action switch
        {
            "Left" => WindowHalf.Left,
            "Right" => WindowHalf.Right,
            "Top" => WindowHalf.Top,
            "Bottom" => WindowHalf.Bottom,
            _ => WindowHalf.Left
        };

        ApplyWindowAction(ToWindowAction(half), cycleEnabled: true);
    }

    private void ApplyWindowAction(WindowAction requestedAction, bool cycleEnabled)
    {
        if (_targetWindow == IntPtr.Zero)
        {
            TargetStatus.Text = $"  ·  Capture a target first with {TriggerLabel.Text}";
            return;
        }

        var selection = WindowCycleService.Select(_targetWindow, requestedAction, cycleEnabled);
        SelectedAction.Text = WindowActionService.ActionName(selection.EffectiveAction);

        var applied = WindowActionService.TryApply(_targetWindow, selection.EffectiveAction, out var message);
        if (applied)
        {
            WindowCycleService.Commit(_targetWindow, selection);
        }

        TargetStatus.Text = $"  ·  {message}{(applied ? selection.StatusSuffix : string.Empty)}";
    }

    private static WindowAction ToWindowAction(WindowHalf action) => action switch
    {
        WindowHalf.Left => WindowAction.LeftHalf,
        WindowHalf.Right => WindowAction.RightHalf,
        WindowHalf.Top => WindowAction.TopHalf,
        WindowHalf.Bottom => WindowAction.BottomHalf,
        _ => WindowAction.LeftHalf
    };

    private void ApplySettings(AppSettings settings)
    {
        _hotkey.SetBinding(settings.TriggerModifiers, settings.TriggerVk);
        _hotkey.SetKeybinds(settings.Keybinds);
        UpdateTriggerLabel();
        BuildRadialGeometry();
        ApplyVisualSettings();
    }

    private void ApplyVisualSettings()
    {
        Resources["AccentBrush"] = CreateBrush(_settings.AccentColor, "#007AFF");
        Resources["SectorFillBrush"] = CreateBrush(_settings.RadialSectorFill, "#7A007AFF");
        Resources["SectorStrokeBrush"] = CreateBrush(_settings.RadialSectorStroke, "#F0007AFF");
        RadialRing.Fill = CreateBrush(_settings.RadialRingFill, "#B61E1E1E");
    }

    private static System.Windows.Media.Brush CreateBrush(string value, string fallback)
    {
        try
        {
            if (new System.Windows.Media.BrushConverter().ConvertFromString(value) is System.Windows.Media.Brush brush)
            {
                brush.Freeze();
                return brush;
            }
        }
        catch
        {
            // Settings.cs normalizes persisted colors. Keep the UI safe if a
            // value is edited in memory before it is saved.
        }

        return new System.Windows.Media.BrushConverter().ConvertFromString(fallback) as System.Windows.Media.Brush
            ?? System.Windows.Media.Brushes.Transparent;
    }
}
