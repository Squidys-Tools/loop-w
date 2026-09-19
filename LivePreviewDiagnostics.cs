using System;
using System.IO;

namespace LoopW;

internal static class LivePreviewDiagnostics
{
    private const long MaxLogBytes = 256 * 1024;
    private static readonly object Sync = new();
    private static readonly HashSet<string> RecordedOnce = new(StringComparer.Ordinal);

    internal static void RecordOnce(
        string key,
        string stage,
        string? detail = null,
        Exception? exception = null)
    {
        lock (Sync)
        {
            if (!RecordedOnce.Add(key))
            {
                return;
            }
        }

        Record(stage, detail, exception);
    }

    internal static void Record(string stage, string? detail = null, Exception? exception = null)
    {
        try
        {
            var directory = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "LoopW");
            Directory.CreateDirectory(directory);

            var line = $"{DateTimeOffset.Now:O} stage={stage}";
            if (!string.IsNullOrWhiteSpace(detail))
            {
                line += $" detail={detail}";
            }

            if (exception != null)
            {
                line += $" exception={exception}";
            }

            var path = Path.Combine(directory, "live-preview.log");
            lock (Sync)
            {
                if (File.Exists(path) && new FileInfo(path).Length >= MaxLogBytes)
                {
                    File.WriteAllText(path, line + Environment.NewLine);
                }
                else
                {
                    File.AppendAllText(path, line + Environment.NewLine);
                }
            }
        }
        catch
        {
            // Diagnostics must never affect the preview or the input path.
        }
    }
}
