//! 32-bit launcher: starts a target suspended, injects `retex_hook.dll`,
//! calls its exported `Initialize`, then resumes the process.
//!
//! Usage: retex-inject <hook.dll> <target.exe> [target args...]
//!
//! The hook reads `retex.ini` next to the target executable (same as the ASI
//! mode), so the launcher only deals with process creation and injection.

#![allow(clippy::manual_c_str_literals)]

use std::ffi::{c_void, OsStr};
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

type Handle = *mut c_void;

const CREATE_SUSPENDED: u32 = 0x0000_0004;
const MEM_COMMIT_RESERVE: u32 = 0x0000_3000;
const PAGE_READWRITE: u32 = 0x0000_0004;
const INFINITE: u32 = 0xFFFF_FFFF;

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count_chars: u32,
    y_count_chars: u32,
    fill_attribute: u32,
    flags: u32,
    show_window: u16,
    cb_reserved2: u16,
    reserved2: *mut u8,
    std_input: *mut c_void,
    std_output: *mut c_void,
    std_error: *mut c_void,
}

#[repr(C)]
struct ProcessInformation {
    process: Handle,
    thread: Handle,
    process_id: u32,
    thread_id: u32,
}

type ThreadStart = unsafe extern "system" fn(*mut c_void) -> u32;

extern "system" {
    fn CreateProcessW(
        application: *const u16,
        command_line: *mut u16,
        process_attributes: *mut c_void,
        thread_attributes: *mut c_void,
        inherit_handles: i32,
        flags: u32,
        environment: *mut c_void,
        current_directory: *const u16,
        startup_info: *mut StartupInfoW,
        process_info: *mut ProcessInformation,
    ) -> i32;
    fn VirtualAllocEx(
        process: Handle,
        address: *mut c_void,
        size: usize,
        ty: u32,
        protect: u32,
    ) -> *mut c_void;
    fn WriteProcessMemory(
        process: Handle,
        address: *mut c_void,
        buffer: *const c_void,
        size: usize,
        written: *mut usize,
    ) -> i32;
    fn CreateRemoteThread(
        process: Handle,
        attributes: *mut c_void,
        stack_size: usize,
        start: ThreadStart,
        parameter: *mut c_void,
        flags: u32,
        thread_id: *mut u32,
    ) -> Handle;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn GetExitCodeThread(handle: Handle, code: *mut u32) -> i32;
    fn ResumeThread(handle: Handle) -> u32;
    fn TerminateProcess(process: Handle, exit_code: u32) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetLastError() -> u32;
}

fn error(context: &str) -> ! {
    unsafe {
        eprintln!("retex-inject: {context} (win32 {})", GetLastError());
    }
    std::process::exit(1);
}

/// Abort after CreateProcess: kill the still-suspended target and report.
fn inject_failed(process: &ProcessInformation, context: &str) -> ! {
    unsafe {
        let code = GetLastError();
        TerminateProcess(process.process, 1);
        CloseHandle(process.thread);
        CloseHandle(process.process);
        eprintln!("retex-inject: {context} (win32 {code})");
        if code == 5 {
            eprintln!(
                "hint: the target may be a packaged/protected process (e.g. Windows 11 \
                 Notepad); install the ASI hook instead"
            );
        }
    }
    std::process::exit(1);
}

/// Quote one argument so `CommandLineToArgvW`/the MSVC runtime parse it
/// back unchanged (backslashes are only special before a quote).
fn quote(value: &str) -> String {
    if !value.is_empty() && !value.contains([' ', '\t', '\n', '\x0B', '"']) {
        return value.to_string();
    }
    let mut out = String::from('"');
    let mut backslashes = 0;
    for c in value.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

fn main() {
    // `args_os` keeps non-UTF-8 paths intact; arguments are UTF-16 anyway.
    let args: Vec<String> = std::env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if args.len() < 3 {
        eprintln!("usage: retex-inject <hook.dll> <target.exe> [args...]");
        std::process::exit(2);
    }

    let hook = std::path::absolute(&args[1]).unwrap_or_else(|_| args[1].clone().into());
    let target = std::path::absolute(&args[2]).unwrap_or_else(|_| args[2].clone().into());
    if !hook.is_file() {
        error(&format!("hook not found: {}", hook.display()));
    }
    if !target.is_file() {
        error(&format!("target not found: {}", target.display()));
    }

    // Load the hook locally to resolve the RVA of its Initialize export.
    let hook_w = wide(hook.as_os_str());
    let local_base = unsafe { LoadLibraryW(hook_w.as_ptr()) };
    if local_base.is_null() {
        error("cannot load the hook locally");
    }
    let init_local = unsafe { GetProcAddress(local_base, b"Initialize\0".as_ptr()) };
    if init_local.is_null() {
        error("hook has no Initialize export");
    }
    let init_rva = init_local as usize - local_base as usize;

    // Command line: target plus any forwarded arguments.
    let mut command_line = quote(&target.to_string_lossy());
    for extra in &args[3..] {
        command_line.push(' ');
        command_line.push_str(&quote(extra));
    }
    let mut command_line_w = wide(OsStr::new(&command_line));

    let mut startup: StartupInfoW = unsafe { mem::zeroed() };
    startup.cb = mem::size_of::<StartupInfoW>() as u32;
    let mut process: ProcessInformation = unsafe { mem::zeroed() };

    // Bind wide strings to locals: a temporary inside an `if` branch would be
    // freed before CreateProcessW reads it.
    let target_w = wide(target.as_os_str());
    let working_dir_w = target.parent().map(|dir| wide(dir.as_os_str()));
    let current_directory = match &working_dir_w {
        Some(dir) if dir.len() > 1 => dir.as_ptr(),
        _ => null(),
    };

    let created = unsafe {
        CreateProcessW(
            target_w.as_ptr(),
            command_line_w.as_mut_ptr(),
            null_mut(),
            null_mut(),
            0,
            CREATE_SUSPENDED,
            null_mut(),
            current_directory,
            &mut startup,
            &mut process,
        )
    };
    if created == 0 {
        error("CreateProcess failed");
    }

    let hook_path_size = hook_w.len() * mem::size_of::<u16>();
    let remote_path = unsafe {
        VirtualAllocEx(
            process.process,
            null_mut(),
            hook_path_size,
            MEM_COMMIT_RESERVE,
            PAGE_READWRITE,
        )
    };
    if remote_path.is_null() {
        inject_failed(&process, "VirtualAllocEx failed");
    }
    let written = unsafe {
        WriteProcessMemory(
            process.process,
            remote_path,
            hook_w.as_ptr() as *const c_void,
            hook_path_size,
            null_mut(),
        )
    };
    if written == 0 {
        inject_failed(&process, "WriteProcessMemory failed");
    }

    let kernel32 = unsafe { GetModuleHandleA(b"kernel32.dll\0".as_ptr()) };
    let load_library = unsafe { GetProcAddress(kernel32, b"LoadLibraryW\0".as_ptr()) };
    if load_library.is_null() {
        inject_failed(&process, "LoadLibraryW not found");
    }

    let thread = unsafe {
        CreateRemoteThread(
            process.process,
            null_mut(),
            0,
            mem::transmute::<*mut c_void, ThreadStart>(load_library),
            remote_path,
            0,
            null_mut(),
        )
    };
    if thread.is_null() {
        inject_failed(&process, "CreateRemoteThread(LoadLibraryW) failed");
    }
    unsafe { WaitForSingleObject(thread, INFINITE) };
    let mut module: u32 = 0;
    unsafe { GetExitCodeThread(thread, &mut module) };
    unsafe { CloseHandle(thread) };
    if module == 0 {
        inject_failed(&process, "remote LoadLibrary returned NULL");
    }

    // Call Initialize in the target using the hook's module base + export RVA.
    let init_remote = (module as usize + init_rva) as *mut c_void;
    let init_thread = unsafe {
        CreateRemoteThread(
            process.process,
            null_mut(),
            0,
            mem::transmute::<*mut c_void, ThreadStart>(init_remote),
            null_mut(),
            0,
            null_mut(),
        )
    };
    if init_thread.is_null() {
        inject_failed(&process, "CreateRemoteThread(Initialize) failed");
    }
    unsafe {
        WaitForSingleObject(init_thread, INFINITE);
        CloseHandle(init_thread);
    }

    unsafe {
        ResumeThread(process.thread);
        CloseHandle(process.thread);
        CloseHandle(process.process);
    }
    println!(
        "retex-inject: injected {} into {} (pid {})",
        hook.display(),
        target.display(),
        process.process_id
    );
}

/// NUL-terminated UTF-16 for the `W` APIs.
fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
