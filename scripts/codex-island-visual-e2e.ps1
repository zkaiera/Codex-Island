param(
  [string] $OutputDir = (Join-Path $env:TEMP "codex-island-visual-e2e"),
  [switch] $NoBackdrop,
  [switch] $AllowInvisible,
  [int] $SessionCount = 8
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class NativeMethods {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll")]
  public static extern bool SetProcessDPIAware();

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extraData);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll", EntryPoint = "mouse_event")]
  public static extern void MouseEvent(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
}
"@

[NativeMethods]::SetProcessDPIAware() | Out-Null

$Exe = Join-Path $env:LOCALAPPDATA "Codex Island\codex-island.exe"
$StateDir = Join-Path $OutputDir "state\sessions"
$SessionPrefix = "codex-island-visual-e2e"
$VirtualScreen = [System.Windows.Forms.SystemInformation]::VirtualScreen
$MouseEventLeftDown = 0x0002
$MouseEventLeftUp = 0x0004

function Reset-OutputDir {
  if (Test-Path -LiteralPath $OutputDir) {
    Remove-Item -LiteralPath $OutputDir -Recurse -Force
  }

  New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
}

function Show-StaticBackdrop {
  $form = New-Object System.Windows.Forms.Form
  $form.Text = "Codex Island E2E Backdrop"
  $form.StartPosition = "Manual"
  $form.FormBorderStyle = "None"
  $form.Bounds = $VirtualScreen
  $form.BackColor = [System.Drawing.Color]::FromArgb(214, 226, 242)
  $form.ShowInTaskbar = $false
  $form.TopMost = $true
  $form.Show()
  $form.Activate()
  [System.Windows.Forms.Application]::DoEvents()
  Start-Sleep -Milliseconds 300
  $form
}

function Write-TestSessions {
  if ($SessionCount -lt 1) {
    throw "SessionCount must be at least 1."
  }

  New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
  Get-ChildItem -LiteralPath $StateDir -Filter "$SessionPrefix*.json" -ErrorAction SilentlyContinue |
    Remove-Item -Force

  $now = (Get-Date).ToUniversalTime()
  for ($index = 1; $index -le $SessionCount; $index++) {
    $createdAt = $now.AddSeconds(-1 * ($SessionCount - $index)).ToString("o")
    $updatedAt = $now.ToString("o")
    $record = [ordered]@{
      session_id = "$SessionPrefix-$index"
      turn_id = $null
      cwd = "C:\work\codex-island-visual-e2e-$index"
      title = "visual-e2e-session-$index"
      source = "windows"
      distro = $null
      last_event = "SessionStart"
      last_tool = $null
      ui_state = "running"
      created_at = $createdAt
      updated_at = $updatedAt
    }

    $path = Join-Path $StateDir "$SessionPrefix-$index.json"
    $json = $record | ConvertTo-Json -Depth 5
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($path, $json, $utf8NoBom)
  }
}

function Get-WindowTitle([IntPtr] $handle) {
  $builder = New-Object System.Text.StringBuilder 256
  [NativeMethods]::GetWindowText($handle, $builder, $builder.Capacity) | Out-Null
  $builder.ToString()
}

function Convert-Rect([IntPtr] $handle, [string] $title) {
  $nativeRect = New-Object NativeMethods+RECT
  [NativeMethods]::GetWindowRect($handle, [ref] $nativeRect) | Out-Null
  [pscustomobject]@{
    Handle = $handle
    Title = $title
    X = $nativeRect.Left
    Y = $nativeRect.Top
    Width = $nativeRect.Right - $nativeRect.Left
    Height = $nativeRect.Bottom - $nativeRect.Top
    Right = $nativeRect.Right
    Bottom = $nativeRect.Bottom
  }
}

function Get-AppWindows {
  $processes = @(Get-Process codex-island -ErrorAction SilentlyContinue)
  if ($processes.Count -eq 0) {
    return @()
  }

  $pids = @{}
  foreach ($process in $processes) {
    $pids[[uint32]$process.Id] = $true
  }

  $windows = New-Object System.Collections.ArrayList
  $callback = [NativeMethods+EnumWindowsProc] {
    param([IntPtr] $handle, [IntPtr] $extraData)
    $processId = 0
    [NativeMethods]::GetWindowThreadProcessId($handle, [ref] $processId) | Out-Null
    if ($pids.ContainsKey([uint32]$processId) -and [NativeMethods]::IsWindowVisible($handle)) {
      $title = Get-WindowTitle $handle
      if ($title) {
        [void]$windows.Add((Convert-Rect $handle $title))
      }
    }
    return $true
  }

  [NativeMethods]::EnumWindows($callback, [IntPtr]::Zero) | Out-Null
  @($windows.ToArray())
}

function Wait-Window([string] $title, [int] $timeoutMs = 8000) {
  $deadline = (Get-Date).AddMilliseconds($timeoutMs)
  while ((Get-Date) -lt $deadline) {
    $window = Get-AppWindows | Where-Object { $_.Title -eq $title } | Select-Object -First 1
    if ($null -ne $window) {
      return $window
    }
    Start-Sleep -Milliseconds 80
  }

  throw "Window '$title' was not visible within ${timeoutMs}ms. Visible windows: $((Get-AppWindows | Select-Object Title,Width,Height | ConvertTo-Json -Compress))"
}

function Wait-CollapsedMain([int] $timeoutMs = 8000) {
  $deadline = (Get-Date).AddMilliseconds($timeoutMs)
  $last = $null
  while ((Get-Date) -lt $deadline) {
    $main = Wait-Window "Codex Island" 1000
    $last = $main
    if ($main.Width -lt 260 -and $main.Height -lt 260) {
      Start-Sleep -Milliseconds 220
      $stable = Wait-Window "Codex Island" 1000
      if (
        $stable.Width -lt 260 -and
        $stable.Height -lt 260 -and
        [Math]::Abs($stable.X - $main.X) -le 3 -and
        [Math]::Abs($stable.Y - $main.Y) -le 3 -and
        [Math]::Abs($stable.Width - $main.Width) -le 3 -and
        [Math]::Abs($stable.Height - $main.Height) -le 3
      ) {
        return $stable
      }
    }

    Start-Sleep -Milliseconds 100
  }

  throw "Main window did not settle into collapsed island size within ${timeoutMs}ms. Last observed: $($last | ConvertTo-Json -Compress)"
}

function Get-PanelWindow {
  Get-AppWindows | Where-Object { $_.Title -eq "Codex Island Panel" } | Select-Object -First 1
}

function Move-Cursor([int] $x, [int] $y) {
  [NativeMethods]::SetCursorPos($x, $y) | Out-Null
}

function Press-LeftMouse {
  [NativeMethods]::MouseEvent($MouseEventLeftDown, 0, 0, 0, [UIntPtr]::Zero)
}

function Release-LeftMouse {
  [NativeMethods]::MouseEvent($MouseEventLeftUp, 0, 0, 0, [UIntPtr]::Zero)
}

function Move-Away([int] $milliseconds = 800) {
  $work = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  Move-Cursor ([int]($work.X + $work.Width / 2)) ([int]($work.Y + $work.Height - 10))
  Start-Sleep -Milliseconds $milliseconds
}

function Drag-MainWindowTo([string] $scenarioName, [int] $targetX, [int] $targetY) {
  $main = Wait-CollapsedMain 9000
  [NativeMethods]::SetForegroundWindow($main.Handle) | Out-Null

  $startX = [int]($main.X + $main.Width / 2)
  $startY = [int]($main.Y + $main.Height / 2)
  Move-Cursor $startX $startY
  Start-Sleep -Milliseconds 60
  Press-LeftMouse
  Start-Sleep -Milliseconds 80

  $steps = 18
  for ($step = 1; $step -le $steps; $step++) {
    $nextX = [int]($startX + (($targetX - $startX) * $step / $steps))
    $nextY = [int]($startY + (($targetY - $startY) * $step / $steps))
    Move-Cursor $nextX $nextY
    Start-Sleep -Milliseconds 18
  }

  Start-Sleep -Milliseconds 80
  Release-LeftMouse
  Start-Sleep -Milliseconds 1100

  $settled = Wait-CollapsedMain 9000
  [pscustomobject]@{
    Scenario = $scenarioName
    TargetX = $targetX
    TargetY = $targetY
    SettledWindow = $settled
  }
}

function Move-IslandForScenario([string] $scenarioName) {
  $work = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
  switch ($scenarioName) {
    "left" {
      return Drag-MainWindowTo $scenarioName ([int]($work.X + 18)) ([int]($work.Y + $work.Height / 2))
    }
    "right" {
      return Drag-MainWindowTo $scenarioName ([int]($work.Right - 18)) ([int]($work.Y + $work.Height / 2))
    }
    "floating" {
      return Drag-MainWindowTo $scenarioName ([int]($work.X + $work.Width / 2)) ([int]($work.Y + 260))
    }
    default {
      throw "Unsupported scenario: $scenarioName"
    }
  }
}

function Capture-Screen([string] $name) {
  $path = Join-Path $OutputDir $name
  $bitmap = New-Object System.Drawing.Bitmap $VirtualScreen.Width, $VirtualScreen.Height
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.CopyFromScreen($VirtualScreen.Left, $VirtualScreen.Top, 0, 0, $bitmap.Size)
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }

  $path
}

function Get-ImageRegionDiff([string] $beforePath, [string] $afterPath, $rect) {
  if ($null -eq $rect -or $rect.Width -le 0 -or $rect.Height -le 0) {
    return $null
  }

  $before = [System.Drawing.Bitmap]::FromFile($beforePath)
  $after = [System.Drawing.Bitmap]::FromFile($afterPath)
  try {
    $left = [Math]::Max($rect.X - $VirtualScreen.Left, 0)
    $top = [Math]::Max($rect.Y - $VirtualScreen.Top, 0)
    $right = [Math]::Min($rect.Right - $VirtualScreen.Left, $after.Width)
    $bottom = [Math]::Min($rect.Bottom - $VirtualScreen.Top, $after.Height)
    $step = 5
    $sum = 0L
    $count = 0L
    $changed = 0L

    for ($y = $top; $y -lt $bottom; $y += $step) {
      for ($x = $left; $x -lt $right; $x += $step) {
        $a = $before.GetPixel($x, $y)
        $b = $after.GetPixel($x, $y)
        $delta = [Math]::Abs($a.R - $b.R) + [Math]::Abs($a.G - $b.G) + [Math]::Abs($a.B - $b.B)
        $sum += $delta
        $count += 1
        if ($delta -gt 36) {
          $changed += 1
        }
      }
    }

    if ($count -eq 0) {
      return $null
    }

    [pscustomobject]@{
      AverageRgbDelta = [Math]::Round($sum / $count, 2)
      ChangedSampleRatio = [Math]::Round($changed / $count, 4)
      Samples = $count
    }
  } finally {
    $before.Dispose()
    $after.Dispose()
  }
}

function Start-App {
  if (-not (Test-Path -LiteralPath $Exe)) {
    throw "Installed executable not found: $Exe"
  }

  Get-Process codex-island -ErrorAction SilentlyContinue | Stop-Process -Force
  Start-Sleep -Milliseconds 400
  $previousStateDirOverride = $env:CODEX_ISLAND_STATE_DIR
  try {
    $env:CODEX_ISLAND_STATE_DIR = $StateDir
    Start-Process -FilePath $Exe
  } finally {
    if ($null -eq $previousStateDirOverride) {
      Remove-Item Env:\CODEX_ISLAND_STATE_DIR -ErrorAction SilentlyContinue
    } else {
      $env:CODEX_ISLAND_STATE_DIR = $previousStateDirOverride
    }
  }
  $main = Wait-Window "Codex Island" 8000
  [NativeMethods]::SetForegroundWindow($main.Handle) | Out-Null
  $main
}

function Run-HoverCapture([string] $cycleName, $mainBefore) {
  $beforePath = Capture-Screen ("{0}-00-before-hover.png" -f $cycleName)

  $centerX = [int]($mainBefore.X + $mainBefore.Width / 2)
  $centerY = [int]($mainBefore.Y + $mainBefore.Height / 2)
  Move-Cursor $centerX $centerY

  $captures = New-Object System.Collections.Generic.List[object]
  $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
  foreach ($targetMs in @(120, 260, 520, 900, 1400)) {
    $remaining = $targetMs - $stopwatch.ElapsedMilliseconds
    if ($remaining -gt 0) {
      Start-Sleep -Milliseconds $remaining
    }

    $path = Capture-Screen ("{0}-hover-{1:D4}ms.png" -f $cycleName, $targetMs)
    $windows = @(Get-AppWindows)
    $panel = $windows | Where-Object { $_.Title -eq "Codex Island Panel" } | Select-Object -First 1
    $captures.Add([pscustomobject]@{
      AtMs = $targetMs
      Screenshot = $path
      Windows = $windows | Select-Object Title, X, Y, Width, Height, Right, Bottom
      PanelRegionDiff = Get-ImageRegionDiff $beforePath $path $panel
    })
  }

  Move-Away 1000
  [pscustomobject]@{
    Cycle = $cycleName
    MainBefore = $mainBefore
    BeforeScreenshot = $beforePath
    Captures = $captures
    AfterLeaveScreenshot = Capture-Screen ("{0}-99-after-leave.png" -f $cycleName)
    WindowsAfterLeave = @(Get-AppWindows) | Select-Object Title, X, Y, Width, Height, Right, Bottom
  }
}

function Get-BestPanelSignal($cycles) {
  $signals = New-Object System.Collections.Generic.List[object]
  foreach ($cycle in $cycles) {
    foreach ($capture in $cycle.Captures) {
      if ($null -ne $capture.PanelRegionDiff) {
        $signals.Add([pscustomobject]@{
          Cycle = $cycle.Cycle
          AtMs = $capture.AtMs
          Screenshot = $capture.Screenshot
          AverageRgbDelta = $capture.PanelRegionDiff.AverageRgbDelta
          ChangedSampleRatio = $capture.PanelRegionDiff.ChangedSampleRatio
          Samples = $capture.PanelRegionDiff.Samples
        })
      }
    }
  }

  $signals | Sort-Object AverageRgbDelta -Descending | Select-Object -First 1
}

function Get-PanelWindowHeightSignal($cycles) {
  $signals = New-Object System.Collections.Generic.List[object]
  foreach ($cycle in $cycles) {
    foreach ($capture in $cycle.Captures) {
      $panel = @($capture.Windows) | Where-Object { $_.Title -eq "Codex Island Panel" } | Select-Object -First 1
      if ($null -ne $panel) {
        $signals.Add([pscustomobject]@{
          Cycle = $cycle.Cycle
          AtMs = $capture.AtMs
          Width = $panel.Width
          Height = $panel.Height
        })
      }
    }
  }

  $signals | Sort-Object Height | Select-Object -First 1
}

function Get-PanelScrollbarSignal($cycles) {
  $signals = New-Object System.Collections.Generic.List[object]
  foreach ($cycle in $cycles) {
    foreach ($capture in $cycle.Captures) {
      $panel = @($capture.Windows) | Where-Object { $_.Title -eq "Codex Island Panel" } | Select-Object -First 1
      if ($null -eq $panel) {
        continue
      }

      $bitmap = [System.Drawing.Bitmap]::FromFile($capture.Screenshot)
      try {
        $xStart = [Math]::Max($panel.Right - 9, $panel.X)
        $xEnd = [Math]::Max($panel.Right - 5, $xStart)
        $yStart = [Math]::Min($panel.Y + 36, $panel.Bottom - 1)
        $yEnd = [Math]::Max($panel.Bottom - 14, $yStart)
        $samples = 0
        $scrollbarPixels = 0

        for ($x = $xStart; $x -le $xEnd; $x++) {
          for ($y = $yStart; $y -le $yEnd; $y += 2) {
            if ($x -lt 0 -or $x -ge $bitmap.Width -or $y -lt 0 -or $y -ge $bitmap.Height) {
              continue
            }

            $pixel = $bitmap.GetPixel($x, $y)
            $average = ($pixel.R + $pixel.G + $pixel.B) / 3
            $maxChannel = [Math]::Max($pixel.R, [Math]::Max($pixel.G, $pixel.B))
            $minChannel = [Math]::Min($pixel.R, [Math]::Min($pixel.G, $pixel.B))
            $samples++
            if ($average -ge 46 -and $average -le 160 -and ($maxChannel - $minChannel) -le 42) {
              $scrollbarPixels++
            }
          }
        }

        $ratio = 0
        if ($samples -gt 0) {
          $ratio = [Math]::Round($scrollbarPixels / $samples, 4)
        }

        $signals.Add([pscustomobject]@{
          Cycle = $cycle.Cycle
          AtMs = $capture.AtMs
          Screenshot = $capture.Screenshot
          Samples = $samples
          ScrollbarPixelRatio = $ratio
        })
      } finally {
        $bitmap.Dispose()
      }
    }
  }

  $signals | Sort-Object ScrollbarPixelRatio -Descending | Select-Object -First 1
}

function Count-VisibleSessionIndicators([string] $screenshotPath, $panel) {
  if ($null -eq $panel) {
    return [pscustomobject]@{
      Count = 0
      Centers = @()
    }
  }

  $bitmap = [System.Drawing.Bitmap]::FromFile($screenshotPath)
  try {
    $xStart = [Math]::Max($panel.X + 14, $panel.X)
    $xEnd = [Math]::Min($panel.X + 48, $panel.Right - 1)
    $yStart = [Math]::Min($panel.Y + 38, $panel.Bottom - 1)
    $yEnd = [Math]::Max($panel.Bottom - 4, $yStart)
    $blueRows = New-Object System.Collections.Generic.List[int]

    for ($y = $yStart; $y -le $yEnd; $y++) {
      $rowHasIndicator = $false
      for ($x = $xStart; $x -le $xEnd; $x++) {
        if ($x -lt 0 -or $x -ge $bitmap.Width -or $y -lt 0 -or $y -ge $bitmap.Height) {
          continue
        }

        $pixel = $bitmap.GetPixel($x, $y)
        if ($pixel.B -ge 150 -and $pixel.G -ge 105 -and $pixel.G -le 190 -and $pixel.R -le 105 -and ($pixel.B - $pixel.R) -ge 80) {
          $rowHasIndicator = $true
          break
        }
      }

      if ($rowHasIndicator) {
        $blueRows.Add($y)
      }
    }

    $centers = New-Object System.Collections.Generic.List[int]
    if ($blueRows.Count -gt 0) {
      $clusterStart = $blueRows[0]
      $clusterEnd = $blueRows[0]
      for ($index = 1; $index -lt $blueRows.Count; $index++) {
        $row = $blueRows[$index]
        if ($row -le $clusterEnd + 3) {
          $clusterEnd = $row
          continue
        }

        if (($clusterEnd - $clusterStart) -ge 4) {
          $centers.Add([int](($clusterStart + $clusterEnd) / 2))
        }
        $clusterStart = $row
        $clusterEnd = $row
      }

      if (($clusterEnd - $clusterStart) -ge 4) {
        $centers.Add([int](($clusterStart + $clusterEnd) / 2))
      }
    }

    [pscustomobject]@{
      Count = $centers.Count
      Centers = @($centers.ToArray())
    }
  } finally {
    $bitmap.Dispose()
  }
}

function Get-PanelIndicatorSignals($cycles) {
  $signals = New-Object System.Collections.Generic.List[object]
  foreach ($cycle in $cycles) {
    $best = $null
    foreach ($capture in $cycle.Captures) {
      $panel = @($capture.Windows) | Where-Object { $_.Title -eq "Codex Island Panel" } | Select-Object -First 1
      $indicatorSignal = Count-VisibleSessionIndicators $capture.Screenshot $panel
      $candidate = [pscustomobject]@{
        Cycle = $cycle.Cycle
        AtMs = $capture.AtMs
        Screenshot = $capture.Screenshot
        VisibleSessionIndicators = $indicatorSignal.Count
        IndicatorCenters = $indicatorSignal.Centers
      }

      if ($null -eq $best -or $candidate.VisibleSessionIndicators -gt $best.VisibleSessionIndicators) {
        $best = $candidate
      }
    }

    if ($null -ne $best) {
      $signals.Add($best)
    }
  }

  @($signals.ToArray())
}

$backdrop = $null
try {
  Reset-OutputDir
  Write-TestSessions
  if (-not $NoBackdrop) {
    $backdrop = Show-StaticBackdrop
  }

  $main = Start-App
  Move-Away 900
  $scenarioMoves = New-Object System.Collections.Generic.List[object]
  $cycles = New-Object System.Collections.Generic.List[object]

  $mainBefore = Wait-CollapsedMain 9000
  $cycles.Add((Run-HoverCapture "top" $mainBefore))

  foreach ($scenarioName in @("left", "right", "floating")) {
    $scenarioMoves.Add((Move-IslandForScenario $scenarioName))
    Move-Away 700
    $scenarioMainBefore = Wait-CollapsedMain 9000
    $cycles.Add((Run-HoverCapture $scenarioName $scenarioMainBefore))
  }

  $cycles = @($cycles.ToArray())
  $bestPanelSignal = Get-BestPanelSignal $cycles
  $panelWindowHeightSignal = Get-PanelWindowHeightSignal $cycles
  $panelScrollbarSignal = Get-PanelScrollbarSignal $cycles
  $panelIndicatorSignals = @(Get-PanelIndicatorSignals $cycles)
  $visualPanelDetected =
    $null -ne $bestPanelSignal -and
    $bestPanelSignal.AverageRgbDelta -ge 32 -and
    $bestPanelSignal.ChangedSampleRatio -ge 0.18
  $noPanelScrollbarDetected =
    $null -ne $panelScrollbarSignal -and
    $panelScrollbarSignal.ScrollbarPixelRatio -le 0.08
  $allSessionIndicatorsVisibleDetected =
    $panelIndicatorSignals.Count -eq $cycles.Count -and
    @($panelIndicatorSignals | Where-Object { $_.VisibleSessionIndicators -lt $SessionCount }).Count -eq 0

  $summary = [pscustomobject]@{
    Exe = $Exe
    OutputDir = $OutputDir
    SessionCount = $SessionCount
    UsedBackdrop = -not $NoBackdrop
    VirtualScreen = [pscustomobject]@{
      X = $VirtualScreen.Left
      Y = $VirtualScreen.Top
      Width = $VirtualScreen.Width
      Height = $VirtualScreen.Height
    }
    MainBefore = $mainBefore
    ScenarioMoves = @($scenarioMoves.ToArray())
    Cycles = $cycles
    BestPanelSignal = $bestPanelSignal
    PanelWindowHeightSignal = $panelWindowHeightSignal
    PanelScrollbarSignal = $panelScrollbarSignal
    PanelIndicatorSignals = $panelIndicatorSignals
    VisualPanelDetected = $visualPanelDetected
    NoPanelScrollbarDetected = $noPanelScrollbarDetected
    AllSessionIndicatorsVisibleDetected = $allSessionIndicatorsVisibleDetected
    FinalWindows = @(Get-AppWindows) | Select-Object Title, X, Y, Width, Height, Right, Bottom
  }

  $json = $summary | ConvertTo-Json -Depth 12
  $json

  if (-not $AllowInvisible -and (-not $visualPanelDetected -or -not $noPanelScrollbarDetected -or -not $allSessionIndicatorsVisibleDetected)) {
    throw $json
  }
} finally {
  if ($null -ne $backdrop) {
    $backdrop.Close()
    $backdrop.Dispose()
  }
}
