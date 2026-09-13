param([string]$OutDir = "C:/Users/user/Documents/BYOK_STT/cap")
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -Namespace V -Name R2 -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
public delegate bool EnumProc(IntPtr h, IntPtr lp);
[DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, UIntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int h2, uint f);
'@

# find app windows
$script:main = [IntPtr]::Zero
$script:ind = [IntPtr]::Zero
$cb = { param($h, $lp)
  $sb = New-Object System.Text.StringBuilder 256
  [V.R2]::GetClassName($h, $sb, 256) | Out-Null
  $c = $sb.ToString()
  if ($c -eq 'BYOK_STT_MAIN') { $script:main = $h }
  elseif ($c -eq 'BYOK_STT_INDICATOR') { $script:ind = $h }
  return $true
}
[V.R2]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
if ($script:main -eq [IntPtr]::Zero -or $script:ind -eq [IntPtr]::Zero) { Write-Error 'app windows not found'; exit 1 }

# clean white demo paste-target window
$form = New-Object System.Windows.Forms.Form
$form.Text = 'byok-stt demo'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(300, 200)
$form.Size = New-Object System.Drawing.Size(990, 580)
$form.BackColor = [System.Drawing.Color]::White
$tb = New-Object System.Windows.Forms.TextBox
$tb.Multiline = $true
$tb.Dock = 'Fill'
$tb.Font = New-Object System.Drawing.Font('Segoe UI', 16)
$tb.BorderStyle = 'None'
$form.Controls.Add($tb)
$form.TopMost = $true
$form.Show()
[System.Windows.Forms.Application]::DoEvents()

# move the bubble window next to the text area
[V.R2]::SetWindowPos($script:ind, [IntPtr]::Zero, 1230, 240, 0, 0, 0x0054) | Out-Null  # move only; topmost re-asserted later

# focus the text box for the paste
$tb.Focus()
[System.Windows.Forms.Application]::DoEvents()
# CRITICAL: re-assert TOPMOST after the form exists, or the form covers the bubble
[V.R2]::SetWindowPos($script:ind, [IntPtr](-1), 0, 0, 0, 0, 0x0043) | Out-Null  # HWND_TOPMOST | SWP_NOMOVE|NOSIZE|NOACTIVATE


New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Get-ChildItem "$OutDir/frame_*.png" -ErrorAction SilentlyContinue | Remove-Item

# capture region: the demo form + bubble, nothing else
$rx = 295; $ry = 195; $rw = 1010; $rh = 590
$outW = 758; $outH = 443

$frames = New-Object System.Collections.ArrayList
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$fps = 100
$nextCap = 0

function Capture([System.Collections.ArrayList]$list, [int]$n) {
    $bmp = New-Object System.Drawing.Bitmap($script:rw, $script:rh)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($script:rx, $script:ry, 0, 0, (New-Object System.Drawing.Size($script:rw, $script:rh)))
    $g.Dispose()
    $list.Add(@($n, $bmp)) | Out-Null
}

while ($sw.ElapsedMilliseconds -lt 1000) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += $fps }
    Start-Sleep -Milliseconds 15
}

[V.R2]::PostMessage($script:main, 0x8001, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null  # start

while ($sw.ElapsedMilliseconds -lt 3400) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += $fps }
    Start-Sleep -Milliseconds 15
}

[V.R2]::PostMessage($script:main, 0x8002, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null  # stop -> transcribing

while ($sw.ElapsedMilliseconds -lt 6500) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += $fps }
    Start-Sleep -Milliseconds 15
}

# demo the "typed" result programmatically — no focus stealing from the user
$null = $tb.AppendText("Hello! This sentence was typed by byok-stt voice typing.")
while ($sw.ElapsedMilliseconds -lt 9500) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += $fps }
    Start-Sleep -Milliseconds 15
}

foreach ($f in $frames) {
    $bmp = $f[1]
    $small = New-Object System.Drawing.Bitmap($outW, $outH)
    $g2 = [System.Drawing.Graphics]::FromImage($small)
    $g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g2.DrawImage($bmp, 0, 0, $outW, $outH)
    $g2.Dispose()
    $small.Save("$OutDir/frame_{0:D4}.png" -f $f[0], [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose(); $small.Dispose()
}
Write-Host ('frames captured: ' + $frames.Count)
$form.Close()
