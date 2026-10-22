Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class Q {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Top(uint pid){var l=new List<IntPtr>();EnumWindows((h,x)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid)l.Add(h);return true;},IntPtr.Zero);return l;}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
}
"@
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [IntPtr]::Zero
foreach ($h in [Q]::Top($procId)) { if ([Q]::C($h) -eq '#32770' -and [Q]::T($h) -eq 'Select Executable') { $dlg = $h; break } }
"dlg=0x{0:X}" -f $dlg.ToInt64()
$kids = [Q]::Kids($dlg)
"kids=$($kids.Count)"
$open = [IntPtr]::Zero
foreach ($c in $kids) { if ([Q]::C($c) -eq 'Button' -and [Q]::GetDlgCtrlID($c) -eq 1) { $open = $c; "found open=0x{0:X} text='{1}'" -f $c.ToInt64(), [Q]::T($c); break } }
if ($open -eq [IntPtr]::Zero) { 'open not found'; exit }
[Q]::PostMessage($open, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
'posted BM_CLICK'
Start-Sleep -Seconds 2
$still = $false
foreach ($h in [Q]::Top($procId)) { if ([Q]::C($h) -eq '#32770' -and [Q]::T($h) -eq 'Select Executable') { $still = $true } }
"still open=$still"
foreach ($h in [Q]::Top($procId)) { "top cls='{0}' text='{1}'" -f [Q]::C($h), [Q]::T($h) }
