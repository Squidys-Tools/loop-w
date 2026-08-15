using System.IO.Pipes;
using System.IO;
using System.Text;

namespace LoopW;

internal sealed class LoopCommandServer : IAsyncDisposable
{
    internal const string PipeName = "LoopW-Commands";

    private readonly Func<string, Task<string>> _handler;
    private readonly CancellationTokenSource _cancellation = new();
    private Task? _serverTask;

    internal LoopCommandServer(Func<string, Task<string>> handler)
    {
        _handler = handler;
    }

    internal void Start()
    {
        _serverTask ??= RunAsync(_cancellation.Token);
    }

    internal void Stop() => _cancellation.Cancel();

    public async ValueTask DisposeAsync()
    {
        Stop();
        if (_serverTask != null)
        {
            try
            {
                await _serverTask.ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                // Normal shutdown.
            }
        }

        _cancellation.Dispose();
    }

    private async Task RunAsync(CancellationToken cancellation)
    {
        while (!cancellation.IsCancellationRequested)
        {
            try
            {
                await using var pipe = new NamedPipeServerStream(
                    PipeName,
                    PipeDirection.InOut,
                    1,
                    PipeTransmissionMode.Byte,
                    PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly);

                await pipe.WaitForConnectionAsync(cancellation).ConfigureAwait(false);
                using var reader = new StreamReader(pipe, Encoding.UTF8, leaveOpen: true);
                await using var writer = new StreamWriter(pipe, new UTF8Encoding(false), leaveOpen: true)
                {
                    AutoFlush = true
                };

                var rawCommand = await reader.ReadLineAsync(cancellation).ConfigureAwait(false);
                if (rawCommand != null)
                {
                    string response;
                    try
                    {
                        response = await _handler(rawCommand).ConfigureAwait(false);
                    }
                    catch (Exception)
                    {
                        response = "ERROR: command execution failed.";
                    }

                    await writer.WriteLineAsync(response).ConfigureAwait(false);
                }
            }
            catch (OperationCanceledException) when (cancellation.IsCancellationRequested)
            {
                return;
            }
            catch (IOException) when (!cancellation.IsCancellationRequested)
            {
                // A client can disconnect mid-command. Keep the resident server alive.
            }
        }
    }
}

internal static class LoopCommandClient
{
    internal static bool TrySend(string command, out string response)
    {
        response = string.Empty;
        for (var attempt = 0; attempt < 3; attempt++)
        {
            try
            {
                using var pipe = new NamedPipeClientStream(
                    ".",
                    LoopCommandServer.PipeName,
                    PipeDirection.InOut,
                    PipeOptions.CurrentUserOnly);
                pipe.Connect(250);
                using var reader = new StreamReader(pipe, Encoding.UTF8, leaveOpen: true);
                using var writer = new StreamWriter(pipe, new UTF8Encoding(false), leaveOpen: true)
                {
                    AutoFlush = true
                };
                writer.WriteLine(command);
                response = reader.ReadLine() ?? string.Empty;
                return true;
            }
            catch (IOException) when (attempt < 2)
            {
                Thread.Sleep(50);
            }
            catch (TimeoutException) when (attempt < 2)
            {
                Thread.Sleep(50);
            }
            catch (IOException)
            {
                return false;
            }
            catch (TimeoutException)
            {
                return false;
            }
            catch (UnauthorizedAccessException)
            {
                return false;
            }
        }

        return false;
    }
}
