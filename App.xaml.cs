using System.Windows;
using Application = System.Windows.Application;

namespace LoopW;

public partial class App : Application
{
    private const string InstanceMutexName = "Local\\LoopW.Instance";
    private const string ActivationEventName = "Local\\LoopW.Activate";

    private Mutex? _instanceMutex;
    private EventWaitHandle? _activationEvent;
    private RegisteredWaitHandle? _activationRegistration;
    private MainWindow? _mainWindow;
    private TrayIcon? _trayIcon;
    private LoopCommandServer? _commandServer;
    private string? _startupCommand;
    private bool _ownsInstance;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);
        ShutdownMode = ShutdownMode.OnExplicitShutdown;

        if (e.Args.Length > 0 && LoopCommandClient.TrySend(string.Join(' ', e.Args), out var existingResponse))
        {
            WriteCliResponse(existingResponse);
            Shutdown();
            return;
        }

        if (e.Args.Length > 0)
        {
            _startupCommand = string.Join(' ', e.Args);
        }

        if (!TryAcquireInstance())
        {
            Shutdown();
            return;
        }

        _activationEvent = new EventWaitHandle(
            false,
            EventResetMode.AutoReset,
            ActivationEventName,
            out _);
        _activationRegistration = ThreadPool.RegisterWaitForSingleObject(
            _activationEvent,
            (_, _) => Dispatcher.BeginInvoke(ShowMainWindow),
            null,
            Timeout.Infinite,
            executeOnlyOnce: false);

        _mainWindow = new MainWindow();
        MainWindow = _mainWindow;
        _mainWindow.Show();
        _mainWindow.Hide();

        _trayIcon = new TrayIcon(
            ShowMainWindow,
            OpenSettings,
            QuitApplication);

        _commandServer = new LoopCommandServer(HandleCommandAsync);
        _commandServer.Start();

        if (_startupCommand != null)
        {
            var startupCommand = _startupCommand;
            Dispatcher.BeginInvoke(() => WriteCliResponse(ExecuteCommand(startupCommand)));
        }
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _activationRegistration?.Unregister(null);
        _commandServer?.Stop();
        _ = _commandServer?.DisposeAsync().AsTask();
        _trayIcon?.Dispose();
        WindowStashService.RestoreAll();

        if (_mainWindow != null)
        {
            _mainWindow.AllowClose();
            _mainWindow.Close();
        }

        _activationEvent?.Dispose();

        if (_ownsInstance)
        {
            _instanceMutex?.ReleaseMutex();
        }

        _instanceMutex?.Dispose();
        base.OnExit(e);
    }

    private bool TryAcquireInstance()
    {
        try
        {
            _instanceMutex = new Mutex(true, InstanceMutexName, out var createdNew);
            _ownsInstance = createdNew;
            if (createdNew)
            {
                return true;
            }

            if (LoopCommandClient.TrySend("activate", out _))
            {
                _instanceMutex.Dispose();
                _instanceMutex = null;
                return false;
            }

            TryActivateExistingInstance();
            _instanceMutex.Dispose();
            _instanceMutex = null;
            return false;
        }
        catch
        {
            TryActivateExistingInstance();
            return false;
        }
    }

    private static void TryActivateExistingInstance()
    {
        try
        {
            using var activation = EventWaitHandle.OpenExisting(ActivationEventName);
            activation.Set();
        }
        catch (WaitHandleCannotBeOpenedException)
        {
            // The first process may still be creating its activation event.
        }
        catch (UnauthorizedAccessException)
        {
            // A different security boundary owns the named event.
        }
    }

    private void ShowMainWindow()
    {
        _mainWindow?.ShowFromTray();
    }

    private void OpenSettings()
    {
        _mainWindow?.OpenSettingsFromTray();
    }

    private void QuitApplication()
    {
        Shutdown();
    }

    private Task<string> HandleCommandAsync(string rawCommand) =>
        Dispatcher.InvokeAsync(() => ExecuteCommand(rawCommand)).Task;

    private string ExecuteCommand(string rawCommand)
    {
        if (!LoopCommandParser.TryParse(rawCommand, out var command, out var error) || command == null)
        {
            return $"ERROR: {error}";
        }

        return command switch
        {
            LoopCommand.Activate => ActivateMainWindow(),
            LoopCommand.Apply apply => _mainWindow?.ExecuteExternalAction(apply.Action)
                ?? "The LoopW window is not ready.",
            LoopCommand.ListActions => LoopCommandFormatter.Actions(),
            LoopCommand.ListKeybinds => _mainWindow?.DescribeKeybinds()
                ?? "The LoopW window is not ready.",
            LoopCommand.ListAll => _mainWindow?.DescribeAll()
                ?? "The LoopW window is not ready.",
            _ => "ERROR: Unsupported command."
        };
    }

    private string ActivateMainWindow()
    {
        ShowMainWindow();
        return "LoopW activated.";
    }

    private static void WriteCliResponse(string response)
    {
        if (!NativeMethods.AttachConsole(NativeMethods.AttachParentProcess))
        {
            return;
        }

        try
        {
            Console.WriteLine(response);
        }
        finally
        {
            NativeMethods.FreeConsole();
        }
    }
}
