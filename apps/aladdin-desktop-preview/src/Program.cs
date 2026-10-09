// Aladdin Desktop Foundation — read-only Windows preview.
// This preview never sends screenshots or text to a model, network, or remote device.
using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;


namespace Aladdin.DesktopPreview
{
    internal enum SafeCommand { Help, Status, Version, Devices, Files, Unsupported }

    internal static class CommandRouter
    {
        internal static SafeCommand Classify(string input)
        {
            string value = (input ?? "").Trim().ToLowerInvariant();
            if (value == "help" || value == "?") return SafeCommand.Help;
            if (value == "status") return SafeCommand.Status;
            if (value == "version") return SafeCommand.Version;
            if (value == "devices") return SafeCommand.Devices;
            if (value == "files") return SafeCommand.Files;
            return SafeCommand.Unsupported;
        }

        internal static bool CanInvokeInstalledHost(SafeCommand command)
        {
            return command == SafeCommand.Status || command == SafeCommand.Version;
        }
    }

    internal static class LocalHost
    {
        internal static string PathToCompatCli()
        {
            // Do not search PATH or execute model-provided executable paths.
            return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                                "Qdral", "bin", "qdral.exe");
        }

        internal static bool IsInstalled()
        {
            return File.Exists(PathToCompatCli());
        }

        internal static string RunReadOnly(SafeCommand command)
        {
            if (!CommandRouter.CanInvokeInstalledHost(command))
                return "Denied: only fixed read-only status and version operations are available.";
            if (!IsInstalled())
                return "The Aladdin compatibility runtime (qdral.exe) is not installed. No operation was attempted.";

            string operation = command == SafeCommand.Status ? "status" : "version";
            try
            {
                using (Process process = new Process())
                {
                    process.StartInfo = new ProcessStartInfo {
                        FileName = PathToCompatCli(),
                        Arguments = operation,
                        UseShellExecute = false,
                        CreateNoWindow = true,
                        RedirectStandardOutput = true,
                        RedirectStandardError = true,
                        WorkingDirectory = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile)
                    };
                    process.Start();
                    if (!process.WaitForExit(5000))
                    {
                        try { process.Kill(); } catch (Exception) { }
                        return "Read-only status query timed out; no action was performed.";
                    }
                    string output = process.StandardOutput.ReadToEnd();
                    if (process.ExitCode != 0)
                        return "The runtime returned an error. Check its CLI locally. Exit code: " + process.ExitCode;
                    if (string.IsNullOrWhiteSpace(output))
                        return "The runtime produced no status output.";
                    return output.Length > 1400 ? output.Substring(0, 1400) + "\n[Truncated]" : output.Trim();
                }
            }
            catch (Exception)
            {
                // Fail closed and do not surface exception details containing environment paths.
                return "Unable to read local runtime status. No action was performed.";
            }
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
        private readonly StackPanel transcript = new StackPanel();
        private readonly TextBox commandBox = new TextBox();
        private readonly TextBlock hostState = new TextBlock();
        private readonly TextBlock deviceState = new TextBlock();
        private readonly ScrollViewer conversation = new ScrollViewer();
        private bool refreshing;

        internal MainWindow()
        {
            Title = "Aladdin — Desktop Foundation";
            Width = 1160;
            Height = 740;
            MinWidth = 910;
            MinHeight = 580;
            Background = Colors.Background;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            FontFamily = new System.Windows.Media.FontFamily("Segoe UI");
            Content = Layout();
            Loaded += delegate { RefreshHost(); };
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

        private static System.Windows.Controls.Button ActionButton(string title, bool enabled)
        {
            return new System.Windows.Controls.Button {
                Content = title, IsEnabled = enabled,
                Background = enabled ? Colors.Accent : Colors.Surface,
                Foreground = enabled ? Colors.Background : Colors.Muted,
                BorderThickness = new Thickness(0),
                Padding = new Thickness(15, 9, 15, 9),
                FontSize = 12, FontWeight = FontWeights.SemiBold,
                Cursor = enabled ? System.Windows.Input.Cursors.Hand : System.Windows.Input.Cursors.Arrow,
                Margin = new Thickness(0, 0, 0, 10)
            };
        }

        private UIElement Layout()
        {
            Grid root = new Grid();
            root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(225) });
            root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            root.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(268) });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(66) });
            root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });

            Border header = new Border {
                Background = Colors.Background, BorderBrush = Colors.Border,
                BorderThickness = new Thickness(0, 0, 0, 1)
            };
            Grid.SetRow(header, 0);
            Grid.SetColumnSpan(header, 3);
            Grid headerContent = new Grid { Margin = new Thickness(24, 0, 24, 0) };
            headerContent.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
            headerContent.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
            StackPanel logo = new StackPanel { Orientation = System.Windows.Controls.Orientation.Horizontal, VerticalAlignment = VerticalAlignment.Center };
            logo.Children.Add(Text("✦", 24, Colors.Accent, true));
            TextBlock logoText = Text("  Aladdin", 21, Colors.Ink, true);
            logoText.VerticalAlignment = VerticalAlignment.Center;
            logo.Children.Add(logoText);
            headerContent.Children.Add(logo);
            TextBlock stage = Text("DESKTOP FOUNDATION  •  READ-ONLY", 11, Colors.Safe, true);
            stage.VerticalAlignment = VerticalAlignment.Center;
            Grid.SetColumn(stage, 1);
            headerContent.Children.Add(stage);
            header.Child = headerContent;
            root.Children.Add(header);

            StackPanel left = new StackPanel { Margin = new Thickness(20, 22, 14, 20) };
            Grid.SetColumn(left, 0); Grid.SetRow(left, 1);
            TextBlock menu = Text("YOUR WORKSPACE", 11, Colors.Muted, true);
            menu.Margin = new Thickness(1, 0, 0, 17);
            left.Children.Add(menu);
            TextBlock local = Text("◉  This computer", 14, Colors.Safe, true);
            local.Margin = new Thickness(0, 0, 0, 18);
            left.Children.Add(local);
            left.Children.Add(Text("ALADDIN CORE", 11, Colors.Muted, true));
            TextBlock tools = Text("Files · Runtime status · MCP-compatible host", 12, Colors.Ink, false);
            tools.Margin = new Thickness(0, 9, 0, 24);
            left.Children.Add(tools);
            left.Children.Add(Text("ALADDIN AI", 11, Colors.Muted, true));
            TextBlock ai = Text("Hala One + Reliance\nServer connection not configured", 12, Colors.Muted, false);
            ai.Margin = new Thickness(0, 9, 0, 24);
            left.Children.Add(ai);
            System.Windows.Controls.Button releases = ActionButton("View GitHub releases  ↗", true);
            releases.Click += delegate {
                Process.Start(new ProcessStartInfo("https://github.com/TheHalfMoon/Aladdin/releases") { UseShellExecute = true });
            };
            left.Children.Add(releases);
            TextBlock note = Text("No model inference or remote computer control is enabled in this preview.", 11, Colors.Muted, false);
            note.Margin = new Thickness(0, 12, 0, 0);
            left.Children.Add(note);
            root.Children.Add(left);

            Grid center = new Grid { Margin = new Thickness(4, 22, 12, 18) };
            center.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            center.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
            center.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            Grid.SetRow(center, 1); Grid.SetColumn(center, 1);
            StackPanel headline = new StackPanel { Margin = new Thickness(12, 0, 12, 18) };
            headline.Children.Add(Text("What can I help you with?", 25, Colors.Ink, true));
            TextBlock sub = Text("A real Windows control-center foundation. Cloud AI and remote actions are safely unavailable until integrated.", 12, Colors.Muted, false);
            sub.Margin = new Thickness(0, 6, 0, 0);
            headline.Children.Add(sub);
            center.Children.Add(headline);

            conversation.VerticalScrollBarVisibility = ScrollBarVisibility.Auto;
            conversation.Content = transcript;
            Border transcriptCard = Card(conversation, Colors.Sidebar, new Thickness(16));
            Grid.SetRow(transcriptCard, 1);
            center.Children.Add(transcriptCard);
            AddMessage(false, "Welcome to Aladdin. Type help, status, devices, version, or files. Other requests are not executed or sent anywhere.");

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
            commandBox.ToolTip = "Read-only commands: help, status, devices, version, files";
            commandBox.KeyDown += delegate(object sender, System.Windows.Input.KeyEventArgs e) {
                if (e.Key == Key.Enter) { e.Handled = true; Send(); }
            };
            composer.Children.Add(commandBox);
            System.Windows.Controls.Button send = ActionButton("Send", true);
            send.Margin = new Thickness(10, 0, 0, 0);
            send.MinWidth = 90;
            send.Click += delegate { Send(); };
            Grid.SetColumn(send, 1);
            composer.Children.Add(send);
            Grid.SetRow(composer, 2);
            center.Children.Add(composer);
            root.Children.Add(center);

            StackPanel right = new StackPanel { Margin = new Thickness(10, 22, 20, 20) };
            Grid.SetRow(right, 1); Grid.SetColumn(right, 2);
            TextBlock deviceTitle = Text("Devices", 19, Colors.Ink, true);
            deviceTitle.Margin = new Thickness(0, 0, 0, 14);
            right.Children.Add(deviceTitle);
            StackPanel host = new StackPanel();
            host.Children.Add(Text("THIS PC", 10, Colors.Safe, true));
            TextBlock machine = Text(Environment.MachineName, 16, Colors.Ink, true);
            machine.Margin = new Thickness(0, 8, 0, 8);
            host.Children.Add(machine);
            deviceState.Text = "Checking runtime";
            deviceState.FontSize = 12; deviceState.Foreground = Colors.Muted;
            host.Children.Add(deviceState);
            Border deviceCard = Card(host, Colors.Sidebar, new Thickness(17));
            deviceCard.Margin = new Thickness(0, 0, 0, 16);
            right.Children.Add(deviceCard);
            StackPanel runtime = new StackPanel();
            runtime.Children.Add(Text("LOCAL RUNTIME", 10, Colors.Muted, true));
            hostState.Text = "Checking...";
            hostState.TextWrapping = TextWrapping.Wrap;
            hostState.Margin = new Thickness(0, 8, 0, 14);
            hostState.Foreground = Colors.Ink;
            runtime.Children.Add(hostState);
            System.Windows.Controls.Button refresh = ActionButton("Refresh status", true);
            refresh.Margin = new Thickness(0);
            refresh.Click += delegate { RefreshHost(); };
            runtime.Children.Add(refresh);
            Border runtimeCard = Card(runtime, Colors.Sidebar, new Thickness(17));
            runtimeCard.Margin = new Thickness(0, 0, 0, 16);
            right.Children.Add(runtimeCard);
            right.Children.Add(Text("MULTI-COMPUTER", 10, Colors.Muted, true));
            TextBlock pair = Text("Remote devices are not paired in this preview. Pairing and approvals must be integrated through the existing Aladdin security boundary.", 12, Colors.Muted, false);
            pair.Margin = new Thickness(0, 9, 0, 12);
            right.Children.Add(pair);
            System.Windows.Controls.Button disabled = ActionButton("Pair a device (not ready)", false);
            right.Children.Add(disabled);
            root.Children.Add(right);
            return root;
        }

        private void AddMessage(bool fromUser, string message)
        {
            StackPanel item = new StackPanel { Margin = new Thickness(0, 0, 0, 12) };
            item.HorizontalAlignment = fromUser ? HorizontalAlignment.Right : HorizontalAlignment.Left;
            TextBlock who = Text(fromUser ? "YOU" : "ALADDIN · LOCAL", 10, Colors.Muted, true);
            who.Margin = new Thickness(4, 0, 4, 5);
            item.Children.Add(who);
            Border bubble = Card(Text(message, 13, Colors.Ink, false), fromUser ? Colors.Bubble : Colors.Surface, new Thickness(13, 10, 13, 10));
            bubble.MaxWidth = 490;
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
                AddMessage(false, "Available: status, version, devices, files, help. These operations are read-only. AI chat, voice, pairing, and execution are not connected.");
            else if (kind == SafeCommand.Devices)
                AddMessage(false, "Local computer: " + Environment.MachineName + ". Aladdin host: " + (LocalHost.IsInstalled() ? "installed" : "not installed") + ". Remote devices: none connected.");
            else if (kind == SafeCommand.Files)
                InspectFolder();
            else if (CommandRouter.CanInvokeInstalledHost(kind))
                QueryHost(kind);
            else
                AddMessage(false, "Not executed. Aladdin AI is not connected. Unknown messages are never treated as shell, web, or computer actions.");
        }

        private async void QueryHost(SafeCommand command)
        {
            AddMessage(false, "Reading local runtime " + (command == SafeCommand.Status ? "status" : "version") + "...");
            string response = await Task.Run(delegate { return LocalHost.RunReadOnly(command); });
            AddMessage(false, response);
        }

        private async void RefreshHost()
        {
            if (refreshing) return;
            refreshing = true;
            hostState.Text = "Checking...";
            deviceState.Text = "Reading local host";
            bool installed = await Task.Run(delegate { return LocalHost.IsInstalled(); });
            hostState.Text = installed ? "Installed • bounded local runtime" : "Not installed";
            deviceState.Text = installed ? "Local runtime found" : "Windows online • Aladdin runtime missing";
            refreshing = false;
        }

        private void InspectFolder()
        {
            using (System.Windows.Forms.FolderBrowserDialog dialog = new System.Windows.Forms.FolderBrowserDialog())
            {
                dialog.Description = "Choose a folder to list locally (read only)";
                if (dialog.ShowDialog() != System.Windows.Forms.DialogResult.OK) return;
                try
                {
                    string[] entries = Directory.GetFileSystemEntries(dialog.SelectedPath).Take(12).ToArray();
                    string summary = entries.Length == 0 ? "(empty)" : string.Join("\n", entries.Select(System.IO.Path.GetFileName).ToArray());
                    AddMessage(false, "Local folder preview (up to 12 entries):\n" + summary + "\nNo files were modified or uploaded.");
                }
                catch (Exception)
                {
                    AddMessage(false, "Unable to list this folder. No changes were made.");
                }
            }
        }
    }

    internal static class Program
    {
        [STAThread]
        private static int Main(string[] args)
        {
            if (args.Length == 1 && args[0] == "--self-test")
            {
                if (CommandRouter.Classify("status") != SafeCommand.Status ||
                    CommandRouter.Classify("version") != SafeCommand.Version ||
                    CommandRouter.Classify("rm -rf /") != SafeCommand.Unsupported ||
                    CommandRouter.Classify("status; calc.exe") != SafeCommand.Unsupported ||
                    CommandRouter.CanInvokeInstalledHost(SafeCommand.Unsupported) ||
                    CommandRouter.CanInvokeInstalledHost(SafeCommand.Files) ||
                    !CommandRouter.CanInvokeInstalledHost(SafeCommand.Status) ||
                    !CommandRouter.CanInvokeInstalledHost(SafeCommand.Version) ||
                    !Path.IsPathRooted(LocalHost.PathToCompatCli()))
                    return 1;
                Console.WriteLine("Aladdin read-only command and host-path self-tests: PASS");
                return 0;
            }
            if (args.Length == 1 && args[0] == "--ui-self-test") { MainWindow window = new MainWindow(); if (window.Content == null) return 2; Console.WriteLine("Aladdin WPF layout construction: PASS"); return 0; }
            new System.Windows.Application().Run(new MainWindow());
            return 0;
        }
    }
}
