Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class X {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  public struct RECT { public int L, T, R, B; }
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static void PhysClick(int x, int y){SetCursorPos(x,y);System.Threading.Thread.Sleep(150);mouse_event(2,0,0,0,UIntPtr.Zero);System.Threading.Thread.Sleep(80);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
"@
[X]::SetProcessDPIAware() | Out-Null
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$tops = [X]::Top($procId)
$dlg = $tops | Where-Object { [X]::C($_) -eq '#32770' -and [X]::T($_) -eq 'Select Executable' } | Select-Object -First 1
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit }
[X]::SetForegroundWindow($dlg) | Out-Null
Start-Sleep -Milliseconds 400
$open = [IntPtr]::Zero
foreach ($c in [X]::Kids($dlg)) { if ([X]::C($c) -eq 'Button' -and [X]::GetDlgCtrlID($c) -eq 1 -and [X]::IsWindowVisible($c)) { $open = $c; break } }
"open=0x{0:X} text='{1}'" -f $open.ToInt64(), [X]::T($open)
$r = New-Object X+RECT
[X]::GetWindowRect($open, [ref]$r) | Out-Null
"rect: {0},{1} - {2},{3}" -f $r.L,$r.T,$r.R,$r.B
[X]::PhysClick([int](($r.L+$r.R)/2), [int](($r.T+$r.B)/2))
Start-Sleep -Seconds 2
$still = ([X]::Top($procId) | Where-Object { [X]::C($_) -eq '#32770' -and [X]::T($_) -eq 'Select Executable' }).Count
"dialog still open: $still"
if ($still -gt 0) {
  "trying Enter..."
  [X]::PostMessage($dlg, 0x0100, [IntPtr]0x0D, [IntPtr]0) | Out-Null
  [X]::PostMessage($dlg, 0x0101, [IntPtr]0x0D, [IntPtr]0) | Out-Null
  Start-Sleep -Seconds 2
  "dialog still open after enter: " + ([X]::Top($procId) | Where-Object { [X]::C($_) -eq '#32770' -and [X]::T($_) -eq 'Select Executable' }).Count
}
$main = [X]::Top($procId) | Where-Object { [X]::C($_) -eq 'tmlwndcls' } | Select-Object -First 1
"main title='{0}'" -f [X]::T($main)
foreach ($c in [X]::Kids($main)) { $t=[X]::T($c); if ($t -and [X]::C($c) -ne 'Static') { "  {0} text='{1}'" -f [X]::C($c), $t } }
