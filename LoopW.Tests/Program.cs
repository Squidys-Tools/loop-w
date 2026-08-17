using System.Text.Json;
using System.Windows.Media;

namespace LoopW.Tests;

internal static class Program
{
    private static readonly (string Name, Action Test)[] Tests =
    {
        ("radial catalog maps each octant", RadialCatalogMapsEachOctant),
        ("radial catalog exposes eight unique actions", RadialCatalogHasEightUniqueActions),
        ("zone frames cover expected halves and quarters", ZoneFramesCoverExpectedAreas),
        ("drag snap resolves monitor edges and corners", DragSnapResolvesEdgesAndCorners),
        ("drag snap ignores the monitor interior", DragSnapIgnoresMonitorInterior),
        ("zone frames split thirds without gaps", ZoneFramesSplitThirdsWithoutGaps),
        ("zone frames include center halves and fourths", ZoneFramesIncludeCenterHalvesAndFourths),
        ("axis maximize preserves the other dimension", AxisMaximizePreservesOtherDimension),
        ("center frame preserves the current size", CenterFramePreservesCurrentSize),
        ("fit frame clamps size and keeps right edge anchored", FitFrameClampsAndAnchors),
        ("manipulation frame scales by DPI", ManipulationFrameScalesByDpi),
        ("scale and axis manipulation stay centered", ScaleAndAxisManipulationStayCentered),
        ("fill available frame avoids neighboring windows", FillAvailableFrameAvoidsObstacles),
        ("directional navigation chooses the nearest window", DirectionalNavigationChoosesNearestWindow),
        ("stack navigation wraps to the next window", StackNavigationWraps),
        ("stash frames keep a visible edge peek", StashFramesKeepVisiblePeek),
        ("stash identity matching rejects ambiguity", StashIdentityMatchingRejectsAmbiguity),
        ("stash settings normalize safely", StashSettingsNormalizeSafely),
        ("stash settings keep newest records", StashSettingsKeepNewestRecords),
        ("same-monitor DPI changes preserve stash frames", SameMonitorDpiChangesPreserveStashFrames),
        ("same-monitor work-area changes rebase stash frames", SameMonitorWorkAreaChangesRebaseStashFrames),
        ("screen padding combines global and edge values", ScreenPaddingCombinesGlobalAndEdges),
        ("logical monitor moves scale frame size", LogicalMonitorMovesScaleFrameSize),
        ("screen and exclusion settings normalize", ScreenAndExclusionSettingsNormalize),
        ("radial geometry creates annulus and wedge paths", RadialGeometryCreatesPaths),
        ("command parser maps direction aliases", CommandParserMapsDirectionAliases),
        ("command parser maps action names", CommandParserMapsActionNames),
        ("action names cover every action", ActionNamesCoverEveryAction),
        ("command parser maps activation", CommandParserMapsActivation),
        ("command parser exposes list commands", CommandParserExposesListCommands),
        ("command parser rejects malformed commands", CommandParserRejectsMalformedCommands),
        ("command formatter includes configured keybinds", CommandFormatterIncludesKeybinds),
        ("trigger settings persist and normalize", TriggerSettingsPersistAndNormalize),
        ("drag snap settings normalize", DragSnapSettingsNormalize),
        ("hotkey names show modifier side", HotkeyNamesShowModifierSide),
        ("command formatter marks trigger bypass", CommandFormatterMarksTriggerBypass),
        ("radial configuration preserves stable keybind targets", RadialConfigurationPreservesStableKeybindTargets),
        ("radial configuration normalizes invalid targets", RadialConfigurationNormalizesInvalidTargets),
        ("radial configuration resolves center and wedge targets", RadialConfigurationResolvesTargets)
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

    private static void DragSnapResolvesEdgesAndCorners()
    {
        var monitor = Rect(0, 0, 1920, 1080);
        var work = Rect(0, 0, 1920, 1040);

        True(DragSnapGeometry.TryResolve(monitor, work, Point(8, 8), 24, out var topLeft));
        Equal(DragSnapZone.TopLeftQuarter, topLeft);
        Equal(WindowAction.TopLeftQuarter, DragSnapGeometry.ActionOf(topLeft));

        True(DragSnapGeometry.TryResolve(monitor, work, Point(1912, 500), 24, out var right));
        Equal(DragSnapZone.RightHalf, right);
        Equal(WindowAction.RightHalf, DragSnapGeometry.ActionOf(right));

        True(DragSnapGeometry.TryResolve(monitor, work, Point(900, 1072), 24, out var bottom));
        Equal(DragSnapZone.BottomHalf, bottom);
    }

    private static void DragSnapIgnoresMonitorInterior()
    {
        True(!DragSnapGeometry.TryResolve(
            Rect(0, 0, 1920, 1080),
            Rect(0, 0, 1920, 1040),
            Point(960, 500),
            24,
            out _));
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

    private static void ZoneFramesIncludeCenterHalvesAndFourths()
    {
        var work = Rect(0, 0, 1200, 800);

        Equal(Rect(300, 0, 900, 800), WindowFrameMath.ZoneFrame(work, WindowAction.HorizontalCenterHalf));
        Equal(Rect(0, 200, 1200, 600), WindowFrameMath.ZoneFrame(work, WindowAction.VerticalCenterHalf));
        Equal(Rect(0, 0, 300, 800), WindowFrameMath.ZoneFrame(work, WindowAction.FirstFourth));
        Equal(Rect(300, 0, 600, 800), WindowFrameMath.ZoneFrame(work, WindowAction.SecondFourth));
        Equal(Rect(600, 0, 900, 800), WindowFrameMath.ZoneFrame(work, WindowAction.ThirdFourth));
        Equal(Rect(900, 0, 1200, 800), WindowFrameMath.ZoneFrame(work, WindowAction.FourthFourth));
        Equal(Rect(0, 0, 900, 800), WindowFrameMath.ZoneFrame(work, WindowAction.LeftThreeFourths));
        Equal(Rect(300, 0, 1200, 800), WindowFrameMath.ZoneFrame(work, WindowAction.RightThreeFourths));
    }

    private static void AxisMaximizePreservesOtherDimension()
    {
        var work = Rect(0, 0, 1200, 800);
        var current = Rect(100, 120, 500, 620);

        Equal(Rect(100, 0, 500, 800), WindowFrameMath.MaximizeHeightFrame(work, current));
        Equal(Rect(0, 120, 1200, 620), WindowFrameMath.MaximizeWidthFrame(work, current));
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

    private static void ScaleAndAxisManipulationStayCentered()
    {
        var work = Rect(0, 0, 1200, 800);
        var current = Rect(100, 100, 500, 400);

        Equal(Rect(80, 85, 520, 415), WindowFrameMath.ManipulateFrame(work, WindowAction.ScaleUp, current, 1));
        Equal(Rect(40, 100, 560, 400), WindowFrameMath.ManipulateFrame(work, WindowAction.GrowHorizontal, current, 1.25));
        Equal(Rect(100, 160, 500, 340), WindowFrameMath.ManipulateFrame(work, WindowAction.ShrinkVertical, current, 1.25));
    }

    private static void FillAvailableFrameAvoidsObstacles()
    {
        var work = Rect(0, 0, 1200, 800);
        var current = Rect(600, 100, 1000, 700);
        var obstacle = Rect(1000, 0, 1200, 800);

        Equal(
            Rect(0, 0, 1000, 800),
            WindowFrameMath.FillAvailableFrame(work, current, new[] { obstacle }));
    }

    private static void DirectionalNavigationChoosesNearestWindow()
    {
        var current = new WindowCandidate(new IntPtr(1), Rect(400, 300, 800, 700));
        var candidates = new[]
        {
            current,
            new WindowCandidate(new IntPtr(2), Rect(800, 320, 1000, 520)),
            new WindowCandidate(new IntPtr(3), Rect(1100, 320, 1300, 520)),
            new WindowCandidate(new IntPtr(4), Rect(600, 800, 800, 1000))
        };

        True(WindowNavigation.TryFindDirectional(
            current.Frame,
            candidates,
            WindowNavigationDirection.Right,
            out var target));
        Equal(new IntPtr(2), target);
    }

    private static void StackNavigationWraps()
    {
        var candidates = new[]
        {
            new WindowCandidate(new IntPtr(1), Rect(0, 0, 200, 200)),
            new WindowCandidate(new IntPtr(2), Rect(200, 0, 400, 200)),
            new WindowCandidate(new IntPtr(3), Rect(400, 0, 600, 200))
        };

        True(WindowNavigation.TryFindNextInStack(candidates, new IntPtr(3), out var target));
        Equal(new IntPtr(1), target);
    }

    private static void StashFramesKeepVisiblePeek()
    {
        var work = Rect(0, 0, 1000, 800);
        var window = Rect(100, 120, 500, 520);

        Equal(StashEdge.Left, WindowStashService.NearestEdge(work, Rect(-2, 120, 398, 520)));
        Equal(Rect(-392, 120, 8, 520), WindowStashService.CalculateStashedFrame(work, window, StashEdge.Left, 8));
        Equal(Rect(992, 120, 1392, 520), WindowStashService.CalculateStashedFrame(work, window, StashEdge.Right, 8));
    }

    private static void StashIdentityMatchingRejectsAmbiguity()
    {
        var record = new StashRecord
        {
            ExecutablePath = @"C:\Apps\Editor.exe",
            ProcessId = 42,
            WindowClass = "EditorWindow",
            Title = "Document"
        };
        var candidates = new[]
        {
            new WindowIdentityCandidate<IntPtr>(
                new IntPtr(1),
                new WindowIdentity(record.ExecutablePath, 42, record.WindowClass, record.Title)),
            new WindowIdentityCandidate<IntPtr>(
                new IntPtr(2),
                new WindowIdentity(record.ExecutablePath, 42, record.WindowClass, "Other document"))
        };

        True(!WindowIdentityMatcher.TryFindUnambiguousMatch(record, candidates, out _));

        var reusedHandle = new[]
        {
            new WindowIdentityCandidate<IntPtr>(
                new IntPtr(1),
                new WindowIdentity(@"C:\Apps\Other.exe", 99, "OtherWindow", record.Title))
        };
        True(!WindowIdentityMatcher.TryFindUnambiguousMatch(record, reusedHandle, out _));

        True(WindowIdentityMatcher.TryFindUnambiguousMatch(
            record,
            new[] { candidates[0] },
            out var match));
        Equal(new IntPtr(1), match);
    }

    private static void StashSettingsNormalizeSafely()
    {
        var settings = new AppSettings
        {
            StashEdgePeek = 1000,
            StashHitZone = -1,
            StashRevealDelayMilliseconds = 99999,
            StashRecords = new List<StashRecord>
            {
                new() { Id = "duplicate" },
                new() { Id = "duplicate", Edge = (StashEdge)999 },
                null!
            }
        };

        settings.Normalize();

        Equal(48, settings.StashEdgePeek);
        Equal(1, settings.StashHitZone);
        Equal(2000, settings.StashRevealDelayMilliseconds);
        Equal(2, settings.StashRecords.Count);
        True(settings.StashRecords.All(record => !string.IsNullOrWhiteSpace(record.Id)));
        True(settings.StashRecords.All(record => Enum.IsDefined(record.Edge)));
    }

    private static void StashSettingsKeepNewestRecords()
    {
        var settings = new AppSettings
        {
            StashRecords = Enumerable.Range(0, 66)
                .Select(index => new StashRecord { Id = $"record-{index}" })
                .ToList()
        };

        settings.Normalize();

        Equal(64, settings.StashRecords.Count);
        Equal("record-2", settings.StashRecords[0].Id);
        Equal("record-65", settings.StashRecords[^1].Id);
    }

    private static void SameMonitorDpiChangesPreserveStashFrames()
    {
        var monitor = StashRect.FromNative(Rect(0, 0, 1920, 1080));
        var work = StashRect.FromNative(Rect(0, 0, 1920, 1040));
        var original = new StashMonitor
        {
            Monitor = monitor,
            Work = work,
            DpiX = 96,
            DpiY = 96
        };
        var changedDpi = new StashMonitor
        {
            Monitor = StashRect.FromNative(Rect(0, 0, 1920, 1080)),
            Work = StashRect.FromNative(Rect(0, 0, 1920, 1040)),
            DpiX = 144,
            DpiY = 144
        };
        var frame = Rect(100, 120, 700, 620);

        Equal(frame, WindowStashService.RebaseRect(frame, original, changedDpi));
    }

    private static void ScreenPaddingCombinesGlobalAndEdges()
    {
        var settings = new AppSettings
        {
            GlobalScreenPadding = 8,
            ScreenPaddingLeft = 4,
            ScreenPaddingTop = 2,
            ScreenPaddingRight = 6,
            ScreenPaddingBottom = 10
        };

        Equal(
            Rect(12, 10, 986, 782),
            MonitorService.ApplyPadding(Rect(0, 0, 1000, 800), settings));
    }

    private static void LogicalMonitorMovesScaleFrameSize()
    {
        var source = new MonitorSnapshot(Rect(0, 0, 1920, 1080), Rect(0, 0, 1920, 1040), 96, 96);
        var target = new MonitorSnapshot(Rect(1920, 0, 3840, 1080), Rect(1920, 0, 3840, 1040), 144, 144);

        Equal(
            Rect(2070, 150, 2970, 900),
            MonitorService.TranslateFrame(
                Rect(100, 100, 700, 600),
                source,
                target,
                MonitorMoveSizePolicy.PreserveLogicalSize));
    }

    private static void SameMonitorWorkAreaChangesRebaseStashFrames()
    {
        var original = new StashMonitor
        {
            Monitor = StashRect.FromNative(Rect(0, 0, 1920, 1080)),
            Work = StashRect.FromNative(Rect(0, 0, 1920, 1040)),
            DpiX = 96,
            DpiY = 96
        };
        var changedWork = new StashMonitor
        {
            Monitor = StashRect.FromNative(Rect(0, 0, 1920, 1080)),
            Work = StashRect.FromNative(Rect(0, 40, 1920, 1040)),
            DpiX = 96,
            DpiY = 96
        };

        Equal(
            Rect(100, 160, 700, 660),
            WindowStashService.RebaseRect(Rect(100, 120, 700, 620), original, changedWork));
    }

    private static void ScreenAndExclusionSettingsNormalize()
    {
        var settings = new AppSettings
        {
            MonitorMoveSizePolicy = (MonitorMoveSizePolicy)999,
            GlobalScreenPadding = -1,
            ScreenPaddingLeft = 999,
            ExcludedExecutablePaths = new List<string> { " C:\\Apps\\Editor.exe ", "c:\\apps\\editor.exe", "" },
            ExcludedProcessNames = new List<string> { "Editor.exe", "editor", "  " }
        };

        settings.Normalize();

        Equal(MonitorMoveSizePolicy.PreservePixels, settings.MonitorMoveSizePolicy);
        Equal(0, settings.GlobalScreenPadding);
        Equal(128, settings.ScreenPaddingLeft);
        Equal(1, settings.ExcludedExecutablePaths.Count);
        Equal(1, settings.ExcludedProcessNames.Count);
        Equal("Editor", settings.ExcludedProcessNames[0]);
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

        True(LoopCommandParser.TryParse("action/maximize-height", out command, out _));
        Equal(new LoopCommand.Apply(WindowAction.MaximizeHeight), command);

        True(LoopCommandParser.TryParse("action/focus-next-in-stack", out command, out _));
        Equal(new LoopCommand.Apply(WindowAction.FocusNextInStack), command);
    }

    private static void ActionNamesCoverEveryAction()
    {
        var newActions = new[]
        {
            WindowAction.MaximizeHeight,
            WindowAction.MaximizeWidth,
            WindowAction.FillAvailableSpace,
            WindowAction.MinimizeOthers,
            WindowAction.HorizontalCenterHalf,
            WindowAction.VerticalCenterHalf,
            WindowAction.FirstFourth,
            WindowAction.SecondFourth,
            WindowAction.ThirdFourth,
            WindowAction.FourthFourth,
            WindowAction.LeftThreeFourths,
            WindowAction.RightThreeFourths,
            WindowAction.ScaleUp,
            WindowAction.ScaleDown,
            WindowAction.GrowHorizontal,
            WindowAction.GrowVertical,
            WindowAction.ShrinkHorizontal,
            WindowAction.ShrinkVertical,
            WindowAction.FocusUp,
            WindowAction.FocusDown,
            WindowAction.FocusLeft,
            WindowAction.FocusRight,
            WindowAction.FocusNextInStack
        };

        foreach (var action in newActions)
        {
            if (string.Equals(WindowActionService.ActionName(action), action.ToString(), StringComparison.Ordinal))
            {
                throw new InvalidOperationException($"No display name for {action}.");
            }
        }
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

    private static void TriggerSettingsPersistAndNormalize()
    {
        var settings = new AppSettings
        {
            TriggerModifierSide = TriggerModifierSide.Right,
            TriggerDelayMilliseconds = 250,
            TriggerTimeoutMilliseconds = 1500,
            DoubleClickToTrigger = true,
            MiddleClickToTrigger = true,
            Keybinds = new List<Keybind>
            {
                new(NativeMethods.ModControl, 0x52, WindowAction.RightHalf)
                {
                    BypassTrigger = true
                }
            }
        };

        var restored = JsonSerializer.Deserialize<AppSettings>(JsonSerializer.Serialize(settings));
        True(restored != null);
        Equal(TriggerModifierSide.Right, restored!.TriggerModifierSide);
        Equal(250, restored.TriggerDelayMilliseconds);
        Equal(1500, restored.TriggerTimeoutMilliseconds);
        True(restored.DoubleClickToTrigger);
        True(restored.MiddleClickToTrigger);
        True(restored.Keybinds[0].BypassTrigger);

        restored.TriggerModifierSide = (TriggerModifierSide)99;
        restored.TriggerDelayMilliseconds = -1;
        restored.TriggerTimeoutMilliseconds = 99999;
        restored.Normalize();
        Equal(TriggerModifierSide.Any, restored.TriggerModifierSide);
        Equal(0, restored.TriggerDelayMilliseconds);
        Equal(10000, restored.TriggerTimeoutMilliseconds);
    }

    private static void DragSnapSettingsNormalize()
    {
        var settings = new AppSettings { DragSnapThreshold = 1000 };
        settings.Normalize();
        Equal(96, settings.DragSnapThreshold);

        settings.DragSnapThreshold = -1;
        settings.Normalize();
        Equal(4, settings.DragSnapThreshold);
        True(settings.DragSnapEnabled);
        True(settings.RestorePreDragFrameOnSnapCancel);
    }

    private static void HotkeyNamesShowModifierSide()
    {
        Equal(
            "Right Ctrl + B",
            HotkeyNames.For(NativeMethods.ModControl, 0x42, TriggerModifierSide.Right));
        Equal(
            "Left Ctrl + B",
            HotkeyNames.For(NativeMethods.ModControl, 0x42, TriggerModifierSide.Left));
    }

    private static void CommandFormatterMarksTriggerBypass()
    {
        var keybinds = new List<Keybind>
        {
            new(NativeMethods.ModControl, 0x52, WindowAction.RightHalf)
            {
                BypassTrigger = true
            }
        };

        var text = LoopCommandFormatter.Keybinds(
            keybinds,
            NativeMethods.ModControl,
            NativeMethods.VkCapital,
            TriggerModifierSide.Right);
        True(text.Contains("trigger: Right Ctrl + Caps Lock", StringComparison.Ordinal));
        True(text.Contains("(bypass trigger)", StringComparison.Ordinal));
    }

    private static void RadialConfigurationPreservesStableKeybindTargets()
    {
        var keybind = new Keybind(NativeMethods.ModControl, 0x52, WindowAction.RightHalf);
        var settings = new AppSettings
        {
            Keybinds = new List<Keybind> { keybind },
            RadialSlots = RadialConfiguration.CreateDefaultSlots(),
            CenterTarget = new RadialTargetSettings
            {
                Kind = RadialTargetKind.Keybind,
                KeybindId = keybind.Id,
                CycleEnabled = true
            }
        };

        settings.Normalize();
        var resolved = RadialTargetResolver.Resolve(settings.CenterTarget, settings.Keybinds);
        True(resolved is RadialTarget.KeybindAction keybindTarget &&
            ReferenceEquals(keybind, keybindTarget.Binding));

        settings.Keybinds.Reverse();
        settings.Normalize();
        resolved = RadialTargetResolver.Resolve(settings.CenterTarget, settings.Keybinds);
        True(resolved is RadialTarget.KeybindAction keybindTargetAfterReorder &&
            ReferenceEquals(keybind, keybindTargetAfterReorder.Binding));
    }

    private static void RadialConfigurationNormalizesInvalidTargets()
    {
        var settings = new AppSettings
        {
            RadialSlots = new List<RadialTargetSettings>
            {
                new() { Kind = RadialTargetKind.Action, Action = (WindowAction)999 },
                new() { Kind = RadialTargetKind.Keybind, KeybindId = "missing" }
            },
            CenterTarget = new RadialTargetSettings
            {
                Kind = (RadialTargetKind)999
            }
        };

        settings.Normalize();
        Equal(RadialConfiguration.SlotCount, settings.RadialSlots.Count);
        Equal(RadialTargetKind.None, settings.RadialSlots[0].Kind);
        Equal(RadialTargetKind.None, settings.RadialSlots[1].Kind);
        Equal(RadialTargetKind.None, settings.CenterTarget.Kind);
    }

    private static void RadialConfigurationResolvesTargets()
    {
        var keybind = new Keybind(0, 0x46, WindowAction.FocusRight);
        var slots = RadialConfiguration.CreateDefaultSlots();
        slots[0] = new RadialTargetSettings
        {
            Kind = RadialTargetKind.Action,
            Action = WindowAction.Maximize,
            CycleEnabled = false
        };
        slots[4] = new RadialTargetSettings
        {
            Kind = RadialTargetKind.Keybind,
            KeybindId = keybind.Id,
            CycleEnabled = true
        };

        var targets = RadialTargetResolver.ResolveSlots(slots, new[] { keybind });
        True(targets[0] is RadialTarget.BuiltInAction { Value: WindowAction.Maximize, CycleEnabled: false });
        True(targets[4] is RadialTarget.KeybindAction { Binding.Action: WindowAction.FocusRight, CycleEnabled: true });
        True(RadialTargetResolver.Resolve(
            RadialConfiguration.CreateDefaultCenter(),
            Array.Empty<Keybind>()) is RadialTarget.None);
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
