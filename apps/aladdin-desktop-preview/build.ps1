# Build with Windows' existing .NET Framework C# compiler, offline.
param([string]$OutputDirectory = (Join-Path $env:TEMP "AladdinDesktopPreviewBuild"))
$ErrorActionPreference = "Stop"
$framework = Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319"
$csc = Join-Path $framework "csc.exe"
$wpf = Join-Path $framework "WPF"
$src = Join-Path $PSScriptRoot "src\Program.cs"
$refs = @(
    (Join-Path $wpf "PresentationFramework.dll"),
    (Join-Path $wpf "PresentationCore.dll"),
    (Join-Path $wpf "WindowsBase.dll"),
    (Join-Path $framework "System.Xaml.dll"),
    (Join-Path $framework "System.Windows.Forms.dll"),
    (Join-Path $framework "System.Core.dll"),
    (Join-Path $framework "System.Web.Extensions.dll")
)
foreach ($component in @($csc, $src) + $refs) {
    if (-not (Test-Path $component)) { throw "Required built-in Windows component missing: $component" }
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
foreach ($target in @("winexe", "exe")) {
    $name = if ($target -eq "winexe") { "Aladdin.Desktop.exe" } else { "Aladdin.Smoke.exe" }
    $out = Join-Path $OutputDirectory $name
    $compilerArgs = @("/nologo", "/target:$target", "/optimize+", "/platform:anycpu", "/out:$out")
    foreach ($reference in $refs) { $compilerArgs += "/reference:$reference" }
    $compilerArgs += $src
    & $csc @compilerArgs
    if ($LASTEXITCODE -ne 0) { throw "Compilation failed for $target" }
}
$app = Join-Path $OutputDirectory "Aladdin.Desktop.exe"
Write-Output ("Compiled: {0} ({1} bytes)" -f $app, (Get-Item $app).Length)
