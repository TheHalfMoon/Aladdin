// Aladdin Desktop Foundation — read-only Windows preview.
// This preview never sends screenshots or text to a model, network, or remote device.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;


namespace Aladdin.DesktopPreview
{
    internal enum SafeCommand { Help, Status, Version, Doctor, Devices, Files, Unsupported }

    internal static class CommandRouter
    {
        internal static SafeCommand Classify(string input)
        {
            string value = (input ?? "").Trim().ToLowerInvariant();
            if (value == "help" || value == "?") return SafeCommand.Help;
            if (value == "status") return SafeCommand.Status;
            if (value == "version") return SafeCommand.Version;
            if (value == "doctor") return SafeCommand.Doctor;
            if (value == "devices") return SafeCommand.Devices;
            if (value == "files") return SafeCommand.Files;
            return SafeCommand.Unsupported;
        }

        internal static bool CanInvokeInstalledHost(SafeCommand command)
        {
            return command == SafeCommand.Status || command == SafeCommand.Version || command == SafeCommand.Doctor;
        }

        /// The only arguments ever passed to the runtime: fixed, read-only queries.
        internal static string[] CliArguments(SafeCommand command)
        {
            if (command == SafeCommand.Status) return new[] { "status", "--json" };
            if (command == SafeCommand.Version) return new[] { "version", "--json" };
            if (command == SafeCommand.Doctor) return new[] { "doctor", "--json" };
            return null;
        }
    }

    /// Result of one bounded child-process run.
    internal sealed class CliResult
    {
        internal int ExitCode = -1;
        internal string Stdout = "";
        internal bool TimedOut;
        internal bool Truncated;
        internal bool FailedToStart;
    }

    internal static class BoundedProcess
    {
        internal const int MaxOutputChars = 256 * 1024;

        /// Runs a child with both streams drained concurrently (so a full pipe can
        /// never deadlock it), a bounded amount of output kept, and the whole
        /// process tree killed on timeout.
        internal static CliResult Run(string fileName, string[] arguments, int timeoutMs, string localAppData)
        {
            CliResult result = new CliResult();
            using (Process process = new Process())
            {
                process.StartInfo = new ProcessStartInfo {
                    FileName = fileName,
                    Arguments = string.Join(" ", arguments.Select(Quote).ToArray()),
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    RedirectStandardInput = true,
                    WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)
                };
                if (localAppData != null) process.StartInfo.EnvironmentVariables["LOCALAPPDATA"] = localAppData;
                try { process.Start(); }
                catch (Exception) { result.FailedToStart = true; return result; }
                try { process.StandardInput.Close(); } catch (Exception) { }
                bool stdoutTruncated = false;
                Task<string> stdout = Task.Factory.StartNew(() => ReadBounded(process.StandardOutput, out stdoutTruncated), TaskCreationOptions.LongRunning);
                bool ignored;
                Task<string> stderr = Task.Factory.StartNew(() => ReadBounded(process.StandardError, out ignored), TaskCreationOptions.LongRunning);
                if (!process.WaitForExit(timeoutMs))
                {
                    result.TimedOut = true;
                    KillTree(process.Id);
                    process.WaitForExit(2000);
                }
                Task.WaitAll(new Task[] { stdout, stderr }, 2000);
                result.Stdout = stdout.IsCompleted ? stdout.Result : "";
                result.Truncated = stdoutTruncated;
                if (!result.TimedOut && process.HasExited) result.ExitCode = process.ExitCode;
            }
            return result;
        }

        private static string Quote(string argument)
        {
            return argument.IndexOfAny(new[] { ' ', '"', '\t' }) < 0 ? argument : "\"" + argument.Replace("\"", "\\\"") + "\"";
        }

        private static string ReadBounded(StreamReader reader, out bool truncated)
        {
            truncated = false;
            StringBuilder kept = new StringBuilder();
            char[] buffer = new char[4096];
            int read;
            // Keep reading after the limit so the child never blocks on a full pipe.
            while ((read = reader.Read(buffer, 0, buffer.Length)) > 0)
            {
                int room = MaxOutputChars - kept.Length;
                if (room > 0) kept.Append(buffer, 0, Math.Min(room, read));
                if (read > room) truncated = true;
            }
            return kept.ToString();
        }

        private static void KillTree(int pid)
        {
            try
            {
                using (Process killer = Process.Start(new ProcessStartInfo {
                    FileName = Path.Combine(Environment.SystemDirectory, "taskkill.exe"),
                    Arguments = "/PID " + pid + " /T /F",
                    UseShellExecute = false,
                    CreateNoWindow = true
                }))
                {
                    if (killer != null) killer.WaitForExit(5000);
                }
            }
            catch (Exception) { }
        }
    }

    internal enum RuntimeAvailability { NotInstalled, Unresponsive, Incompatible, Installed }

    /// What the app truthfully knows about the local Aladdin runtime.
    internal sealed class RuntimeStatus
    {
        internal RuntimeAvailability Availability;
        internal string Version;
        internal string State;
        internal string Detail;
        internal int? Workspaces;

        internal string Headline()
        {
            switch (Availability)
            {
                case RuntimeAvailability.NotInstalled: return "Not installed";
                case RuntimeAvailability.Unresponsive: return "Installed • not responding";
                case RuntimeAvailability.Incompatible: return "Installed • incompatible response";
                default: return "Aladdin runtime " + Version + " • " + Describe(State);
            }
        }

        internal static string Describe(string state)
        {
            if (state == "running") return "running";
            if (state == "degraded") return "degraded";
            if (state == "not_running") return "not running";
            return "state unknown";
        }
    }

    internal static class LocalHost
    {
        /// Same resolution as the runtime itself: %LOCALAPPDATA%, then the
        /// known folder. Never PATH, never a model- or user-provided path.
        internal static string LocalAppData()
        {
            string fromEnvironment = Environment.GetEnvironmentVariable("LOCALAPPDATA");
            if (!string.IsNullOrEmpty(fromEnvironment) && Path.IsPathRooted(fromEnvironment)) return fromEnvironment;
            return Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        }

        internal static string PathToCompatCli()
        {
            return Path.Combine(LocalAppData(), "Qdral", "bin", "qdral.exe");
        }

        internal static bool IsInstalled()
        {
            return File.Exists(PathToCompatCli());
        }

        internal static CliResult Query(SafeCommand command, int timeoutMs)
        {
            string[] arguments = CommandRouter.CliArguments(command);
            if (arguments == null) throw new ArgumentException("not a read-only runtime query");
            return BoundedProcess.Run(PathToCompatCli(), arguments, timeoutMs, LocalAppData());
        }

        internal static RuntimeStatus ReadStatus(int timeoutMs)
        {
            if (!IsInstalled()) return new RuntimeStatus { Availability = RuntimeAvailability.NotInstalled };
            CliResult result = Query(SafeCommand.Status, timeoutMs);
            return Interpret.Status(result);
        }
    }

    /// Turns the runtime's JSON into truthful, bounded user-facing text.
    internal static class Interpret
    {
        private static Dictionary<string, object> Parse(string json)
        {
            try
            {
                JavaScriptSerializer serializer = new JavaScriptSerializer { MaxJsonLength = BoundedProcess.MaxOutputChars };
                return serializer.DeserializeObject(json) as Dictionary<string, object>;
            }
            catch (Exception) { return null; }
        }

        private static string Text(Dictionary<string, object> map, string key)
        {
            object value;
            return map != null && map.TryGetValue(key, out value) && value != null ? Convert.ToString(value) : null;
        }

        internal static RuntimeStatus Status(CliResult result)
        {
            if (result.FailedToStart || result.TimedOut) return new RuntimeStatus { Availability = RuntimeAvailability.Unresponsive };
            Dictionary<string, object> json = Parse(result.Stdout);
            object runtime = null;
            if (json == null || result.ExitCode != 0 || !json.TryGetValue("runtime", out runtime) || !(runtime is Dictionary<string, object>))
                return new RuntimeStatus { Availability = RuntimeAvailability.Incompatible };
            Dictionary<string, object> runtimeMap = (Dictionary<string, object>)runtime;
            string version = Text(json, "installed");
            if (version == null) return new RuntimeStatus { Availability = RuntimeAvailability.NotInstalled };
            int workspaces;
            return new RuntimeStatus {
                Availability = RuntimeAvailability.Installed,
                Version = version,
                State = Text(runtimeMap, "state"),
                Detail = Text(runtimeMap, "detail"),
                Workspaces = int.TryParse(Text(json, "workspaces"), out workspaces) ? (int?)workspaces : null
            };
        }

        internal static string StatusMessage(RuntimeStatus status)
        {
            if (status.Availability == RuntimeAvailability.NotInstalled)
                return "The Aladdin runtime is not installed for this Windows user. No operation was attempted.";
            if (status.Availability == RuntimeAvailability.Unresponsive)
                return "The Aladdin runtime did not answer a read-only status query in time. No action was performed.";
            if (status.Availability == RuntimeAvailability.Incompatible)
                return "The installed Aladdin runtime returned a response this preview does not understand. No action was performed.";
            StringBuilder text = new StringBuilder();
            text.Append("Aladdin runtime ").Append(status.Version).Append(" is installed and ").Append(RuntimeStatus.Describe(status.State));
            if (!string.IsNullOrEmpty(status.Detail)) text.Append(" (").Append(status.Detail).Append(")");
            text.Append(".");
            if (status.Workspaces.HasValue) text.Append(" Workspaces configured: ").Append(status.Workspaces.Value).Append(".");
            return text.ToString();
        }

        internal static string VersionMessage(CliResult result)
        {
            if (result.FailedToStart || result.TimedOut) return "The Aladdin runtime did not answer a read-only version query in time.";
            Dictionary<string, object> json = Parse(result.Stdout);
            string cli = Text(json, "cli");
            if (json == null || cli == null) return "The installed Aladdin runtime returned a version response this preview does not understand.";
            string installed = Text(json, "installed");
            return "Aladdin CLI " + cli + "; installed release " + (installed ?? "none") + ".";
        }

        /// Doctor exits non-zero when any check fails; its JSON is still the report.
        internal static string DoctorMessage(CliResult result)
        {
            if (result.FailedToStart || result.TimedOut) return "The Aladdin runtime did not finish its read-only health check in time.";
            Dictionary<string, object> json = Parse(result.Stdout);
            object doctor;
            object checks;
            if (json == null || !json.TryGetValue("doctor", out doctor) || !(doctor is Dictionary<string, object>)
                || !((Dictionary<string, object>)doctor).TryGetValue("checks", out checks) || !(checks is IEnumerable))
                return "The installed Aladdin runtime returned a health report this preview does not understand.";
            int pass = 0, warn = 0, fail = 0, other = 0;
            List<string> problems = new List<string>();
            foreach (object item in (IEnumerable)checks)
            {
                Dictionary<string, object> check = item as Dictionary<string, object>;
                string state = Text(check, "status");
                if (state == "pass") pass++;
                else if (state == "warn") { warn++; problems.Add("WARN " + Text(check, "name") + ": " + Text(check, "detail")); }
                else if (state == "fail") { fail++; problems.Add("FAIL " + Text(check, "name") + ": " + Text(check, "detail")); }
                else other++;
            }
            StringBuilder text = new StringBuilder();
            text.Append("Health check: ").Append(pass).Append(" passed, ").Append(warn).Append(" warnings, ").Append(fail).Append(" failed");
            if (other > 0) text.Append(", ").Append(other).Append(" unknown");
            text.Append(".");
            foreach (string problem in problems.Take(8)) text.Append("\n").Append(problem);
            if (problems.Count > 8) text.Append("\n…").Append(problems.Count - 8).Append(" more");
            return text.ToString();
        }
    }

    internal static class FolderPreview
    {
        internal const int MaxEntries = 12;

        /// Lists at most MaxEntries names without materializing the directory.
        internal static string List(string folder, CancellationToken cancel)
        {
            List<string> names = new List<string>();
            bool more = false;
            foreach (string entry in Directory.EnumerateFileSystemEntries(folder))
            {
                cancel.ThrowIfCancellationRequested();
                if (names.Count == MaxEntries) { more = true; break; }
                names.Add(Path.GetFileName(entry));
            }
            string summary = names.Count == 0 ? "(empty)" : string.Join("\n", names.ToArray());
            return "Local folder preview (up to " + MaxEntries + " entries" + (more ? "; more not shown" : "") + "):\n" + summary + "\nNo files were modified or uploaded.";
        }
    }

    internal static class Colors
    {
        internal static Brush Ink = Hex("#E9EDFA");
        internal static Brush Muted = Hex("#9BA7C5");
        internal static Brush Background = Hex("#090D19");
        internal static Brush Sidebar = Hex("#111729");
        internal static Brush Surface = Hex("#161E34");
        internal static Brush Border = Hex("#29334C");
        internal static Brush Accent = Hex("#9A8CF5");
        internal static Brush Safe = Hex("#73D6BD");
        internal static Brush Bubble = Hex("#222F4D");
        internal static Brush Hex(string value)
        {
            return new SolidColorBrush((Color)ColorConverter.ConvertFromString(value));
        }
    }

    internal sealed class MainWindow : Window
    {
        private const int QueryTimeoutMs = 8000;
        // Doctor starts the daemon for an IPC probe and takes about 8 s on a typical machine.
        private const int DoctorTimeoutMs = 30000;
        private readonly StackPanel transcript = new StackPanel();
        private readonly TextBox commandBox = new TextBox();
        private readonly TextBlock hostState = new TextBlock();
        private readonly TextBlock deviceState = new TextBlock();
        private readonly ScrollViewer conversation = new ScrollViewer();
        private bool refreshing;
        private int messages;

        internal MainWindow()
        {
            Title = "Aladdin — Desktop Foundation";
            Width = 1060;
            Height = 700;
            MinWidth = 720;
            MinHeight = 520;
            Background = Colors.Background;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            FontFamily = new System.Windows.Media.FontFamily("Segoe UI");
            AutomationProperties.SetAutomationId(this, "AladdinMainWindow");
            Content = Layout();
            Loaded += delegate { RefreshHost(); commandBox.Focus(); };
        }

        private static TextBlock Text(string value, double size, Brush brush, bool bold)
        {
            return new TextBlock {
                Text = value, FontSize = size, Foreground = brush,
                FontWeight = bold ? FontWeights.SemiBold : FontWeights.Normal,
                TextWrapping = TextWrapping.Wrap
            };
        }

        private static Border Card(UIElement child, Brush background, Thickness padding)
        {
            return new Border {
                Background = background, BorderBrush = Colors.Border, BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(14), Padding = padding, Child = child
            };
        }

        private static System.Windows.Controls.Button ActionButton(string title, bool enabled, string automationId)
        {
            System.Windows.Controls.Button button = new System.Windows.Controls.Button {
                Content = title, IsEnabled = enabled,
                Background = enabled ? Colors.Accent : Colors.Surface,
                Foreground = enabled ? Colors.Background : Colors.Muted,
                BorderThickness = new Thickness(0),
                Padding = new Thickness(15, 9, 15, 9),
                FontSize = 12, FontWeight = FontWeights.SemiBold,
                Cursor = enabled ? System.Windows.Input.Cursors.Hand : System.Windows.Input.Cursors.Arrow,
                Margin = new Thickness(0, 0, 0, 10)
            };
            AutomationProperties.SetAutomationId(button, automationId);
            AutomationProperties.SetName(button, title);
            return button;
        }

        private UIElement Layout()
        {
            Grid root = new Grid();
            root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(280) });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(60) });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

            Border header = new Border {
                Background = Colors.Background, BorderBrush = Colors.Border,
                BorderThickness = new Thickness(0, 0, 0, 1)
            };
            Grid.SetRow(header, 0);
            Grid.SetColumnSpan(header, 2);
            Grid headerContent = new Grid { Margin = new Thickness(22, 0, 22, 0) };
            headerContent.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            headerContent.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            StackPanel logo = new StackPanel { Orientation = System.Windows.Controls.Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
            logo.Children.Add(Text("✦", 22, Colors.Accent, true));
            TextBlock logoText = Text("  Aladdin", 20, Colors.Ink, true);
            logoText.VerticalAlignment = VerticalAlignment.Center;
            logo.Children.Add(logoText);
            headerContent.Children.Add(logo);
            TextBlock stage = Text("DESKTOP FOUNDATION  •  READ-ONLY", 11, Colors.Safe, true);
            stage.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(stage, 1);
            headerContent.Children.Add(stage);
            header.Child = headerContent;
            root.Children.Add(header);

            Grid center = new Grid { Margin = new Thickness(18, 18, 10, 16) };
            center.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            center.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            center.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            Grid.SetRow(center, 1); Grid.SetColumn(center, 0);
            StackPanel headline = new StackPanel { Margin = new Thickness(6, 0, 6, 14) };
            headline.Children.Add(Text("What can I help you with?", 23, Colors.Ink, true));
            TextBlock sub = Text("Read-only foundation: it reports the real local runtime and never executes free text. Aladdin AI and remote actions are not connected.", 12, Colors.Muted, false);
            sub.Margin = new Thickness(0, 6, 0, 0);
            headline.Children.Add(sub);
            center.Children.Add(headline);

            conversation.VerticalScrollBarVisibility = ScrollBarVisibility.Auto;
            conversation.Content = transcript;
            AutomationProperties.SetAutomationId(conversation, "Transcript");
            AutomationProperties.SetName(conversation, "Conversation transcript");
            Border transcriptCard = Card(conversation, Colors.Sidebar, new Thickness(14));
            Grid.SetRow(transcriptCard, 1);
            center.Children.Add(transcriptCard);
            AddMessage(false, "Welcome to Aladdin. Type help, status, version, doctor, devices, or files. Other requests are not executed or sent anywhere.");

            Grid composer = new Grid { Margin = new Thickness(0, 12, 0, 0) };
            composer.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            composer.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            commandBox.Background = Colors.Surface;
            commandBox.Foreground = Colors.Ink;
            commandBox.CaretBrush = Colors.Ink;
            commandBox.BorderBrush = Colors.Border;
            commandBox.BorderThickness = new Thickness(1);
            commandBox.FontSize = 14;
            commandBox.Padding = new Thickness(14, 12, 14, 12);
            commandBox.MaxLength = 1024;
            commandBox.ToolTip = "Read-only commands: help, status, version, doctor, devices, files";
            AutomationProperties.SetAutomationId(commandBox, "CommandBox");
            AutomationProperties.SetName(commandBox, "Command");
            AutomationProperties.SetHelpText(commandBox, "Type a read-only command and press Enter");
            commandBox.KeyDown += delegate(object sender, System.Windows.Input.KeyEventArgs e) {
                if (e.Key == Key.Enter) { e.Handled = true; Send(); }
            };
            composer.Children.Add(commandBox);
            System.Windows.Controls.Button send = ActionButton("Send", true, "SendButton");
            send.Margin = new Thickness(10, 0, 0, 0);
            send.MinWidth = 90;
            send.Click += delegate { Send(); };
            Grid.SetColumn(send, 1);
            composer.Children.Add(send);
            Grid.SetRow(composer, 2);
            center.Children.Add(composer);
            root.Children.Add(center);

            StackPanel right = new StackPanel { Margin = new Thickness(8, 18, 18, 16) };
            Grid.SetRow(right, 1); Grid.SetColumn(right, 1);
            TextBlock deviceTitle = Text("This computer", 18, Colors.Ink, true);
            deviceTitle.Margin = new Thickness(0, 0, 0, 12);
            right.Children.Add(deviceTitle);
            StackPanel host = new StackPanel();
            host.Children.Add(Text(Environment.MachineName, 15, Colors.Ink, true));
            deviceState.Text = "Checking runtime";
            deviceState.FontSize = 12; deviceState.Foreground = Colors.Muted;
            deviceState.Margin = new Thickness(0, 6, 0, 0);
            AutomationProperties.SetAutomationId(deviceState, "DeviceState");
            host.Children.Add(deviceState);
            Border deviceCard = Card(host, Colors.Sidebar, new Thickness(16));
            deviceCard.Margin = new Thickness(0, 0, 0, 14);
            right.Children.Add(deviceCard);
            StackPanel runtime = new StackPanel();
            runtime.Children.Add(Text("LOCAL RUNTIME", 10, Colors.Muted, true));
            hostState.Text = "Checking...";
            hostState.TextWrapping = TextWrapping.Wrap;
            hostState.Margin = new Thickness(0, 8, 0, 14);
            hostState.Foreground = Colors.Ink;
            AutomationProperties.SetAutomationId(hostState, "RuntimeState");
            runtime.Children.Add(hostState);
            System.Windows.Controls.Button refresh = ActionButton("Refresh status", true, "RefreshButton");
            refresh.Margin = new Thickness(0);
            refresh.Click += delegate { RefreshHost(); };
            runtime.Children.Add(refresh);
            Border runtimeCard = Card(runtime, Colors.Sidebar, new Thickness(16));
            runtimeCard.Margin = new Thickness(0, 0, 0, 14);
            right.Children.Add(runtimeCard);
            right.Children.Add(Text("ALADDIN AI", 10, Colors.Muted, true));
            TextBlock ai = Text("Hala One + Reliance: not connected in this preview.", 12, Colors.Muted, false);
            ai.Margin = new Thickness(0, 8, 0, 14);
            right.Children.Add(ai);
            right.Children.Add(Text("OTHER COMPUTERS", 10, Colors.Muted, true));
            TextBlock pair = Text("No paired devices are shown because pairing is not integrated in this preview.", 12, Colors.Muted, false);
            pair.Margin = new Thickness(0, 8, 0, 10);
            right.Children.Add(pair);
            root.Children.Add(right);
            return root;
        }

        private void AddMessage(bool fromUser, string message)
        {
            messages++;
            StackPanel item = new StackPanel { Margin = new Thickness(0, 0, 0, 12) };
            item.HorizontalAlignment = fromUser ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            TextBlock who = Text(fromUser ? "YOU" : "ALADDIN · LOCAL", 10, Colors.Muted, true);
            who.Margin = new Thickness(4, 0, 4, 5);
            item.Children.Add(who);
            TextBlock body = Text(message, 13, Colors.Ink, false);
            AutomationProperties.SetAutomationId(body, "Message" + messages);
            Border bubble = Card(body, fromUser ? Colors.Bubble : Colors.Surface, new Thickness(13, 10, 13, 10));
            bubble.MaxWidth = 560;
            item.Children.Add(bubble);
            transcript.Children.Add(item);
            conversation.ScrollToEnd();
        }

        private void Send()
        {
            string value = commandBox.Text.Trim();
            if (value.Length == 0) return;
            commandBox.Clear();
            AddMessage(true, value);
            SafeCommand kind = CommandRouter.Classify(value);
            if (kind == SafeCommand.Help)
                AddMessage(false, "Available: status, version, doctor, devices, files, help. These operations are read-only. AI chat, voice, pairing, and execution are not connected.");
            else if (kind == SafeCommand.Devices)
                AddMessage(false, "Local computer: " + Environment.MachineName + ". Aladdin runtime: " + (LocalHost.IsInstalled() ? "installed" : "not installed") + ". Remote devices: none (pairing is not integrated in this preview).");
            else if (kind == SafeCommand.Files)
                InspectFolder();
            else if (CommandRouter.CanInvokeInstalledHost(kind))
                QueryHost(kind);
            else
                AddMessage(false, "Not executed. Aladdin AI is not connected. Unknown messages are never treated as shell, web, or computer actions.");
        }

        private async void QueryHost(SafeCommand command)
        {
            if (!LocalHost.IsInstalled())
            {
                AddMessage(false, Interpret.StatusMessage(new RuntimeStatus { Availability = RuntimeAvailability.NotInstalled }));
                return;
            }
            AddMessage(false, "Reading local runtime " + CommandRouter.CliArguments(command)[0] + "...");
            string response = await Task.Run(delegate {
                if (command == SafeCommand.Status) return Interpret.StatusMessage(LocalHost.ReadStatus(QueryTimeoutMs));
                CliResult result = LocalHost.Query(command, command == SafeCommand.Doctor ? DoctorTimeoutMs : QueryTimeoutMs);
                return command == SafeCommand.Version ? Interpret.VersionMessage(result) : Interpret.DoctorMessage(result);
            });
            AddMessage(false, response);
        }

        private async void RefreshHost()
        {
            if (refreshing) return;
            refreshing = true;
            hostState.Text = "Checking...";
            deviceState.Text = "Reading local runtime";
            RuntimeStatus status = await Task.Run(delegate { return LocalHost.ReadStatus(QueryTimeoutMs); });
            hostState.Text = status.Headline();
            deviceState.Text = status.Availability == RuntimeAvailability.NotInstalled
                ? "Windows online • Aladdin runtime missing"
                : "Windows online • " + status.Headline();
            refreshing = false;
        }

        private async void InspectFolder()
        {
            string folder;
            using (System.Windows.Forms.FolderBrowserDialog dialog = new System.Windows.Forms.FolderBrowserDialog())
            {
                dialog.Description = "Choose a folder to list locally (read only)";
                if (dialog.ShowDialog() != System.Windows.Forms.DialogResult.OK) return;
                folder = dialog.SelectedPath;
            }
            AddMessage(false, "Listing the selected folder...");
            using (CancellationTokenSource cancel = new CancellationTokenSource(TimeSpan.FromSeconds(10)))
            {
                string result;
                try { result = await Task.Run(() => FolderPreview.List(folder, cancel.Token), cancel.Token); }
                catch (OperationCanceledException) { result = "Listing this folder took too long and was stopped. No changes were made."; }
                catch (Exception) { result = "Unable to list this folder. No changes were made."; }
                AddMessage(false, result);
            }
        }
    }

    internal static class Program
    {
        private static int SelfTest()
        {
            List<string> failures = new List<string>();
            Action<bool, string> expect = (ok, name) => { if (!ok) failures.Add(name); };
            expect(CommandRouter.Classify("status") == SafeCommand.Status, "status routes");
            expect(CommandRouter.Classify(" Doctor ") == SafeCommand.Doctor, "doctor routes");
            expect(CommandRouter.Classify("rm -rf /") == SafeCommand.Unsupported, "shell text rejected");
            expect(CommandRouter.Classify("status; calc.exe") == SafeCommand.Unsupported, "chained text rejected");
            expect(CommandRouter.CliArguments(SafeCommand.Files) == null, "files never reaches the runtime");
            expect(!CommandRouter.CanInvokeInstalledHost(SafeCommand.Unsupported), "unsupported never runs");
            expect(Path.IsPathRooted(LocalHost.PathToCompatCli()), "runtime path is rooted");

            RuntimeStatus installed = Interpret.Status(new CliResult { ExitCode = 0, Stdout =
                "{\"installed\":\"0.2.0\",\"ok\":true,\"runtime\":{\"detail\":\"no supervisor record\",\"state\":\"not_running\"},\"tunnel_configured\":false,\"workspaces\":0}" });
            expect(installed.Availability == RuntimeAvailability.Installed && installed.Version == "0.2.0", "status parses");
            expect(Interpret.StatusMessage(installed).Contains("not running (no supervisor record)"), "status message is truthful");
            expect(Interpret.Status(new CliResult { TimedOut = true }).Availability == RuntimeAvailability.Unresponsive, "timeout is unresponsive");
            expect(Interpret.Status(new CliResult { ExitCode = 0, Stdout = "not json" }).Availability == RuntimeAvailability.Incompatible, "garbage is incompatible");
            expect(Interpret.Status(new CliResult { ExitCode = 2, Stdout = "{\"ok\":false}" }).Availability == RuntimeAvailability.Incompatible, "error is never reported as healthy");
            string doctor = Interpret.DoctorMessage(new CliResult { ExitCode = 7, Stdout =
                "{\"doctor\":{\"checks\":[{\"name\":\"platform\",\"status\":\"pass\",\"detail\":\"ok\"},{\"name\":\"workspaces\",\"status\":\"fail\",\"detail\":\"none\"},{\"name\":\"tunnel\",\"status\":\"warn\",\"detail\":\"off\"}]}}" });
            expect(doctor.StartsWith("Health check: 1 passed, 1 warnings, 1 failed.") && doctor.Contains("FAIL workspaces: none"), "doctor summarizes failures");
            expect(Interpret.VersionMessage(new CliResult { ExitCode = 0, Stdout = "{\"cli\":\"0.2.0\",\"installed\":\"0.2.0\"}" }) == "Aladdin CLI 0.2.0; installed release 0.2.0.", "version parses");

            // A stalled child is stopped at the timeout and its tree is killed.
            Stopwatch clock = Stopwatch.StartNew();
            CliResult stalled = BoundedProcess.Run(Path.Combine(Environment.SystemDirectory, "ping.exe"), new[] { "-n", "30", "127.0.0.1" }, 1000, null);
            expect(stalled.TimedOut && clock.ElapsedMilliseconds < 8000, "stalled child times out");
            // A child that floods stdout and stderr never deadlocks and is bounded.
            CliResult flood = BoundedProcess.Run(Path.Combine(Environment.SystemDirectory, "cmd.exe"),
                new[] { "/c", "for /L %i in (1,1,40000) do @echo flooding-stdout-and-stderr-line-%i & echo err 1>&2" }, 30000, null);
            expect(!flood.TimedOut && flood.ExitCode == 0 && flood.Truncated && flood.Stdout.Length == BoundedProcess.MaxOutputChars, "flooding child is drained and bounded");

            string dir = Path.Combine(Path.GetTempPath(), "aladdin-folder-test-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(dir);
            try
            {
                for (int i = 0; i < 40; i++) File.WriteAllText(Path.Combine(dir, "f" + i + ".txt"), "");
                string listing = FolderPreview.List(dir, CancellationToken.None);
                expect(listing.Contains("more not shown") && listing.Split('\n').Length == FolderPreview.MaxEntries + 2, "folder preview is bounded");
                bool cancelled = false;
                using (CancellationTokenSource cancel = new CancellationTokenSource())
                {
                    cancel.Cancel();
                    try { FolderPreview.List(dir, cancel.Token); } catch (OperationCanceledException) { cancelled = true; }
                }
                expect(cancelled, "folder preview honors cancellation");
            }
            finally { try { Directory.Delete(dir, true); } catch (Exception) { } }

            if (failures.Count > 0)
            {
                foreach (string failure in failures) Console.WriteLine("FAIL: " + failure);
                return 1;
            }
            Console.WriteLine("Aladdin read-only router, runtime interpretation, bounded process and folder self-tests: PASS");
            return 0;
        }

        [STAThread]
        private static int Main(string[] args)
        {
            if (args.Length == 1 && args[0] == "--self-test") return SelfTest();
            if (args.Length == 1 && args[0] == "--ui-self-test") { MainWindow window = new MainWindow(); if (window.Content == null) return 2; Console.WriteLine("Aladdin WPF layout construction: PASS"); return 0; }
            new System.Windows.Application().Run(new MainWindow());
            return 0;
        }
    }
}
