param([string]$OutFile = "C:/Users/user/Documents/BYOK_STT/docs/settings.png")
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace V -Name CS -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
public delegate bool EnumProc(IntPtr h, IntPtr lp);
[DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
'@
$p = Start-Process -FilePath 'C:/Users/user/Documents/BYOK_STT/target/release/byok-stt.exe' -ArgumentList '--settings' -PassThru
Start-Sleep -Milliseconds 2500
$script:hw = [IntPtr]::Zero
$target = $p.Id
$cb = { param($h, $lp)
  $pid2 = 0
  [V.CS]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
  if ($pid2 -eq $target) {
    $sb = New-Object System.Text.StringBuilder 256
    [V.CS]::GetClassName($h, $sb, 256) | Out-Null
    if ($sb.ToString() -eq 'BYOK_STT_SETTINGS') { $script:hw = $h }
  }
  return $true
}
[V.CS]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
if ($script:hw -eq [IntPtr]::Zero) { Write-Host 'NOT FOUND'; Stop-Process -Id $target -Force; exit 1 }
$bmp = New-Object System.Drawing.Bitmap(472, 332)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[V.CS]::PrintWindow($script:hw, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc); $g.Dispose()
$big = New-Object System.Drawing.Bitmap($bmp, 944, 664)
$big.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
$big.Dispose(); $bmp.Dispose()
Write-Host ('saved ' + $OutFile)
Stop-Process -Id $target -Force -ErrorAction SilentlyContinue
