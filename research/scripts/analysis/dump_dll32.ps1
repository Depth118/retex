param([string]$Path, [string]$Out)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class L32 {
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi, SetLastError=true)] public static extern IntPtr LoadLibraryA(string p);
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi)] public static extern IntPtr GetModuleHandleA(string p);
  [DllImport("kernel32.dll")] public static extern bool FreeLibrary(IntPtr h);
}
"@
$bytes = [IO.File]::ReadAllBytes($Path)
$lf = [BitConverter]::ToInt32($bytes, 0x3C)
$opt = $lf + 24
$sizeImage = [BitConverter]::ToUInt32($bytes, $opt + 56)
$name = [IO.Path]::GetFileName($Path)
"loading $name sizeImage=0x{0:X}" -f $sizeImage
$h = [L32]::LoadLibraryA($Path)
"LoadLibrary=0x{0:X} err={1}" -f $h.ToInt64(), [Runtime.InteropServices.Marshal]::GetLastWin32Error()
if ($h -eq [IntPtr]::Zero) { exit 1 }
$base = [L32]::GetModuleHandleA($name)
"base=0x{0:X}" -f $base.ToInt64()
$buf = New-Object byte[] $sizeImage
$key = 0xAA
[Runtime.InteropServices.Marshal]::Copy($base, $buf, 0, [int]$sizeImage)
for ($i = 0; $i -lt $buf.Length; $i++) { $buf[$i] = $buf[$i] -bxor $key }
[IO.File]::WriteAllBytes($Out, $buf)
"dumped {0} bytes to {1}" -f $buf.Length, $Out

