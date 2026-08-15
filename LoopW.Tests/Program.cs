using System.Windows.Media;

namespace LoopW.Tests;

internal static class Program
{
    private static readonly (string Name, Action Test)[] Tests =
    {
        ("radial catalog maps each octant", RadialCatalogMapsEachOctant),
        ("radial catalog exposes eight unique actions", RadialCatalogHasEightUniqueActions),
        ("zone frames cover expected halves and quarters", ZoneFramesCoverExpectedAreas),
        ("zone frames split thirds without gaps", ZoneFramesSplitThirdsWithoutGaps),
        ("center frame preserves the current size", CenterFramePreservesCurrentSize),
        ("fit frame clamps size and keeps right edge anchored", FitFrameClampsAndAnchors),
        ("manipulation frame scales by DPI", ManipulationFrameScalesByDpi),
        ("stash frames keep a visible edge peek", StashFramesKeepVisiblePeek),
        ("radial geometry creates annulus and wedge paths", RadialGeometryCreatesPaths),
        ("command parser maps direction aliases", CommandParserMapsDirectionAliases),
        ("command parser maps action names", CommandParserMapsActionNames),
        ("command parser maps activation", CommandParserMapsActivation),
        ("command parser exposes list commands", CommandParserExposesListCommands),
        ("command parser rejects malformed commands", CommandParserRejectsMalformedCommands),
        ("command formatter includes configured keybinds", CommandFormatterIncludesKeybinds)
    };

    public static int Main()
    {
        var failures = 0;
        foreach (var (name, test) in Tests)
        {
            try
            {
                test();
                Console.WriteLine($"PASS  {name}");
            }
            catch (Exception error)
            {
                failures++;
                Console.WriteLine($"FAIL  {name}: {error.Message}");
            }
        }

        Console.WriteLine(failures == 0
            ? $"All {Tests.Length} tests passed."
            : $"{failures} of {Tests.Length} tests failed.");
        return failures == 0 ? 0 : 1;
    }

    private static void RadialCatalogMapsEachOctant()
    {
        Equal(WindowAction.RightHalf, RadialActionCatalog.ActionAt(0));
        Equal(WindowAction.BottomRightQuarter, RadialActionCatalog.ActionAt(45));
        Equal(WindowAction.BottomHalf, RadialActionCatalog.ActionAt(90));
        Equal(WindowAction.BottomLeftQuarter, RadialActionCatalog.ActionAt(135));
        Equal(WindowAction.LeftHalf, RadialActionCatalog.ActionAt(180));
        Equal(WindowAction.TopLeftQuarter, RadialActionCatalog.ActionAt(225));
        Equal(WindowAction.TopHalf, RadialActionCatalog.ActionAt(270));
        Equal(WindowAction.TopRightQuarter, RadialActionCatalog.ActionAt(315));
        Equal(WindowAction.TopHalf, RadialActionCatalog.ActionAt(-90));
    }

    private static void RadialCatalogHasEightUniqueActions()
    {
        Equal(8, RadialActionCatalog.Slots.Count);
        Equal(8, RadialActionCatalog.Slots.Select(slot => slot.Action).Distinct().Count());
    }

    private static void ZoneFramesCoverExpectedAreas()
    {
        var work = Rect(0, 0, 1200, 800);

        Equal(Rect(0, 0, 600, 800), WindowFrameMath.ZoneFrame(work, WindowAction.LeftHalf));
        Equal(Rect(600, 0, 1200, 800), WindowFrameMath.ZoneFrame(work, WindowAction.RightHalf));
        Equal(Rect(0, 0, 600, 400), WindowFrameMath.ZoneFrame(work, WindowAction.TopLeftQuarter));
        Equal(Rect(600, 400, 1200, 800), WindowFrameMath.ZoneFrame(work, WindowAction.BottomRightQuarter));
    }

    private static void ZoneFramesSplitThirdsWithoutGaps()
    {
        var work = Rect(0, 0, 1200, 900);

        var left = WindowFrameMath.ZoneFrame(work, WindowAction.LeftThird);
        var center = WindowFrameMath.ZoneFrame(work, WindowAction.HorizontalCenterThird);
        var right = WindowFrameMath.ZoneFrame(work, WindowAction.RightThird);

        Equal(400, left.Width);
        Equal(400, center.Width);
        Equal(400, right.Width);
        Equal(left.Right, center.Left);
        Equal(center.Right, right.Left);
        Equal(work.Right, right.Right);
    }

    private static void CenterFramePreservesCurrentSize()
    {
        var work = Rect(0, 0, 1200, 800);
        var current = Rect(100, 120, 700, 620);

        Equal(Rect(300, 150, 900, 650), WindowFrameMath.CenterFrame(work, current));
    }

    private static void FitFrameClampsAndAnchors()
    {
        var bounds = Rect(0, 0, 1000, 800);
        var requested = Rect(-100, -100, 1200, 900);
        var limits = new NativeMethods.MinMaxInfo
        {
            MinTrackSize = Point(600, 500),
            MaxTrackSize = Point(700, 650)
        };

        Equal(Rect(300, 0, 1000, 650), WindowFrameMath.FitFrame(bounds, WindowAction.RightHalf, requested, limits));
    }

    private static void ManipulationFrameScalesByDpi()
    {
        var current = Rect(100, 100, 500, 400);
        var resized = WindowFrameMath.ManipulateFrame(Rect(0, 0, 1200, 800), WindowAction.GrowRight, current, 1.25);

        Equal(Rect(100, 100, 560, 400), resized);
    }

    private static void StashFramesKeepVisiblePeek()
    {
        var work = Rect(0, 0, 1000, 800);
        var window = Rect(100, 120, 500, 520);

        Equal(StashEdge.Left, WindowStashService.NearestEdge(work, Rect(-2, 120, 398, 520)));
        Equal(Rect(-392, 120, 8, 520), WindowStashService.CalculateStashedFrame(work, window, StashEdge.Left, 8));
        Equal(Rect(992, 120, 1392, 520), WindowStashService.CalculateStashedFrame(work, window, StashEdge.Right, 8));
    }

    private static void RadialGeometryCreatesPaths()
    {
        var annulus = RadialGeometry.BuildAnnulus(50, 40, 20);
        var wedge = RadialGeometry.BuildWedge(50, 40, 20, 0, 45);

        Equal(2, annulus.Figures.Count);
        Equal(1, wedge.Figures.Count);
        Equal(4, wedge.Figures[0].Segments.Count);
    }

    private static void CommandParserMapsDirectionAliases()
    {
        True(LoopCommandParser.TryParse("direction/right", out var command, out _));
        Equal(new LoopCommand.Apply(WindowAction.RightHalf), command);

        True(LoopCommandParser.TryParse("direction/prev", out command, out _));
        Equal(new LoopCommand.Apply(WindowAction.PreviousScreen), command);
    }

    private static void CommandParserMapsActionNames()
    {
        True(LoopCommandParser.TryParse("action/bottom-right-quarter", out var command, out _));
        Equal(new LoopCommand.Apply(WindowAction.BottomRightQuarter), command);

        True(LoopCommandParser.TryParse("action/RevealStashed", out command, out _));
        Equal(new LoopCommand.Apply(WindowAction.RevealStashed), command);
    }

    private static void CommandParserExposesListCommands()
    {
        True(LoopCommandParser.TryParse("list/actions", out var actions, out _));
        True(actions is LoopCommand.ListActions);
        True(LoopCommandParser.TryParse("list/keybinds", out var keybinds, out _));
        True(keybinds is LoopCommand.ListKeybinds);
        True(LoopCommandParser.TryParse("list/all", out var all, out _));
        True(all is LoopCommand.ListAll);
    }

    private static void CommandParserMapsActivation()
    {
        True(LoopCommandParser.TryParse("activate", out var command, out _));
        True(command is LoopCommand.Activate);
    }

    private static void CommandParserRejectsMalformedCommands()
    {
        True(!LoopCommandParser.TryParse("direction/sideways", out _, out var error));
        True(error.Contains("Unknown direction", StringComparison.Ordinal));
        True(!LoopCommandParser.TryParse("list/actions extra", out _, out error));
        True(error.Contains("Expected one command", StringComparison.Ordinal));
    }

    private static void CommandFormatterIncludesKeybinds()
    {
        var keybinds = new List<Keybind>
        {
            new(NativeMethods.ModControl, 0x52, WindowAction.RightHalf)
        };

        var text = LoopCommandFormatter.Keybinds(keybinds, NativeMethods.ModShift, NativeMethods.VkCapital);
        True(text.Contains("Shift + Caps Lock", StringComparison.Ordinal));
        True(text.Contains("Ctrl + R -> action/righthalf (cycle)", StringComparison.Ordinal));
    }

    private static NativeMethods.Rect Rect(int left, int top, int right, int bottom) =>
        new() { Left = left, Top = top, Right = right, Bottom = bottom };

    private static NativeMethods.Point Point(int x, int y) =>
        new() { X = x, Y = y };

    private static void Equal<T>(T expected, T actual)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
        {
            throw new InvalidOperationException($"Expected {expected}, got {actual}.");
        }
    }

    private static void True(bool value)
    {
        if (!value)
        {
            throw new InvalidOperationException("Expected condition to be true.");
        }
    }

    private static void Equal(NativeMethods.Rect expected, NativeMethods.Rect actual)
    {
        if (expected.Left != actual.Left || expected.Top != actual.Top ||
            expected.Right != actual.Right || expected.Bottom != actual.Bottom)
        {
            throw new InvalidOperationException(
                $"Expected {Format(expected)}, got {Format(actual)}.");
        }
    }

    private static string Format(NativeMethods.Rect rect) =>
        $"({rect.Left},{rect.Top})-({rect.Right},{rect.Bottom})";
}
