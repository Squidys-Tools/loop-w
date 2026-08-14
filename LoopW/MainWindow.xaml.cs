using System;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace LoopW;

public partial class MainWindow : Window
{
    private readonly GlobalHotkey _hotkey = new();
    private IntPtr _targetWindow;
    private RadialOverlayWindow? _activeOverlay;
    private bool _capturing;

    public MainWindow()
    {
        InitializeComponent();
        var settings = AppSettings.Load();
        _hotkey.SetBinding(settings.TriggerModifiers, settings.TriggerVk);
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        _hotkey.TriggerPressed += Hotkey_TriggerPressed;
        _hotkey.TriggerReleased += Hotkey_TriggerReleased;
        _hotkey.KeyCaptured += Hotkey_KeyCaptured;
        _hotkey.CaptureCancelled += Hotkey_CaptureCancelled;
        _hotkey.CaptureRejected += Hotkey_CaptureRejected;
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
        new AppSettings { TriggerModifiers = modifiers, TriggerVk = vk }.Save();
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
        var overlay = new RadialOverlayWindow(_targetWindow, CommitOverlayAction);
        _activeOverlay = overlay;
        overlay.Closed += (_, _) => _activeOverlay = null;
        overlay.ShowAtCursor();
    }

    private void CommitOverlayAction(WindowHalf action)
    {
        var label = action switch
        {
            WindowHalf.Left => "Left half",
            WindowHalf.Right => "Right half",
            WindowHalf.Top => "Top half",
            WindowHalf.Bottom => "Bottom half",
            _ => "Window"
        };

        SelectedAction.Text = label;
        WindowActionService.ApplyHalf(_targetWindow, action, out var message);
        TargetStatus.Text = $"  ·  {message}";
    }

    private void Action_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is string action)
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
        SelectedAction.Text = action == "Top" ? "Top half" : action == "Bottom" ? "Bottom half" : action + " half";

        if (_targetWindow == IntPtr.Zero)
        {
            TargetStatus.Text = $"  ·  Capture a target first with {TriggerLabel.Text}";
            return;
        }

        var half = action switch
        {
            "Left" => WindowHalf.Left,
            "Right" => WindowHalf.Right,
            "Top" => WindowHalf.Top,
            "Bottom" => WindowHalf.Bottom,
            _ => WindowHalf.Left
        };

        WindowActionService.ApplyHalf(_targetWindow, half, out var message);
        TargetStatus.Text = $"  ·  {message}";
    }
}
