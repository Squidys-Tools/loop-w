using System.Diagnostics;
using System.Globalization;
using System.Text;

namespace LoopW;

internal enum PerformanceMetric
{
    Startup,
    HotkeyHook,
    MouseHook,
    StashPoll,
    OverlayCapture,
    ActionPlacement,
    SettingsSave,
    SettingsApply
}

internal static class PerformanceDiagnostics
{
    private static readonly MetricState[] States = CreateStates();

    public static bool Enabled { get; } = IsEnabledFromEnvironment();

    public static Measurement Measure(PerformanceMetric metric) =>
        Enabled ? new Measurement(States[(int)metric], Stopwatch.GetTimestamp()) : default;

    public static void WriteSummary()
    {
        if (!Enabled)
        {
            return;
        }

        var summary = BuildSummary();
        Trace.WriteLine(summary);
        Debug.WriteLine(summary);
    }

    private static string BuildSummary()
    {
        var builder = new StringBuilder("LoopW performance diagnostics\n");
        foreach (var metric in Enum.GetValues<PerformanceMetric>())
        {
            var state = States[(int)metric];
            var count = Interlocked.Read(ref state.Count);
            var totalTicks = Interlocked.Read(ref state.TotalTicks);
            var maxTicks = Interlocked.Read(ref state.MaxTicks);
            var averageMilliseconds = count == 0
                ? 0
                : totalTicks * 1000.0 / Stopwatch.Frequency / count;
            var maximumMilliseconds = maxTicks * 1000.0 / Stopwatch.Frequency;

            builder.Append("  ")
                .Append(metric)
                .Append(": count=")
                .Append(count.ToString(CultureInfo.InvariantCulture))
                .Append(", avg=")
                .Append(averageMilliseconds.ToString("F2", CultureInfo.InvariantCulture))
                .Append(" ms, max=")
                .Append(maximumMilliseconds.ToString("F2", CultureInfo.InvariantCulture))
                .AppendLine(" ms");
        }

        return builder.ToString();
    }

    private static MetricState[] CreateStates()
    {
        var states = new MetricState[Enum.GetValues<PerformanceMetric>().Length];
        for (var i = 0; i < states.Length; i++)
        {
            states[i] = new MetricState();
        }

        return states;
    }

    private static bool IsEnabledFromEnvironment()
    {
        var value = Environment.GetEnvironmentVariable("LOOPW_PERF");
        return string.Equals(value, "1", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(value, "true", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(value, "yes", StringComparison.OrdinalIgnoreCase);
    }

    internal sealed class MetricState
    {
        public long Count;
        public long TotalTicks;
        public long MaxTicks;
    }

    internal readonly struct Measurement : IDisposable
    {
        private readonly MetricState? _state;
        private readonly long _startedAt;

        internal Measurement(MetricState state, long startedAt)
        {
            _state = state;
            _startedAt = startedAt;
        }

        public void Dispose()
        {
            if (_state == null)
            {
                return;
            }

            var elapsed = Math.Max(0, Stopwatch.GetTimestamp() - _startedAt);
            Interlocked.Increment(ref _state.Count);
            Interlocked.Add(ref _state.TotalTicks, elapsed);

            long observed;
            do
            {
                observed = Interlocked.Read(ref _state.MaxTicks);
                if (observed >= elapsed)
                {
                    return;
                }
            }
            while (Interlocked.CompareExchange(ref _state.MaxTicks, elapsed, observed) != observed);
        }
    }
}
