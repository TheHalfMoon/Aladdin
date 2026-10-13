param([Parameter(Mandatory)][string]$Stop, [Parameter(Mandatory)][string]$Out, [Parameter(Mandatory)][string]$Pids)
# SG-000108 A8: read-only sampler of the OS's TCP and UDP tables for every
# process of the suite's Edge: the whole process tree under each browser
# edge.ts recorded in $Pids, plus any process carrying a qdral-a8-edge-*
# profile on its command line. One JSON line per sample, until the stop
# file appears. It reads; it changes nothing.
$ErrorActionPreference = "Stop"
while (-not (Test-Path -LiteralPath $Stop)) {
  $owned = @{}
  $all = @(Get-CimInstance Win32_Process)
  $roots = @()
  if (Test-Path -LiteralPath $Pids) {
    $roots = @(Get-Content -LiteralPath $Pids | ForEach-Object { [int](($_ -split "`t")[0]) })
  }
  foreach ($p in $all) {
    if ($roots -contains [int]$p.ProcessId -or $p.CommandLine -like "*qdral-a8-edge-*") { $owned[[int]$p.ProcessId] = $p.Name }
  }
  do {
    $grew = $false
    foreach ($p in $all) {
      if ($owned.ContainsKey([int]$p.ParentProcessId) -and -not $owned.ContainsKey([int]$p.ProcessId)) {
        $owned[[int]$p.ProcessId] = $p.Name
        $grew = $true
      }
    }
  } while ($grew)
  $rows = [System.Collections.Generic.List[object]]::new()
  if ($owned.Count -gt 0) {
    Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $owned.ContainsKey([int]$_.OwningProcess) } |
      ForEach-Object {
        $rows.Add([pscustomobject]@{ kind = "tcp"; state = "$($_.State)"; local = "$($_.LocalAddress)"; lport = $_.LocalPort;
          remote = "$($_.RemoteAddress)"; rport = $_.RemotePort; pid = [int]$_.OwningProcess; name = $owned[[int]$_.OwningProcess] })
      }
    Get-NetUDPEndpoint -ErrorAction SilentlyContinue | Where-Object { $owned.ContainsKey([int]$_.OwningProcess) } |
      ForEach-Object {
        $rows.Add([pscustomobject]@{ kind = "udp"; state = ""; local = "$($_.LocalAddress)"; lport = $_.LocalPort;
          remote = ""; rport = 0; pid = [int]$_.OwningProcess; name = $owned[[int]$_.OwningProcess] })
      }
  }
  $sample = [pscustomobject]@{ processes = $owned.Count; rows = @($rows) }
  Add-Content -LiteralPath $Out -Value ($sample | ConvertTo-Json -Compress -Depth 4)
  Start-Sleep -Milliseconds 500
}
