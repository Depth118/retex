Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class R2 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  public static string C(IntPtr h){var s=new StringBuilder(128);GetClassName(h,s,128);return s.ToString();}
  public static string T(IntPtr h){var s=new StringBuilder(512);GetWindowText(h,s,512);return s.ToString();}
  public static IntPtr Find(uint pid, string cls, string text){
    IntPtr f=IntPtr.Zero;
    EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&C(h)==cls&&T(h)==text){f=h;return false;}return true;},IntPtr.Zero);
    return f;
  }
}
"@
$p = Get-Process Texmod -ErrorAction SilentlyContinue
if (-not $p) { 'no texmod'; exit }
$procId = [uint32]$p.Id
$dlg = [R2]::Find($procId, '#32770', 'Select Executable')
"dlg=0x{0:X}" -f $dlg.ToInt64()
if ($dlg -eq [IntPtr]::Zero) { exit }
$root = [System.Windows.Automation.AutomationElement]::FromHandle($dlg)
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
$btns = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
"buttons: $($btns.Count)"
$target = $null
foreach ($b in $btns) {
  $n = $b.Current.Name; $a = $b.Current.AutomationId; $e = $b.Current.IsEnabled
  $pats = ($b.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) -join ','
  "  name='$n' auto='$a' enabled=$e patterns=$pats"
  if ($n -eq 'Open' -or $n -eq '&Open') { $target = $b }
}
if ($target -eq $null) { 'no Open button element'; exit }
$inv = $target.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
$inv.Invoke()
'invoked Open'
Start-Sleep -Seconds 2
"still open=" + ([R2]::Find($procId, '#32770', 'Select Executable') -ne [IntPtr]::Zero)
"main title='" + [R2]::T($p.MainWindowHandle) + "'"
