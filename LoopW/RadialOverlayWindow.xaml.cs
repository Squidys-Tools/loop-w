using System;
using System.Windows;
using System.Windows.Input;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Threading;

namespace LoopW;

public partial class RadialOverlayWindow : Window
{
    private readonly IntPtr _targetWindow;
    private readonly Action<WindowHalf> _commit;
    private readonly PreviewOverlayWindow _preview = new();
    private readonly DispatcherTimer _triggerTimer = new() { Interval = TimeSpan.FromMilliseconds(30) };
    private WindowHalf? _selected;
    private bool _closing;

    internal RadialOverlayWindow(IntPtr targetWindow, Action<WindowHalf> commit)
    {
        InitializeComponent();
        _targetWindow = targetWindow;
        _commit = commit;
        _triggerTimer.Tick += TriggerTimer_Tick;
        TopButton.RenderTransform = new ScaleTransform(1, 1);
        RightButton.RenderTransform = new ScaleTransform(1, 1);
        BottomButton.RenderTransform = new ScaleTransform(1, 1);
        LeftButton.RenderTransform = new ScaleTransform(1, 1);
    }

    private void Overlay_Loaded(object sender, RoutedEventArgs e)
    {
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(220));
        var scale = (ScaleTransform)OverlaySurface.RenderTransform;
        OverlaySurface.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(0, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(0.9, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(0.9, 1, duration) { EasingFunction = ease });
    }

    public void ShowAtCursor()
    {
        if (!NativeMethods.GetCursorPos(out var cursor))
        {
            return;
        }

        Left = cursor.X - Width / 2;
        Top = cursor.Y - Height / 2;
        Show();
        _triggerTimer.Start();
    }

    private void TriggerTimer_Tick(object? sender, EventArgs e)
    {
        if ((NativeMethods.GetAsyncKeyState(NativeMethods.VkShift) & 0x8000) == 0)
        {
            if (_selected.HasValue)
            {
                Commit(_selected.Value);
            }
            else
            {
                CloseOverlay();
            }
        }
    }

    private void Overlay_MouseMove(object sender, MouseEventArgs e)
    {
        var point = e.GetPosition(this);
        var dx = point.X - Width / 2;
        var dy = point.Y - Height / 2;

        if (Math.Sqrt(dx * dx + dy * dy) < 54)
        {
            SetSelection(null);
            return;
        }

        var selection = Math.Abs(dx) > Math.Abs(dy)
            ? (dx < 0 ? WindowHalf.Left : WindowHalf.Right)
            : (dy < 0 ? WindowHalf.Top : WindowHalf.Bottom);

        SetSelection(selection);
    }

    private void Overlay_MouseLeftButtonUp(object sender, MouseButtonEventArgs e)
    {
        if (_selected.HasValue)
        {
            Commit(_selected.Value);
        }
        else
        {
            CloseOverlay();
        }
    }

    private void Overlay_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            CloseOverlay();
            e.Handled = true;
            return;
        }

        var selection = e.Key switch
        {
            Key.Left => WindowHalf.Left,
            Key.Right => WindowHalf.Right,
            Key.Up => WindowHalf.Top,
            Key.Down => WindowHalf.Bottom,
            _ => (WindowHalf?)null
        };

        if (selection.HasValue)
        {
            Commit(selection.Value);
            e.Handled = true;
        }
    }

    private void Action_Click(object sender, RoutedEventArgs e)
    {
        if (sender is FrameworkElement element && element.Tag is string action)
        {
            Commit(action switch
            {
                "Left" => WindowHalf.Left,
                "Right" => WindowHalf.Right,
                "Top" => WindowHalf.Top,
                "Bottom" => WindowHalf.Bottom,
                _ => WindowHalf.Left
            });
        }
    }

    private void SetSelection(WindowHalf? selection)
    {
        _selected = selection;
        SelectionLabel.Text = selection switch
        {
            WindowHalf.Left => "Left half",
            WindowHalf.Right => "Right half",
            WindowHalf.Top => "Top half",
            WindowHalf.Bottom => "Bottom half",
            _ => "Move"
        };

        SetButtonState(TopButton, selection == WindowHalf.Top);
        SetButtonState(RightButton, selection == WindowHalf.Right);
        SetButtonState(BottomButton, selection == WindowHalf.Bottom);
        SetButtonState(LeftButton, selection == WindowHalf.Left);

        if (selection.HasValue && WindowActionService.TryGetHalfFrame(_targetWindow, selection.Value, out var frame, out _))
        {
            _preview.ShowFrame(frame, selection.Value);
            _preview.Topmost = true;
        }
        else if (_preview.IsVisible)
        {
            _preview.Hide();
        }
    }

    private void Commit(WindowHalf action)
    {
        if (_closing)
        {
            return;
        }

        _commit(action);
        CloseOverlay();
    }

    private void CloseOverlay()
    {
        if (_closing)
        {
            return;
        }

        _closing = true;
        _triggerTimer.Stop();
        _preview.HideAnimated();

        var ease = new CubicEase { EasingMode = EasingMode.EaseIn };
        var duration = new Duration(TimeSpan.FromMilliseconds(120));
        var scale = (ScaleTransform)OverlaySurface.RenderTransform;
        var opacity = new DoubleAnimation(OverlaySurface.Opacity, 0, duration) { EasingFunction = ease };
        opacity.Completed += (_, _) => Close();
        OverlaySurface.BeginAnimation(UIElement.OpacityProperty, opacity);
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(scale.ScaleX, 0.94, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(scale.ScaleY, 0.94, duration) { EasingFunction = ease });
    }

    private static void SetButtonState(Button button, bool selected)
    {
        button.Background = selected ? new SolidColorBrush(Color.FromArgb(235, 48, 65, 83)) : new SolidColorBrush(Color.FromArgb(234, 27, 32, 43));
        button.BorderBrush = selected ? new SolidColorBrush(Color.FromRgb(167, 243, 208)) : new SolidColorBrush(Color.FromRgb(67, 80, 101));
        var scale = (ScaleTransform)button.RenderTransform;
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var target = selected ? 1.1 : 1.0;
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(scale.ScaleX, target, new Duration(TimeSpan.FromMilliseconds(130))) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(scale.ScaleY, target, new Duration(TimeSpan.FromMilliseconds(130))) { EasingFunction = ease });
    }
}
