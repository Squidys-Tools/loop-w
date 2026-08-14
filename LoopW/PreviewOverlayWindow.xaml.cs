using System;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Animation;

namespace LoopW;

public partial class PreviewOverlayWindow : Window
{
    public PreviewOverlayWindow()
    {
        InitializeComponent();
    }

    internal void ShowFrame(NativeMethods.Rect frame, WindowHalf action)
    {
        var wasVisible = IsVisible;
        var oldLeft = Left;
        var oldTop = Top;
        var oldWidth = Width;
        var oldHeight = Height;

        BeginAnimation(LeftProperty, null);
        BeginAnimation(TopProperty, null);
        BeginAnimation(WidthProperty, null);
        BeginAnimation(HeightProperty, null);

        Left = frame.Left;
        Top = frame.Top;
        Width = Math.Max(frame.Width, 1);
        Height = Math.Max(frame.Height, 1);
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
            Show();
            AnimateSurfaceIn();
        }
        else
        {
            var ease = new CubicEase { EasingMode = EasingMode.EaseOut };
            var duration = new Duration(TimeSpan.FromMilliseconds(140));
            BeginAnimation(LeftProperty, new DoubleAnimation(oldLeft, Left, duration) { EasingFunction = ease });
            BeginAnimation(TopProperty, new DoubleAnimation(oldTop, Top, duration) { EasingFunction = ease });
            BeginAnimation(WidthProperty, new DoubleAnimation(oldWidth, Width, duration) { EasingFunction = ease });
            BeginAnimation(HeightProperty, new DoubleAnimation(oldHeight, Height, duration) { EasingFunction = ease });
        }
    }

    internal void HideAnimated()
    {
        if (!IsVisible)
        {
            return;
        }

        var animation = new DoubleAnimation(PreviewSurface.Opacity, 0, new Duration(TimeSpan.FromMilliseconds(100)))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseIn }
        };
        animation.Completed += (_, _) => Hide();
        PreviewSurface.BeginAnimation(UIElement.OpacityProperty, animation);
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
