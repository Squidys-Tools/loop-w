using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;

namespace LoopW;

public partial class PreviewOverlayWindow : Window
{
    private double _workLeft;
    private double _workTop;
    private double _workWidth;
    private double _workHeight;

    public PreviewOverlayWindow()
    {
        InitializeComponent();
        SourceInitialized += PreviewOverlayWindow_SourceInitialized;
    }

    private void PreviewOverlayWindow_SourceInitialized(object? sender, EventArgs e)
    {
        NativeMethods.MakeMouseClickThrough(new System.Windows.Interop.WindowInteropHelper(this).Handle);
    }

    internal void ShowFrame(NativeMethods.Rect frame, WindowHalf action)
    {
        if (!NativeMethods.TryGetMonitorWorkRect(frame, out var workArea))
        {
            return;
        }

        var monitor = NativeMethods.MonitorFromRect(ref frame, NativeMethods.MonitorDefaultToNearest);
        var scaleX = 1.0;
        var scaleY = 1.0;
        if (NativeMethods.TryGetMonitorDpi(monitor, out var dpiX, out var dpiY))
        {
            scaleX = 96.0 / dpiX;
            scaleY = 96.0 / dpiY;
        }

        EnsureWindowCovering(workArea, scaleX, scaleY);

        var localX = frame.Left * scaleX - _workLeft;
        var localY = frame.Top * scaleY - _workTop;
        var targetWidth = Math.Max(frame.Width * scaleX, 1);
        var targetHeight = Math.Max(frame.Height * scaleY, 1);

        ActionLabel.Text = action switch
        {
            WindowHalf.Left => "Left half",
            WindowHalf.Right => "Right half",
            WindowHalf.Top => "Top half",
            WindowHalf.Bottom => "Bottom half",
            _ => "Preview"
        };

        if (!IsVisible)
        {
            PreviewSurface.BeginAnimation(Canvas.LeftProperty, null);
            PreviewSurface.BeginAnimation(Canvas.TopProperty, null);
            PreviewSurface.BeginAnimation(FrameworkElement.WidthProperty, null);
            PreviewSurface.BeginAnimation(FrameworkElement.HeightProperty, null);
            PreviewSurface.BeginAnimation(UIElement.OpacityProperty, null);

            Canvas.SetLeft(PreviewSurface, localX);
            Canvas.SetTop(PreviewSurface, localY);
            PreviewSurface.Width = targetWidth;
            PreviewSurface.Height = targetHeight;
            Show();
            AnimateSurfaceIn();
            return;
        }

        var oldLeft = (double)PreviewSurface.GetValue(Canvas.LeftProperty);
        var oldTop = (double)PreviewSurface.GetValue(Canvas.TopProperty);
        var oldWidth = PreviewSurface.Width;
        var oldHeight = PreviewSurface.Height;

        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(140));
        PreviewSurface.BeginAnimation(Canvas.LeftProperty, new DoubleAnimation(oldLeft, localX, duration) { EasingFunction = ease });
        PreviewSurface.BeginAnimation(Canvas.TopProperty, new DoubleAnimation(oldTop, localY, duration) { EasingFunction = ease });
        PreviewSurface.BeginAnimation(FrameworkElement.WidthProperty, new DoubleAnimation(oldWidth, targetWidth, duration) { EasingFunction = ease });
        PreviewSurface.BeginAnimation(FrameworkElement.HeightProperty, new DoubleAnimation(oldHeight, targetHeight, duration) { EasingFunction = ease });
    }

    internal void HideAnimated(bool destroy = false)
    {
        if (!IsVisible)
        {
            if (destroy)
            {
                Close();
            }
            return;
        }

        var animation = new DoubleAnimation(PreviewSurface.Opacity, 0, new Duration(TimeSpan.FromMilliseconds(100)))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseIn }
        };
        animation.Completed += (_, _) =>
        {
            if (destroy)
            {
                Close();
            }
            else
            {
                Hide();
            }
        };
        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, animation);
    }

    private void EnsureWindowCovering(NativeMethods.Rect workArea, double scaleX, double scaleY)
    {
        var workLeft = workArea.Left * scaleX;
        var workTop = workArea.Top * scaleY;
        var workWidth = workArea.Width * scaleX;
        var workHeight = workArea.Height * scaleY;

        var fits = IsVisible &&
            Math.Abs(_workLeft - workLeft) < 0.5 &&
            Math.Abs(_workTop - workTop) < 0.5 &&
            Math.Abs(_workWidth - workWidth) < 0.5 &&
            Math.Abs(_workHeight - workHeight) < 0.5;

        if (fits)
        {
            return;
        }

        _workLeft = workLeft;
        _workTop = workTop;
        _workWidth = workWidth;
        _workHeight = workHeight;
        Left = workLeft;
        Top = workTop;
        Width = workWidth;
        Height = workHeight;
    }

    private void AnimateSurfaceIn()
    {
        PreviewSurface.Opacity = 0;
        var scale = (ScaleTransform)PreviewSurface.RenderTransform;
        scale.ScaleX = 0.98;
        scale.ScaleY = 0.98;
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(180));
        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(0, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleXProperty, new DoubleAnimation(0.98, 1, duration) { EasingFunction = ease });
        scale.BeginAnimation(ScaleTransform.ScaleYProperty, new DoubleAnimation(0.98, 1, duration) { EasingFunction = ease });
    }
}
