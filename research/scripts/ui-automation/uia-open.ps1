Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class P {
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
$dlg = [P]::Find($procId, '#32770', 'Select Executable')
"dlg=0x{0:X}" -f $dlg.ToInt64()
if ($dlg -eq [IntPtr]::Zero) { exit }
$root = [System.Windows.Automation.AutomationElement]::FromHandle($dlg)
"name='$($root.Current.Name)' class='$($root.Current.ClassName)'"
$btn = $null
foreach ($id in @('1','OpenButton')) {
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, $id)
  $b = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
  if ($b -ne $null) { $btn = $b; "found by AutomationId '$id' name='$($b.Current.Name)'"; break }
}
if ($btn -eq $null) {
  $cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Open')
  $btn = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
  if ($btn -ne $null) { "found by Name 'Open'" }
}
if ($btn -eq $null) { 'button not found'; exit }
$inv = $btn.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
$inv.Invoke()
'invoked'
Start-Sleep -Seconds 2
"still open=" + ([P]::Find($procId, '#32770', 'Select Executable') -ne [IntPtr]::Zero)
"main title='" + [P]::T($p.MainWindowHandle) + "'"
