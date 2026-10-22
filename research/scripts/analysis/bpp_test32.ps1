param([string]$Dll)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class P2 {
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi)] public static extern IntPtr LoadLibraryA(string p);
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi)] public static extern IntPtr GetModuleHandleA(string p);
  [UnmanagedFunctionPointer(CallingConvention.StdCall)]
  public delegate float BppFn(uint fmt);
}
"@
$h = [P2]::LoadLibraryA($Dll)
if ($h -eq [IntPtr]::Zero) { 'load failed'; exit 1 }
$base = [P2]::GetModuleHandleA([IO.Path]::GetFileName($Dll))
"base=0x{0:X}" -f $base.ToInt64()
$fn = [Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer([IntPtr]($base.ToInt64() + 0x8A50), [P2+BppFn])
for ($fmt = 0; $fmt -le 0x80; $fmt++) { $v = $fn.Invoke([uint32]$fmt); if ($v -ne 0.5) { "0x{0:X2} = {1}" -f $fmt, $v } }
"fourcc:"
foreach ($fcc in 0x31545844,0x32545844,0x33545844,0x34545844,0x35545844) { "0x{0:X8} = {1}" -f $fcc, $fn.Invoke([uint32]$fcc) }

