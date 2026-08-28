using System;
using System.IO;

namespace LoopW;

internal static class LivePreviewDiagnostics
{
    private static readonly object Sync = new();

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

            lock (Sync)
            {
                File.AppendAllText(
                    Path.Combine(directory, "live-preview.log"),
                    line + Environment.NewLine);
            }
        }
        catch
        {
            // Diagnostics must never affect the preview or the input path.
        }
    }
}
