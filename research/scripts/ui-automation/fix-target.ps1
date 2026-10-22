Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class U {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr p, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr h, string t);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static List<IntPtr> Kids(IntPtr p){var l=new List<IntPtr>();EnumChildWindows(p,(h,x)=>{l.Add(h);return true;},IntPtr.Zero);return l;}
  public static IntPtr WinByClass(uint pid, string cls, string text){
    IntPtr f=IntPtr.Zero;
    EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls&&(text==null||T(h)==text)){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
  public static void Click(IntPtr h){PostMessage(h,0x00F5,IntPtr.Zero,IntPtr.Zero);}
}
"@
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$pid32 = [uint32]$p.Id
# 1) dismiss the invalid-application message box if present
$err = [U]::WinByClass($pid32, '#32770', 'Invalid Application Selected')
"err=$err"
if ($err -ne [IntPtr]::Zero) {
  foreach ($c in [U]::Kids($err)) { if ([U]::C($c) -eq 'Button' -and [U]::T($c) -eq 'OK') { [U]::Click($c); "clicked err OK"; break } }
  Start-Sleep -Milliseconds 500
}
# 2) set the path in the file dialog with SetWindowTextW
$dlg = [U]::WinByClass($pid32, '#32770', 'Select Executable')
"dlg=$dlg class=$([U]::C($dlg)) text='$([U]::T($dlg))'"
if ($dlg -eq [IntPtr]::Zero) { exit }
$edit = [IntPtr]::Zero
foreach ($c in [U]::Kids($dlg)) { if ([U]::C($c) -eq 'Edit' -and [U]::GetDlgCtrlID($c) -eq 1148) { $edit = $c; break } }
"edit=$edit before='$([U]::T($edit))'"
$ok = [U]::SetWindowText($edit, 'C:\Windows\SysWOW64\notepad.exe')
"setwindowtext=$ok after='$([U]::T($edit))'"
# also set the ComboBox text
foreach ($c in [U]::Kids($dlg)) { if ([U]::C($c) -eq 'ComboBox' -and [U]::GetDlgCtrlID($c) -eq 1148) { [U]::SetWindowText($c, 'C:\Windows\SysWOW64\notepad.exe'); "combo set: '$([U]::T($c))'" } }
Start-Sleep -Milliseconds 600
# 3) click Open
foreach ($c in [U]::Kids($dlg)) { if ([U]::C($c) -eq 'Button' -and [U]::GetDlgCtrlID($c) -eq 1) { [U]::Click($c); "clicked open"; break } }
Start-Sleep -Seconds 2
$main = [U]::WinByClass($pid32, 'tmlwndcls', $null)
"main title now='$([U]::T($main))'"
foreach ($c in [U]::Kids($main)) { $t=[U]::T($c); if ($t -match 'exe|notepad|Speed') { "  child $([U]::C($c)) id=$([U]::GetDlgCtrlID($c)) text='$t'" } }
