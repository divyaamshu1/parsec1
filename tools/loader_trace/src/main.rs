use std::env;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::um::debugapi::{ContinueDebugEvent, WaitForDebugEvent};
use winapi::um::minwinbase::{DEBUG_EVENT, LOAD_DLL_DEBUG_EVENT, EXIT_PROCESS_DEBUG_EVENT, CREATE_PROCESS_DEBUG_EVENT, UNLOAD_DLL_DEBUG_EVENT, EXCEPTION_DEBUG_EVENT};
use winapi::um::processthreadsapi::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};
use winapi::um::winbase::{DEBUG_PROCESS, INFINITE};
use winapi::um::fileapi::GetFinalPathNameByHandleW;
use winapi::um::handleapi::CloseHandle;
use winapi::um::tlhelp32::{CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32};
use winapi::um::processthreadsapi::GetProcessId;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let target = if args.len() > 1 { &args[1] } else { "target/debug/parsec-cli.exe" };

    let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
    si.cb = std::mem::size_of::<STARTUPINFOW>() as DWORD;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let mut cmdline = to_wide(target);

    let res = unsafe {
        CreateProcessW(
            null_mut(),
            cmdline.as_mut_ptr(),
            null_mut(),
            null_mut(),
            FALSE,
            DEBUG_PROCESS,
            null_mut(),
            null_mut(),
            &mut si,
            &mut pi,
        )
    };

    if res == 0 {
        eprintln!("CreateProcessW failed: {}", std::io::Error::last_os_error());
        return;
    }

    println!("Launched {}; PID={}", target, unsafe { pi.dwProcessId });

    // keep process handle for module mapping
    let h_process = pi.hProcess;

    loop {
        let mut dbg: DEBUG_EVENT = unsafe { std::mem::zeroed() };
        let ok = unsafe { WaitForDebugEvent(&mut dbg, INFINITE) };
        if ok == 0 {
            eprintln!("WaitForDebugEvent failed");
            break;
        }
        match dbg.dwDebugEventCode {
            CREATE_PROCESS_DEBUG_EVENT => {
                println!("CREATE_PROCESS_DEBUG_EVENT: pid={} tid={}", dbg.dwProcessId, dbg.dwThreadId);
            }
            LOAD_DLL_DEBUG_EVENT => {
                unsafe {
                    let info = dbg.u.LoadDll();
                    let h = info.hFile;
                    if !h.is_null() {
                        let mut buf: Vec<u16> = vec![0u16; 1024];
                        let ret = GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), buf.len() as u32, 0);
                        if ret > 0 {
                            let s = String::from_utf16_lossy(&buf[..ret as usize]);
                            println!("LOAD_DLL: {}", s);
                        } else {
                            println!("LOAD_DLL: <handle but no path>");
                        }
                        CloseHandle(h);
                    } else {
                        println!("LOAD_DLL: <no handle>");
                    }
                }
            }
            UNLOAD_DLL_DEBUG_EVENT => {
                println!("UNLOAD_DLL_DEBUG_EVENT: pid={} tid={}", dbg.dwProcessId, dbg.dwThreadId);
            }
            EXCEPTION_DEBUG_EVENT => {
                unsafe {
                    let info = dbg.u.Exception();
                    let code = (*info).ExceptionRecord.ExceptionCode;
                    let addr = (*info).ExceptionRecord.ExceptionAddress as usize;
                    println!("EXCEPTION_DEBUG_EVENT: code=0x{:X} addr=0x{:X}", code, addr);

                    // map address to module via Toolhelp32
                    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, GetProcessId(h_process));
                    if snap != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                        let mut me: MODULEENTRY32W = std::mem::zeroed();
                        me.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;
                        if Module32FirstW(snap, &mut me) != 0 {
                            loop {
                                let base = me.modBaseAddr as usize;
                                let size = me.modBaseSize as usize;
                                if addr >= base && addr < base + size {
                                    // convert path
                                    let len = (0..me.szExePath.len()).take_while(|&i| me.szExePath[i] != 0).count();
                                    let slice = &me.szExePath[..len];
                                    let path = String::from_utf16_lossy(slice);
                                    println!("  -> faulting module: {} (+0x{:X})", path, addr - base);
                                    break;
                                }
                                if Module32NextW(snap, &mut me) == 0 { break; }
                            }
                        }
                        CloseHandle(snap as _);
                    }
                }
            }
            EXIT_PROCESS_DEBUG_EVENT => {
                println!("EXIT_PROCESS_DEBUG_EVENT: pid={} tid={}", dbg.dwProcessId, dbg.dwThreadId);
                unsafe { ContinueDebugEvent(dbg.dwProcessId, dbg.dwThreadId, 0x00010002) }; // DBG_CONTINUE
                break;
            }
            _ => {
                println!("Debug event: code={} pid={} tid={}", dbg.dwDebugEventCode, dbg.dwProcessId, dbg.dwThreadId);
            }
        }

        unsafe { ContinueDebugEvent(dbg.dwProcessId, dbg.dwThreadId, 0x00010002) }; // DBG_CONTINUE
    }

    unsafe {
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }
}
