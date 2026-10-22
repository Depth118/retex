Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class V {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
}
"@
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$tops = [V]::Top($procId)
"--- top-level windows ---"
foreach ($h in $tops) { "0x{0:X} cls='{1}' enabled={2} text='{3}'" -f $h.ToInt64(), [V]::C($h), [V]::IsWindowEnabled($h), [V]::T($h) }
$dlg = $tops | Where-Object { [V]::C($_) -eq '#32770' -and [V]::T($_) -eq 'Select Executable' } | Select-Object -First 1
"dlg=0x{0:X} enabled={1}" -f $dlg.ToInt64(), [V]::IsWindowEnabled($dlg)
# WM_COMMAND IDOK
[V]::PostMessage($dlg, 0x0111, [IntPtr]1, [IntPtr]::Zero) | Out-Null
Start-Sleep -Seconds 2
"--- after IDOK ---"
foreach ($h in [V]::Top($procId)) { "0x{0:X} cls='{1}' text='{2}'" -f $h.ToInt64(), [V]::C($h), [V]::T($h) }
$main = [V]::Top($procId) | Where-Object { [V]::C($_) -eq 'tmlwndcls' } | Select-Object -First 1
"main=0x{0:X} title='{1}'" -f $main.ToInt64(), [V]::T($main)
foreach ($c in [V]::Kids($main)) { $t=[V]::T($c); if ($t) { "  child {0} text='{1}'" -f [V]::C($c), $t } }
