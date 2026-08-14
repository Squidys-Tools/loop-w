using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Text;

namespace LoopW;

public partial class MainWindow : Window
{
    private const int HotKeyId = 7001;
    private HwndSource? _windowSource;
    private IntPtr _targetWindow;

    public MainWindow()
    {
        InitializeComponent();
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        var helper = new WindowInteropHelper(this);
        _windowSource = HwndSource.FromHwnd(helper.Handle);
        _windowSource?.AddHook(WindowMessageHook);

        if (!NativeMethods.RegisterHotKey(helper.Handle, HotKeyId, NativeMethods.ModShift, NativeMethods.VkSpace))
        {
            TargetStatus.Text = "  ·  Shift + Space is unavailable — choose another trigger later";
        }
    }

    private void Window_Closed(object? sender, EventArgs e)
    {
        var handle = new WindowInteropHelper(this).Handle;
        NativeMethods.UnregisterHotKey(handle, HotKeyId);
        _windowSource?.RemoveHook(WindowMessageHook);
    }

    private IntPtr WindowMessageHook(IntPtr hwnd, int message, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (message == NativeMethods.WmHotKey && wParam.ToInt32() == HotKeyId)
        {
            CaptureTargetWindow();
            handled = true;
        }

        return IntPtr.Zero;
    }

    private void CaptureTargetWindow()
    {
        var foreground = NativeMethods.GetForegroundWindow();
        var ownWindow = new WindowInteropHelper(this).Handle;

        if (foreground == IntPtr.Zero || foreground == ownWindow)
        {
            TargetStatus.Text = "  ·  Focus another app, then press Shift + Space";
            return;
        }

        _targetWindow = foreground;
        var title = new StringBuilder(256);
        NativeMethods.GetWindowText(_targetWindow, title, title.Capacity);
        TargetStatus.Text = $"  ·  Target: {(title.Length > 0 ? title.ToString() : "active window")}";
        var overlay = new RadialOverlayWindow(_targetWindow, CommitOverlayAction);
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
        if (WindowActionService.ApplyHalf(_targetWindow, action, out var error))
        {
            TargetStatus.Text = $"  ·  Applied {label.ToLowerInvariant()} to target window";
        }
        else
        {
            TargetStatus.Text = $"  ·  {error}";
        }
    }

    private void Action_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is string action)
        {
            ApplyAction(action);
        }
    }

    private void Window_KeyDown(object sender, System.Windows.Input.KeyEventArgs e)
    {
        var action = e.Key switch
        {
            System.Windows.Input.Key.Left => "Left",
            System.Windows.Input.Key.Right => "Right",
            System.Windows.Input.Key.Up => "Top",
            System.Windows.Input.Key.Down => "Bottom",
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
            TargetStatus.Text = "  ·  Capture a target first with Shift + Space";
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

        if (!WindowActionService.ApplyHalf(_targetWindow, half, out var error))
        {
            TargetStatus.Text = $"  ·  {error}";
        }
        else
        {
            TargetStatus.Text = $"  ·  Applied {SelectedAction.Text.ToLowerInvariant()} to target window";
        }
    }
}
