using System;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Effects;
using System.Windows.Shapes;

namespace LoopW;

public partial class RadialMenuSurface : UserControl
{
    private const double DefaultOverlayScale = 0.7333333333;
    private readonly Path[] _wedgePaths;
    private readonly bool[] _enabledSlots = new bool[RadialConfiguration.SlotCount];
    private int? _selectedSlot;
    private WindowAction? _selectedAction;
    private bool _selectedCenter;
    private bool _centerEnabled = true;

    public RadialMenuSurface()
    {
        InitializeComponent();
        _wedgePaths = new[]
        {
            RightWedge, BottomRightWedge, BottomWedge, BottomLeftWedge,
            LeftWedge, TopLeftWedge, TopWedge, TopRightWedge
        };

        Array.Fill(_enabledSlots, true);

        MouseMove += Surface_MouseMove;
        MouseLeave += Surface_MouseLeave;
    }

    public bool IsPointerSelectionEnabled { get; set; }

    public double Center { get; private set; }

    public double OuterRadius { get; private set; }

    public double InnerRadius { get; private set; }

    public double BlurMargin { get; private set; }

    public Image BackdropImageElement => BackdropImage;

    public int? SelectedSlot => _selectedSlot;

    public void ApplySettings(AppSettings settings, double overlayScale = DefaultOverlayScale)
    {
        OuterRadius = Math.Max(1, settings.RadialOuterRadius * overlayScale);
        InnerRadius = Math.Min(Math.Max(1, settings.RadialInnerRadius * overlayScale), OuterRadius - 1);
        Center = OuterRadius * 1.1;
        BlurMargin = OuterRadius * 0.27;
        Width = Center * 2;
        Height = Center * 2;

        BackdropImage.Width = Width + BlurMargin * 2;
        BackdropImage.Height = Height + BlurMargin * 2;
        BackdropImage.Margin = new Thickness(-BlurMargin);
        BackdropImage.Clip = RadialGeometry.BuildAnnulus(Center + BlurMargin, OuterRadius, InnerRadius);
        BackdropImage.Effect = new BlurEffect
        {
            Radius = Math.Max(1, OuterRadius * 0.2045)
        };

        Ring.Data = RadialGeometry.BuildAnnulus(Center, OuterRadius, InnerRadius);
        CenterHighlight.Data = new EllipseGeometry(new Point(Center, Center), InnerRadius, InnerRadius);
        Ring.Fill = CreateBrush(settings.RadialRingFill, "#B61E1E1E");
        Ring.Effect = settings.IsLightAppearance
            ? new DropShadowEffect
            {
                BlurRadius = 14,
                Direction = 270,
                ShadowDepth = 3,
                Opacity = 0.22,
                Color = Color.FromRgb(0x52, 0x61, 0x6B)
            }
            : null;

        var sectorFill = CreateBrush(settings.RadialSectorFill, "#7A007AFF");
        var sectorStroke = CreateBrush(settings.RadialSectorStroke, "#F0007AFF");
        var strokeThickness = settings.IsLightAppearance ? 0.8 : 1.5;
        CenterHighlight.Fill = sectorFill;
        for (var i = 0; i < RadialActionCatalog.Geometry.Count; i++)
        {
            var slot = RadialActionCatalog.Geometry[i];
            var wedge = _wedgePaths[i];
            wedge.Data = RadialGeometry.BuildWedge(
                Center,
                OuterRadius,
                InnerRadius,
                slot.FromDegrees,
                slot.ToDegrees);
            wedge.Fill = sectorFill;
            wedge.Stroke = sectorStroke;
            wedge.StrokeThickness = strokeThickness;
            wedge.BeginAnimation(UIElement.OpacityProperty, null);
            wedge.Opacity = 0;
        }

        SetSelectableSlots(null);
        SetCenterEnabled(true);
        SetSelectedSlot(null, animate: false);
        SetSelectedCenter(false, animate: false);
    }

    public WindowAction? ActionAt(Point point)
    {
        var index = SlotAt(point);
        return index.HasValue ? RadialActionCatalog.Slots[index.Value].Action : null;
    }

    public int? SlotAt(Point point)
    {
        var dx = point.X - Center;
        var dy = point.Y - Center;
        var distance = Math.Sqrt(dx * dx + dy * dy);
        if (distance < InnerRadius || distance > OuterRadius)
        {
            return null;
        }

        return RadialActionCatalog.IndexAt(Math.Atan2(dy, dx) * 180 / Math.PI);
    }

    public void SetSelectedAction(WindowAction? selection)
    {
        SetSelectedAction(selection, animate: true);
    }

    public void SetSelectableSlots(IReadOnlyList<bool>? enabled)
    {
        for (var i = 0; i < _enabledSlots.Length; i++)
        {
            _enabledSlots[i] = enabled is null || i >= enabled.Count || enabled[i];
        }

        if (_selectedSlot.HasValue && !_enabledSlots[_selectedSlot.Value])
        {
            SetSelectedSlot(null);
        }
    }

    public void SetCenterEnabled(bool enabled)
    {
        _centerEnabled = enabled;
        if (!enabled)
        {
            SetSelectedCenter(false);
        }
    }

    private void Surface_MouseMove(object sender, MouseEventArgs e)
    {
        if (IsPointerSelectionEnabled)
        {
            var point = e.GetPosition(this);
            var slot = SlotAt(point);
            SetSelectedSlot(slot, animate: false);
            SetSelectedCenter(_centerEnabled && IsCenterAt(point), animate: false);
        }
    }

    private void Surface_MouseLeave(object sender, MouseEventArgs e)
    {
        if (IsPointerSelectionEnabled)
        {
            SetSelectedSlot(null, animate: false);
            SetSelectedCenter(false, animate: false);
        }
    }

    private void SetSelectedAction(WindowAction? selection, bool animate)
    {
        if (_selectedAction == selection && animate)
        {
            return;
        }

        var previous = _selectedAction;
        _selectedAction = selection;
        for (var i = 0; i < RadialActionCatalog.Geometry.Count; i++)
        {
            var action = RadialActionCatalog.Slots[i].Action;
            if (!animate || action == previous || action == selection)
            {
                SetWedgeState(_wedgePaths[i], action == selection && _enabledSlots[i], animate);
            }
        }
    }

    public void SetSelectedSlot(int? selection)
    {
        SetSelectedSlot(selection, animate: true);
    }

    private void SetSelectedSlot(int? selection, bool animate)
    {
        if (selection.HasValue &&
            (selection.Value < 0 || selection.Value >= _enabledSlots.Length || !_enabledSlots[selection.Value]))
        {
            selection = null;
        }

        if (_selectedSlot == selection && animate)
        {
            return;
        }

        var previous = _selectedSlot;
        _selectedSlot = selection;
        _selectedAction = selection.HasValue ? RadialActionCatalog.Slots[selection.Value].Action : null;
        if (previous.HasValue)
        {
            SetWedgeState(_wedgePaths[previous.Value], false, animate);
        }

        if (selection.HasValue)
        {
            SetWedgeState(_wedgePaths[selection.Value], true, animate);
        }
    }

    public void SetSelectedCenter(bool selected)
    {
        SetSelectedCenter(selected, animate: true);
    }

    private void SetSelectedCenter(bool selected, bool animate)
    {
        if (_selectedCenter == selected && animate)
        {
            return;
        }

        _selectedCenter = selected;
        SetWedgeState(CenterHighlight, selected, animate);
    }

    private bool IsCenterAt(Point point)
    {
        var dx = point.X - Center;
        var dy = point.Y - Center;
        return Math.Sqrt(dx * dx + dy * dy) < InnerRadius;
    }

    private static void SetWedgeState(Path wedge, bool selected, bool animate)
    {
        var target = selected ? 1 : 0;
        wedge.BeginAnimation(UIElement.OpacityProperty, null);
        var from = wedge.Opacity;
        wedge.Opacity = target;

        if (!animate || Math.Abs(from - target) < 0.001)
        {
            return;
        }

        var animation = new DoubleAnimation(from, target, new Duration(TimeSpan.FromMilliseconds(130)))
        {
            EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut },
            FillBehavior = FillBehavior.Stop
        };
        wedge.BeginAnimation(UIElement.OpacityProperty, animation, HandoffBehavior.SnapshotAndReplace);
    }

    private static Brush CreateBrush(string? value, string fallback)
    {
        try
        {
            if (new BrushConverter().ConvertFromString(value ?? string.Empty) is Brush brush)
            {
                brush.Freeze();
                return brush;
            }
        }
        catch
        {
            // Invalid in-memory values use a safe fallback.
        }

        return new BrushConverter().ConvertFromString(fallback) as Brush ?? Brushes.Transparent;
    }
}
