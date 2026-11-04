param([switch]$AppOnly)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$taskDesktop = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$taskWorkspace = [System.IO.Path]::GetFullPath((Join-Path $taskDesktop '../..'))
$taskBrandSource = [System.IO.File]::ReadAllText((Join-Path $taskDesktop 'src/renderer/components/BrandMark.tsx'))
$taskTokenSource = [System.IO.File]::ReadAllText((Join-Path $taskWorkspace 'packages/design-system/src/tokens.css'))
$taskLetters = [ordered]@{}
foreach ($taskLetter in @('v', 'u', 'a')) {
  $taskGroup = [regex]::Match($taskBrandSource, '(?s)\b' + $taskLetter + ': \[(.*?)\]').Groups[1].Value
  $taskLetters[$taskLetter] = @([regex]::Matches($taskGroup, '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
  if ($taskLetters[$taskLetter].Count -eq 0) { throw "Missing brand geometry: $taskLetter" }
}
function Get-TaskColor($taskBlock, $taskName) {
  $taskMatch = [regex]::Match($taskBlock, '--vua-' + $taskName + ':\s*(#[a-fA-F0-9]{6})')
  if (-not $taskMatch.Success) { throw "Missing brand token: $taskName" }
  return [System.Drawing.ColorTranslator]::FromHtml($taskMatch.Groups[1].Value)
}
function Get-TaskPolygon($taskPath) {
  $taskTokens = @([regex]::Matches($taskPath, '[MLHVZ]|-?\d+(?:\.\d+)?') | ForEach-Object { $_.Value })
  $taskPoints = [System.Collections.Generic.List[System.Drawing.PointF]]::new()
  $taskX = 0.0; $taskY = 0.0; $taskIndex = 0
  while ($taskIndex -lt $taskTokens.Count) {
    $taskCommand = $taskTokens[$taskIndex++]
    switch ($taskCommand) {
      { $_ -in 'M', 'L' } {
        $taskX = [float]::Parse($taskTokens[$taskIndex++], [cultureinfo]::InvariantCulture)
        $taskY = [float]::Parse($taskTokens[$taskIndex++], [cultureinfo]::InvariantCulture)
      }
      'H' { $taskX = [float]::Parse($taskTokens[$taskIndex++], [cultureinfo]::InvariantCulture) }
      'V' { $taskY = [float]::Parse($taskTokens[$taskIndex++], [cultureinfo]::InvariantCulture) }
      'Z' { continue }
      default { throw "Unsupported brand path command: $taskCommand" }
    }
    if ($taskCommand -ne 'Z') { $taskPoints.Add([System.Drawing.PointF]::new($taskX, $taskY)) }
  }
  return ,$taskPoints.ToArray()
}
$taskDarkTokens = [regex]::Match($taskTokenSource, '(?s):root\s*\{(.*?)\}').Groups[1].Value
$taskLightTokens = [regex]::Match($taskTokenSource, '(?s)\[data-theme="light"\]\s*\{(.*?)\}').Groups[1].Value
$taskIcons = [ordered]@{}
$taskAppImages = [ordered]@{}
foreach ($taskTheme in @('dark', 'light')) {
  $taskBlock = if ($taskTheme -eq 'dark') { $taskDarkTokens } else { $taskLightTokens }
  $taskColors = @{ v=(Get-TaskColor $taskBlock 'purple'); u=(Get-TaskColor $taskBlock 'text-strong'); a=(Get-TaskColor $taskBlock 'orange') }
  $taskIcons[$taskTheme] = @()
  $taskSizes = if ($taskTheme -eq 'dark') { @(16, 20, 24, 32, 48, 64, 128, 256) } else { @(16, 20, 24, 32) }
  foreach ($taskSize in $taskSizes) {
    $taskCanvas = [System.Drawing.Bitmap]::new($taskSize * 4, $taskSize * 4)
    $taskGraphics = [System.Drawing.Graphics]::FromImage($taskCanvas)
    $taskGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $taskGraphics.ScaleTransform($taskSize * 4 / 200.0, $taskSize * 4 / 200.0)
    foreach ($taskLetter in @('v', 'u', 'a')) {
      $taskBrush = [System.Drawing.SolidBrush]::new($taskColors[$taskLetter])
      foreach ($taskPath in $taskLetters[$taskLetter]) { $taskGraphics.FillPolygon($taskBrush, (Get-TaskPolygon $taskPath)) }
      $taskBrush.Dispose()
    }
    $taskGraphics.Dispose()
    $taskBitmap = [System.Drawing.Bitmap]::new($taskSize, $taskSize)
    $taskResize = [System.Drawing.Graphics]::FromImage($taskBitmap)
    $taskResize.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $taskResize.DrawImage($taskCanvas, 0, 0, $taskSize, $taskSize)
    $taskResize.Dispose(); $taskCanvas.Dispose()
    $taskStream = [System.IO.MemoryStream]::new()
    $taskBitmap.Save($taskStream, [System.Drawing.Imaging.ImageFormat]::Png)
    $taskBytes = $taskStream.ToArray()
    if ($taskSize -le 32) { $taskIcons[$taskTheme] += [ordered]@{ scaleFactor=($taskSize / 16); png=[Convert]::ToBase64String($taskBytes) } }
    if ($taskTheme -eq 'dark') { $taskAppImages[[string]$taskSize] = $taskBytes }
    $taskBitmap.Dispose(); $taskStream.Dispose()
  }
}
$taskOutput = "// Generated from BrandMark.tsx and design tokens by scripts/generate-tray-icons.ps1.`n// Transparent PNGs retain the accepted VUA geometry at Windows tray DPI scales.`nexport const trayIcons = " + ($taskIcons | ConvertTo-Json -Depth 5) + " as const;`n"
if (-not $AppOnly) { [System.IO.File]::WriteAllText((Join-Path $taskDesktop 'src/electron/tray-icons.ts'), $taskOutput, [System.Text.UTF8Encoding]::new($false)) }
$taskAppPng = [Convert]::ToBase64String($taskAppImages['256'])
$taskAppSource = "// Generated from BrandMark.tsx by scripts/generate-tray-icons.ps1. Same geometry as packaging/icon.ico.`nexport const appIconPng = '$taskAppPng';`n"
[System.IO.File]::WriteAllText((Join-Path $taskDesktop 'src/electron/app-icon-data.ts'), $taskAppSource, [System.Text.UTF8Encoding]::new($false))
$taskIconStream = [System.IO.MemoryStream]::new()
$taskWriter = [System.IO.BinaryWriter]::new($taskIconStream)
$taskWriter.Write([uint16]0); $taskWriter.Write([uint16]1); $taskWriter.Write([uint16]$taskAppImages.Count)
$taskOffset = 6 + 16 * $taskAppImages.Count
foreach ($taskSize in $taskAppImages.Keys) {
  $taskEdge = if ($taskSize -eq 256) { 0 } else { $taskSize }
  $taskWriter.Write([byte]$taskEdge); $taskWriter.Write([byte]$taskEdge)
  $taskWriter.Write([uint16]0); $taskWriter.Write([uint16]1); $taskWriter.Write([uint16]32)
  $taskWriter.Write([uint32]$taskAppImages[$taskSize].Length); $taskWriter.Write([uint32]$taskOffset)
  $taskOffset += $taskAppImages[$taskSize].Length
}
foreach ($taskBytes in $taskAppImages.Values) { $taskWriter.Write([byte[]]$taskBytes) }
$taskWriter.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $taskDesktop 'packaging/icon.ico'), $taskIconStream.ToArray())
$taskWriter.Dispose(); $taskIconStream.Dispose()
Write-Output 'Generated VUA tray icons for dark/light system themes and 100/125/150/200% DPI.'
Write-Output 'Generated the VUA application icon and multi-resolution Windows ICO.'
