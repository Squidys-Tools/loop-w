using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;

namespace LoopW;

public partial class PreviewOverlayWindow : Window
{
    private readonly AppSettings _settings;
    private readonly double _blurMargin;
    private double _workLeft;
    private double _workTop;
    private double _workWidth;
    private double _workHeight;
    private BitmapSource? _backdrop;

    public PreviewOverlayWindow(AppSettings settings)
    {
        InitializeComponent();
        _settings = settings;
        _blurMargin = settings.PreviewPadding;
        PreviewSurface.CornerRadius = new CornerRadius(settings.PreviewCornerRadius);
        SurfaceTint.CornerRadius = new CornerRadius(Math.Max(1, settings.PreviewCornerRadius - 3));
        PreviewSurface.BorderBrush = CreateBrush(settings.PreviewBorderColor, "#FFFFFFFF");
        PreviewSurface.BorderThickness = new Thickness(settings.PreviewBorderWidth);
        SourceInitialized += PreviewOverlayWindow_SourceInitialized;
    }

    private void PreviewOverlayWindow_SourceInitialized(object? sender, EventArgs e)
    {
        NativeMethods.MakeMouseClickThrough(new System.Windows.Interop.WindowInteropHelper(this).Handle);
    }

    internal void ShowFrame(NativeMethods.Rect frame, WindowAction action)
    {
        if (!NativeMethods.TryGetMonitorWorkRect(frame, out var workArea))
        {
            return;
        }

        var monitor = NativeMethods.MonitorFromRect(ref frame, NativeMethods.MonitorDefaultToNearest);
        var scaleX = 1.0;
        var scaleY = 1.0;
        var dpiX = 96.0;
        var dpiY = 96.0;
        if (NativeMethods.TryGetMonitorDpi(monitor, out dpiX, out dpiY))
        {
            scaleX = 96.0 / dpiX;
            scaleY = 96.0 / dpiY;
        }

        EnsureWindowCovering(workArea, scaleX, scaleY);

        var localX = frame.Left * scaleX - _workLeft;
        var localY = frame.Top * scaleY - _workTop;
        var targetWidth = Math.Max(frame.Width * scaleX, 1);
        var targetHeight = Math.Max(frame.Height * scaleY, 1);

        // The window is sized to the work area, which never leaves the screen.
        // Clamp the surface into it so a few pixels of rounding (or a frame that
        // lands flush against a screen edge) can't push the preview off-screen.
        targetWidth = Math.Min(targetWidth, Math.Max(_workWidth - localX, 1));
        targetHeight = Math.Min(targetHeight, Math.Max(_workHeight - localY, 1));
        localX = Math.Max(0, Math.Min(localX, Math.Max(_workWidth - targetWidth, 0)));
        localY = Math.Max(0, Math.Min(localY, Math.Max(_workHeight - targetHeight, 0)));

        if (!IsVisible)
        {
            // The window isn't on screen yet, so this snapshot can't capture the
            // preview's own surface. Capture once and crop from it below.
            CaptureFullBackdrop(workArea, dpiX, dpiY);
        }

        BackdropImage.Clip = null;
        UpdateBackdrop(frame, workArea, dpiX, dpiY, localX, localY, targetWidth, targetHeight);

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
            _backdrop = null;
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

    private void CaptureFullBackdrop(NativeMethods.Rect workArea, double dpiX, double dpiY)
    {
        _backdrop = null;
        try
        {
            var source = ScreenCapture.CaptureRegion(workArea.Left, workArea.Top, workArea.Width, workArea.Height);
            if (source != null)
            {
                source.Freeze();
                _backdrop = source;
            }
        }
        catch
        {
            // blur is decorative; the preview still renders without a backdrop
        }
    }

    private void UpdateBackdrop(
        NativeMethods.Rect frame,
        NativeMethods.Rect workArea,
        double dpiX,
        double dpiY,
        double localX,
        double localY,
        double targetWidth,
        double targetHeight)
    {
        if (_backdrop == null)
        {
            BackdropImage.Source = null;
            return;
        }

        var scaleX = 96.0 / dpiX;
        var scaleY = 96.0 / dpiY;
        var marginX = (int)Math.Round(_blurMargin * dpiX / 96);
        var marginY = (int)Math.Round(_blurMargin * dpiY / 96);

        var x = Math.Max(0, frame.Left - marginX - workArea.Left);
        var y = Math.Max(0, frame.Top - marginY - workArea.Top);
        var width = Math.Max(1, Math.Min(frame.Width + marginX * 2, _backdrop.PixelWidth - x));
        var height = Math.Max(1, Math.Min(frame.Height + marginY * 2, _backdrop.PixelHeight - y));

        try
        {
            BackdropImage.Source = new CroppedBitmap(_backdrop, new Int32Rect(x, y, width, height));

            // Size and position the image to the crop (physical px mapped to DIU) so
            // Stretch="Fill" is 1:1 and edge crops that lose their blur margin can't
            // shift or stretch the backdrop relative to the screen region.
            var imageLeft = (workArea.Left + x) * scaleX - _workLeft - localX;
            var imageTop = (workArea.Top + y) * scaleY - _workTop - localY;
            BackdropImage.Width = width * scaleX;
            BackdropImage.Height = height * scaleY;
            BackdropImage.Margin = new Thickness(imageLeft, imageTop, 0, 0);

            BackdropImage.Clip = new RectangleGeometry(
                new Rect(-imageLeft, -imageTop, Math.Max(targetWidth, 1), Math.Max(targetHeight, 1)), 11, 11);
        }
        catch
        {
            // blur is decorative; the preview still renders without a backdrop
        }
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
            // Invalid in-memory values use a safe fallback.
        }

        return new System.Windows.Media.BrushConverter().ConvertFromString(fallback) as System.Windows.Media.Brush
            ?? System.Windows.Media.Brushes.Transparent;
    }
}
