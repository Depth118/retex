param([string]$Dll)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class N {
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi, SetLastError=true)] public static extern IntPtr LoadLibraryA(string p);
  [DllImport("kernel32.dll", CharSet=CharSet.Ansi)] public static extern IntPtr GetModuleHandleA(string p);
  [DllImport("kernel32.dll")] public static extern IntPtr VirtualAlloc(IntPtr a, UIntPtr s, uint t, uint p);
  [DllImport("kernel32.dll")] public static extern bool VirtualFree(IntPtr a, UIntPtr s, uint t);
  [DllImport("kernel32.dll")] public static extern bool FreeLibrary(IntPtr h);

  [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
  public delegate void TableInit(IntPtr self);
  [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
  public delegate uint CrcFn(IntPtr self, IntPtr data, uint len);
}
"@
$h = [N]::LoadLibraryA($Dll)
"LoadLibrary=0x{0:X} err={1}" -f $h.ToInt64(), [Runtime.InteropServices.Marshal]::GetLastWin32Error()
if ($h -eq [IntPtr]::Zero) { exit 1 }
$name = [IO.Path]::GetFileName($Dll)
$base = [N]::GetModuleHandleA($name)
"base=0x{0:X}" -f $base.ToInt64()

# object holding a table pointer (16 bytes is plenty)
$obj = [Runtime.InteropServices.Marshal]::AllocHGlobal(16)
# data buffer
$len = 1024
$data = [Runtime.InteropServices.Marshal]::AllocHGlobal($len)
$bytes = New-Object byte[] $len
for ($i = 0; $i -lt $len; $i++) { $bytes[$i] = $i % 256 }
[Runtime.InteropServices.Marshal]::Copy($bytes, 0, $data, $len)

$initPtr = [IntPtr]($base.ToInt64() + 0x8C00)
$crcPtr  = [IntPtr]($base.ToInt64() + 0x8CE0)
"init=0x{0:X} crc=0x{1:X}" -f $initPtr.ToInt64(), $crcPtr.ToInt64()

$init = [Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer($initPtr, [N+TableInit])
$crc  = [Runtime.InteropServices.Marshal]::GetDelegateForFunctionPointer($crcPtr,  [N+CrcFn])
$init.Invoke($obj)
$result = $crc.Invoke($obj, $data, [uint32]$len)
"TexMod CRC = 0x{0:X8}" -f $result

