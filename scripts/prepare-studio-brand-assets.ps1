# Encode the approved square artwork into application and browser icon formats.
Add-Type -AssemblyName System.Drawing
$brandRoot = (Resolve-Path "$PSScriptRoot/../landing/studio/brand").Path
$brandSource = [System.Drawing.Image]::FromFile("$brandRoot/fframes-studio-icon-blue-v1.png")
try {
    $iconPayloads = @()
    foreach ($iconSize in @(16, 24, 32, 48, 64, 128, 180, 192, 256, 512)) {
        $iconBitmap = [System.Drawing.Bitmap]::new($iconSize, $iconSize, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $iconGraphics = [System.Drawing.Graphics]::FromImage($iconBitmap)
        $iconGraphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
        $iconGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $iconGraphics.DrawImage($brandSource, 0, 0, $iconSize, $iconSize)
        $iconGraphics.Dispose()
        $iconStream = [System.IO.MemoryStream]::new()
        $iconBitmap.Save($iconStream, [System.Drawing.Imaging.ImageFormat]::Png)
        $iconBytes = $iconStream.ToArray()
        if ($iconSize -le 256) { $iconPayloads += ,@{ Size = $iconSize; Bytes = $iconBytes } }
        if ($iconSize -eq 32) { [System.IO.File]::WriteAllBytes("$brandRoot/favicon.png", $iconBytes) }
        if ($iconSize -eq 180) { [System.IO.File]::WriteAllBytes("$brandRoot/apple-touch-icon.png", $iconBytes) }
        if ($iconSize -eq 192) { [System.IO.File]::WriteAllBytes("$brandRoot/icon-192.png", $iconBytes) }
        if ($iconSize -eq 512) { [System.IO.File]::WriteAllBytes("$brandRoot/icon-512.png", $iconBytes) }
        if ($iconSize -eq 512) { [System.IO.File]::WriteAllBytes("$PSScriptRoot/../desktop/app/assets/branding/studio-icon.png", $iconBytes) }
        $iconBitmap.Dispose()
        $iconStream.Dispose()
    }
    $icoStream = [System.IO.MemoryStream]::new()
    $icoWriter = [System.IO.BinaryWriter]::new($icoStream)
    $icoWriter.Write([uint16]0); $icoWriter.Write([uint16]1); $icoWriter.Write([uint16]$iconPayloads.Count)
    $icoOffset = 6 + 16 * $iconPayloads.Count
    foreach ($payload in $iconPayloads) {
        $encodedSize = if ($payload.Size -eq 256) { 0 } else { $payload.Size }
        $icoWriter.Write([byte]$encodedSize); $icoWriter.Write([byte]$encodedSize)
        $icoWriter.Write([byte]0); $icoWriter.Write([byte]0)
        $icoWriter.Write([uint16]1); $icoWriter.Write([uint16]32)
        $icoWriter.Write([uint32]$payload.Bytes.Length); $icoWriter.Write([uint32]$icoOffset)
        $icoOffset += $payload.Bytes.Length
    }
    foreach ($payload in $iconPayloads) { $icoWriter.Write([byte[]]$payload.Bytes) }
    [System.IO.File]::WriteAllBytes("$PSScriptRoot/../desktop/packaging/windows/fframes-studio.ico", $icoStream.ToArray())
    [System.IO.File]::WriteAllBytes("$brandRoot/favicon.ico", $icoStream.ToArray())
    $icoWriter.Dispose(); $icoStream.Dispose()
} finally { $brandSource.Dispose() }
