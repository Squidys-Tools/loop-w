using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;

namespace LoopW;

public partial class PreviewOverlayWindow : Window
{
    private const double PreviewScreenGap = 8;

    private readonly AppSettings _settings;
    private readonly double _blurMargin;
    private double _workLeft;
    private double _workTop;
    private double _workWidth;
    private double _workHeight;
    private BitmapSource? _backdrop;
    private NativeMethods.Rect _lastFrame;
    private NativeMethods.Rect _lastWorkArea;
    private WindowAction? _lastAction;
    private double _lastDpiX;
    private double _lastDpiY;
    private bool _hasRenderedFrame;
    private long _transitionVersion;

    public PreviewOverlayWindow(AppSettings settings)
    {
        InitializeComponent();
        _settings = settings;
        PreviewSurface.ApplySettings(settings);
        _blurMargin = PreviewSurface.BlurMargin;

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

        if (IsVisible && _hasRenderedFrame &&
            action == _lastAction &&
            SameRect(frame, _lastFrame) &&
            SameRect(workArea, _lastWorkArea) &&
            Math.Abs(dpiX - _lastDpiX) < 0.01 &&
            Math.Abs(dpiY - _lastDpiY) < 0.01)
        {
            return;
        }

        EnsureWindowCovering(workArea, scaleX, scaleY);

        var localX = frame.Left * scaleX - _workLeft + PreviewScreenGap;
        var localY = frame.Top * scaleY - _workTop + PreviewScreenGap;
        var targetWidth = Math.Max(frame.Width * scaleX - PreviewScreenGap * 2, 1);
        var targetHeight = Math.Max(frame.Height * scaleY - PreviewScreenGap * 2, 1);

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

        PreviewSurface.BackdropImageElement.Clip = null;
        UpdateBackdrop(frame, workArea, dpiX, dpiY, localX, localY, targetWidth, targetHeight);

        _lastFrame = frame;
        _lastWorkArea = workArea;
        _lastAction = action;
        _lastDpiX = dpiX;
        _lastDpiY = dpiY;
        _hasRenderedFrame = true;

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
            PreviewSurface.SurfaceScaleTransform.ScaleX = 1;
            PreviewSurface.SurfaceScaleTransform.ScaleY = 1;
            PreviewSurface.SurfaceTranslateTransform.X = 0;
            PreviewSurface.SurfaceTranslateTransform.Y = 0;
            Show();
            AnimateSurfaceIn();
            return;
        }

        var oldLeft = Canvas.GetLeft(PreviewSurface);
        var oldTop = Canvas.GetTop(PreviewSurface);
        var oldWidth = PreviewSurface.Width;
        var oldHeight = PreviewSurface.Height;
        var oldScaleX = PreviewSurface.SurfaceScaleTransform.ScaleX;
        var oldScaleY = PreviewSurface.SurfaceScaleTransform.ScaleY;
        var oldTranslateX = PreviewSurface.SurfaceTranslateTransform.X;
        var oldTranslateY = PreviewSurface.SurfaceTranslateTransform.Y;
        var oldVisualWidth = oldWidth * oldScaleX;
        var oldVisualHeight = oldHeight * oldScaleY;
        var oldVisualLeft = oldLeft + (oldWidth - oldVisualWidth) / 2 + oldTranslateX;
        var oldVisualTop = oldTop + (oldHeight - oldVisualHeight) / 2 + oldTranslateY;

        var oldCenterX = oldVisualLeft + oldVisualWidth / 2;
        var oldCenterY = oldVisualTop + oldVisualHeight / 2;
        var targetCenterX = localX + targetWidth / 2;
        var targetCenterY = localY + targetHeight / 2;

        PreviewSurface.BeginAnimation(Canvas.LeftProperty, null);
        PreviewSurface.BeginAnimation(Canvas.TopProperty, null);
        PreviewSurface.BeginAnimation(FrameworkElement.WidthProperty, null);
        PreviewSurface.BeginAnimation(FrameworkElement.HeightProperty, null);
        Canvas.SetLeft(PreviewSurface, localX);
        Canvas.SetTop(PreviewSurface, localY);
        PreviewSurface.Width = targetWidth;
        PreviewSurface.Height = targetHeight;

        PreviewSurface.SurfaceScaleTransform.ScaleX = oldVisualWidth / targetWidth;
        PreviewSurface.SurfaceScaleTransform.ScaleY = oldVisualHeight / targetHeight;
        PreviewSurface.SurfaceTranslateTransform.X = oldCenterX - targetCenterX;
        PreviewSurface.SurfaceTranslateTransform.Y = oldCenterY - targetCenterY;

        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(140));
        var transitionVersion = ++_transitionVersion;
        PreviewSurface.SetTransitionRendering(true);

        var scaleXAnimation = new DoubleAnimation(
            PreviewSurface.SurfaceScaleTransform.ScaleX,
            1,
            duration)
        {
            EasingFunction = ease,
            FillBehavior = FillBehavior.Stop
        };
        scaleXAnimation.Completed += (_, _) => CompleteTransition(transitionVersion);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(
            ScaleTransform.ScaleXProperty,
            scaleXAnimation,
            HandoffBehavior.SnapshotAndReplace);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(
            ScaleTransform.ScaleYProperty,
            new DoubleAnimation(
                PreviewSurface.SurfaceScaleTransform.ScaleY,
                1,
                duration)
            {
                EasingFunction = ease,
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(
            TranslateTransform.XProperty,
            new DoubleAnimation(
                PreviewSurface.SurfaceTranslateTransform.X,
                0,
                duration)
            {
                EasingFunction = ease,
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(
            TranslateTransform.YProperty,
            new DoubleAnimation(
                PreviewSurface.SurfaceTranslateTransform.Y,
                0,
                duration)
            {
                EasingFunction = ease,
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
    }

    internal void HideImmediately(bool destroy = false)
    {
        _transitionVersion++;
        _backdrop = null;
        _hasRenderedFrame = false;
        _lastAction = null;
        PreviewSurface.SetTransitionRendering(false);
        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, null);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(ScaleTransform.ScaleXProperty, null);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(ScaleTransform.ScaleYProperty, null);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(TranslateTransform.XProperty, null);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(TranslateTransform.YProperty, null);
        PreviewSurface.SurfaceScaleTransform.ScaleX = 1;
        PreviewSurface.SurfaceScaleTransform.ScaleY = 1;
        PreviewSurface.SurfaceTranslateTransform.X = 0;
        PreviewSurface.SurfaceTranslateTransform.Y = 0;
        PreviewSurface.Opacity = 1;

        if (destroy)
        {
            Close();
        }
        else
        {
            Hide();
        }
    }

    private void CaptureFullBackdrop(NativeMethods.Rect workArea, double dpiX, double dpiY)
    {
        using var performance = PerformanceDiagnostics.Measure(PerformanceMetric.OverlayCapture);
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
            PreviewSurface.BackdropImageElement.Source = null;
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
            PreviewSurface.BackdropImageElement.Source = new CroppedBitmap(_backdrop, new Int32Rect(x, y, width, height));

            // Size and position the image to the crop (physical px mapped to DIU) so
            // Stretch="Fill" is 1:1 and edge crops that lose their blur margin can't
            // shift or stretch the backdrop relative to the screen region.
            var imageLeft = (workArea.Left + x) * scaleX - _workLeft - localX;
            var imageTop = (workArea.Top + y) * scaleY - _workTop - localY;
            PreviewSurface.BackdropImageElement.Width = width * scaleX;
            PreviewSurface.BackdropImageElement.Height = height * scaleY;
            PreviewSurface.BackdropImageElement.Margin = new Thickness(imageLeft, imageTop, 0, 0);

            PreviewSurface.BackdropImageElement.Clip = new RectangleGeometry(
                new Rect(-imageLeft, -imageTop, Math.Max(targetWidth, 1), Math.Max(targetHeight, 1)),
                PreviewSurface.BackdropClipRadius,
                PreviewSurface.BackdropClipRadius);
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
        var transitionVersion = ++_transitionVersion;
        PreviewSurface.SetTransitionRendering(true);
        PreviewSurface.Opacity = 1;
        var scale = PreviewSurface.SurfaceScaleTransform;
        scale.ScaleX = 1;
        scale.ScaleY = 1;
        PreviewSurface.SurfaceTranslateTransform.X = 0;
        PreviewSurface.SurfaceTranslateTransform.Y = 0;
        var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
        var duration = new Duration(TimeSpan.FromMilliseconds(180));
        var opacityAnimation = new DoubleAnimation(0, 1, duration)
        {
            EasingFunction = ease,
            FillBehavior = FillBehavior.Stop
        };
        opacityAnimation.Completed += (_, _) => CompleteTransition(transitionVersion);
        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, opacityAnimation);
        scale.BeginAnimation(
            ScaleTransform.ScaleXProperty,
            new DoubleAnimation(0.98, 1, duration)
            {
                EasingFunction = ease,
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
        scale.BeginAnimation(
            ScaleTransform.ScaleYProperty,
            new DoubleAnimation(0.98, 1, duration)
            {
                EasingFunction = ease,
                FillBehavior = FillBehavior.Stop
            },
            HandoffBehavior.SnapshotAndReplace);
    }

    private void CompleteTransition(long transitionVersion)
    {
        if (transitionVersion != _transitionVersion || !IsVisible)
        {
            return;
        }

        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, null);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(ScaleTransform.ScaleXProperty, null);
        PreviewSurface.SurfaceScaleTransform.BeginAnimation(ScaleTransform.ScaleYProperty, null);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(TranslateTransform.XProperty, null);
        PreviewSurface.SurfaceTranslateTransform.BeginAnimation(TranslateTransform.YProperty, null);
        PreviewSurface.Opacity = 1;
        PreviewSurface.SurfaceScaleTransform.ScaleX = 1;
        PreviewSurface.SurfaceScaleTransform.ScaleY = 1;
        PreviewSurface.SurfaceTranslateTransform.X = 0;
        PreviewSurface.SurfaceTranslateTransform.Y = 0;
        PreviewSurface.SetTransitionRendering(false);
    }

    private static bool SameRect(NativeMethods.Rect left, NativeMethods.Rect right) =>
        left.Left == right.Left && left.Top == right.Top &&
        left.Right == right.Right && left.Bottom == right.Bottom;

}
