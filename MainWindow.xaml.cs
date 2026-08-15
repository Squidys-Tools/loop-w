using System;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Shapes;
using System.Windows.Threading;
using Cursors = System.Windows.Input.Cursors;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;

namespace LoopW;

public partial class MainWindow : Window
{
    private const double RadialCanvasPadding = 76;

    private readonly GlobalHotkey _hotkey = new();
    private readonly AppSettings _settings;
    private IntPtr _targetWindow;
    private RadialOverlayWindow? _activeOverlay;
    private SettingsWindow? _settingsWindow;
    private readonly DispatcherTimer _stashTimer;
    private bool _capturing;
    private bool _allowClose;

    public MainWindow()
    {
        InitializeComponent();
        _settings = AppSettings.Load();
        _hotkey.SetBinding(_settings.TriggerModifiers, _settings.TriggerVk);
        _hotkey.SetKeybinds(_settings.Keybinds);
        _stashTimer = new DispatcherTimer(DispatcherPriority.Input)
        {
            Interval = TimeSpan.FromMilliseconds(80)
        };
        _stashTimer.Tick += StashTimer_Tick;
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
        _stashTimer.Start();

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
        _stashTimer.Stop();
        _hotkey.Dispose();
    }

    private void StashTimer_Tick(object? sender, EventArgs e)
    {
        if (NativeMethods.GetCursorPos(out var cursor) &&
            WindowStashService.TryRevealAtCursor(cursor, out var message))
        {
            TargetStatus.Text = $"  ·  {message}";
        }
    }

    private void Window_Closing(object? sender, System.ComponentModel.CancelEventArgs e)
    {
        if (_allowClose)
        {
            return;
        }

        e.Cancel = true;
        Hide();
    }

    internal void AllowClose() => _allowClose = true;

    internal void ShowFromTray()
    {
        Show();
        if (WindowState == WindowState.Minimized)
        {
            WindowState = WindowState.Normal;
        }

        Activate();
    }

    internal void OpenSettingsFromTray() => OpenSettings();

    internal string ExecuteExternalAction(WindowAction action)
    {
        var ownWindow = new System.Windows.Interop.WindowInteropHelper(this).Handle;
        if (action == WindowAction.RevealStashed)
        {
            var revealed = WindowActionService.TryApply(IntPtr.Zero, action, out var revealMessage);
            TargetStatus.Text = $"  ·  {revealMessage}";
            return revealMessage;
        }

        var foreground = NativeMethods.GetForegroundWindow();
        if (foreground == IntPtr.Zero || foreground == ownWindow)
        {
            return "No external foreground window is available.";
        }

        var applied = WindowActionService.TryApply(foreground, action, out var message);
        if (applied)
        {
            SelectedAction.Text = WindowActionService.ActionName(action);
        }

        TargetStatus.Text = $"  ·  {message}";
        return message;
    }

    internal string DescribeKeybinds() =>
        LoopCommandFormatter.Keybinds(_settings.Keybinds, _settings.TriggerModifiers, _settings.TriggerVk);

    internal string DescribeAll() =>
        LoopCommandFormatter.All(_settings.Keybinds, _settings.TriggerModifiers, _settings.TriggerVk);

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

        var paths = new[]
        {
            RightWedge, BottomRightWedge, BottomWedge, BottomLeftWedge,
            LeftWedge, TopLeftWedge, TopWedge, TopRightWedge
        };

        foreach (var path in paths)
        {
            path.Width = diameter;
            path.Height = diameter;
            Canvas.SetLeft(path, RadialCanvasPadding);
            Canvas.SetTop(path, RadialCanvasPadding);
        }

        RadialRing.Data = RadialGeometry.BuildAnnulus(outerRadius, outerRadius, innerRadius);
        for (var i = 0; i < RadialActionCatalog.Slots.Count; i++)
        {
            var slot = RadialActionCatalog.Slots[i];
            paths[i].Data = RadialGeometry.BuildWedge(
                outerRadius,
                outerRadius,
                innerRadius,
                slot.FromDegrees,
                slot.ToDegrees);
        }
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
        if (keybind.Action == WindowAction.RevealStashed)
        {
            _activeOverlay?.Dismiss();
            ApplyWindowAction(keybind.Action, cycleEnabled: false);
            return;
        }

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

    private void CommitOverlayAction(WindowAction action)
    {
        ApplyWindowAction(action, cycleEnabled: WindowCycleService.CanCycle(action));
    }

    private void Wedge_MouseUp(object sender, MouseButtonEventArgs e)
    {
        if (sender is Path { Tag: string action } && Enum.TryParse<WindowAction>(action, out var selectedAction))
        {
            ApplyWindowAction(selectedAction, WindowCycleService.CanCycle(selectedAction));
        }
    }

    private void Window_KeyDown(object sender, KeyEventArgs e)
    {
        var action = e.Key switch
        {
            Key.Left => WindowAction.LeftHalf,
            Key.Right => WindowAction.RightHalf,
            Key.Up => WindowAction.TopHalf,
            Key.Down => WindowAction.BottomHalf,
            _ => (WindowAction?)null
        };

        if (action.HasValue)
        {
            ApplyWindowAction(action.Value, WindowCycleService.CanCycle(action.Value));
            e.Handled = true;
        }
    }

    private void ApplyWindowAction(WindowAction requestedAction, bool cycleEnabled)
    {
        if (requestedAction == WindowAction.RevealStashed)
        {
            SelectedAction.Text = WindowActionService.ActionName(requestedAction);
            WindowActionService.TryApply(IntPtr.Zero, requestedAction, out var revealMessage);
            TargetStatus.Text = $"  ·  {revealMessage}";
            return;
        }

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
