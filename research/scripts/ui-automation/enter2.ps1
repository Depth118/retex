Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class K2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static IntPtr Find(uint pid, string cls, string text){
    IntPtr f=IntPtr.Zero;
    EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls&&T(h)==text){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
  public static IntPtr ById(IntPtr parent, string cls, int id){
    IntPtr f=IntPtr.Zero;
    EnumChildWindows(parent,(h,l)=>{if(C(h)==cls&&GetDlgCtrlID(h)==id){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
}
"@
[K2]::SetProcessDPIAware() | Out-Null
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [K2]::Find($procId, '#32770', 'Select Executable')
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
$open = [K2]::ById($dlg, 'Button', 1)
[K2]::SetForegroundWindow($dlg) | Out-Null
Start-Sleep -Milliseconds 700
"fg=0x{0:X} dlg=0x{1:X}" -f [K2]::GetForegroundWindow().ToInt64(), $dlg.ToInt64()
[K2]::keybd_event(0x0D,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 90; [K2]::keybd_event(0x0D,0,2,[UIntPtr]::Zero)
Start-Sleep -Seconds 2
"closed after Enter: " + ([K2]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero)
if ([K2]::Find($procId, '#32770', 'Select Executable') -ne [IntPtr]::Zero) {
  $r = New-Object K2+RECT; [K2]::GetWindowRect($open, [ref]$r) | Out-Null
  $x = [int](($r.L+$r.R)/2); $y = [int](($r.T+$r.B)/2)
  "clicking $x,$y"
  [K2]::SetCursorPos($x,$y); Start-Sleep -Milliseconds 250
  [K2]::mouse_event(2,0,0,0,[UIntPtr]::Zero); Start-Sleep -Milliseconds 120
  [K2]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
  Start-Sleep -Seconds 2
  "closed after phys click: " + ([K2]::Find($procId, '#32770', 'Select Executable') -eq [IntPtr]::Zero)
}
$tops = @()
foreach ($h in @()) {}
"main title='" + [K2]::T($p.MainWindowHandle) + "'"
