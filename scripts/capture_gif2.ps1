param([string]$OutDir = "C:/Users/user/Documents/BYOK_STT/cap2")
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace V -Name G3 -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
public delegate bool EnumProc(IntPtr h, IntPtr lp);
[DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, UIntPtr w, IntPtr l);
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int w, int h2, uint f);
'@

$script:ind = [IntPtr]::Zero
$cb = { param($h, $lp) $sb = New-Object System.Text.StringBuilder 256; [V.G3]::GetClassName($h, $sb, 256) | Out-Null; if ($sb.ToString() -eq 'BYOK_STT_INDICATOR') { $script:ind = $h }; return $true }
[V.G3]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
if ($script:ind -eq [IntPtr]::Zero) { Write-Error 'bubble not found'; exit 1 }

Add-Type -AssemblyName System.Windows.Forms
# clean white backdrop behind the bubble (covers the user's desktop)
$bg = New-Object System.Windows.Forms.Form
$bg.StartPosition = 'Manual'
$bg.Location = New-Object System.Drawing.Point(1000, 60)
$bg.Size = New-Object System.Drawing.Size(520, 460)
$bg.BackColor = [System.Drawing.Color]::White
$bg.TopMost = $true
$bg.Show()
# put the bubble back above the white backdrop
[V.G3]::SetWindowPos($script:ind, [IntPtr](-1), 1230, 240, 0, 0, 0x0043) | Out-Null  # HWND_TOPMOST | NOMOVE|NOSIZE|NOACTIVATE
Start-Sleep -Milliseconds 600

# capture region centered on the bubble at (1230,240,48x48)
$rx = 1060; $ry = 120; $rw = 400; $rh = 320
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
Get-ChildItem "$OutDir/frame_*.png" -ErrorAction SilentlyContinue | Remove-Item

$frames = New-Object System.Collections.ArrayList
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$nextCap = 0

function Capture([System.Collections.ArrayList]$list, [int]$n) {
    $bmp = New-Object System.Drawing.Bitmap($script:rw, $script:rh)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($script:rx, $script:ry, 0, 0, (New-Object System.Drawing.Size($script:rw, $script:rh)))
    $g.Dispose()
    $list.Add(@($n, $bmp)) | Out-Null
}

# idle gray
while ($sw.ElapsedMilliseconds -lt 1200) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += 100 }
    Start-Sleep -Milliseconds 15
}

# click bubble -> start (red)
[V.G3]::PostMessage($script:ind, 0x0201, [UIntPtr]::new(1), [IntPtr]((24 -shl 16) -bor 24)) | Out-Null
[V.G3]::PostMessage($script:ind, 0x0202, [UIntPtr]::new(0), [IntPtr]((24 -shl 16) -bor 24)) | Out-Null
while ($sw.ElapsedMilliseconds -lt 3200) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += 100 }
    Start-Sleep -Milliseconds 15
}

# click again -> stop (amber spinner, then gray)
[V.G3]::PostMessage($script:ind, 0x0201, [UIntPtr]::new(1), [IntPtr]((24 -shl 16) -bor 24)) | Out-Null
[V.G3]::PostMessage($script:ind, 0x0202, [UIntPtr]::new(0), [IntPtr]((24 -shl 16) -bor 24)) | Out-Null
while ($sw.ElapsedMilliseconds -lt 7500) {
    if ($sw.ElapsedMilliseconds -ge $nextCap) { Capture $frames $nextCap; $nextCap += 100 }
    Start-Sleep -Milliseconds 15
}

$outW = 400; $outH = 320
foreach ($f in $frames) {
    $f[1].Save("$OutDir/frame_{0:D4}.png" -f $f[0], [System.Drawing.Imaging.ImageFormat]::Png)
    $f[1].Dispose()
}
Write-Host ('frames captured: ' + $frames.Count)
