#![cfg(windows)]

use sushot_core::Platform;
use sushot_platform_windows::WindowsPlatform;
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{
    GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetGuiResources, GetProcessHandleCount,
};

const CAPTURES: usize = 1000;
const WARM_UP: usize = 20;
const MEMORY_GROWTH_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
struct Usage {
    gdi_objects: u32,
    user_objects: u32,
    handles: u32,
    private_bytes: usize,
}

fn usage() -> Usage {
    let process = unsafe { GetCurrentProcess() };
    let mut handles = 0;
    unsafe { GetProcessHandleCount(process, &mut handles) }.expect("handle count is readable");
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    unsafe {
        GetProcessMemoryInfo(
            process,
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
            counters.cb,
        )
    }
    .expect("memory counters are readable");
    Usage {
        gdi_objects: unsafe { GetGuiResources(process, GR_GDIOBJECTS) },
        user_objects: unsafe { GetGuiResources(process, GR_USEROBJECTS) },
        handles,
        private_bytes: counters.PrivateUsage,
    }
}

fn capture_to_clipboard() {
    let frame = WindowsPlatform
        .grab_virtual_desktop()
        .expect("the desktop is captured");
    WindowsPlatform
        .copy_image(&frame)
        .expect("the image reaches the clipboard");
}

#[test]
#[ignore = "takes about a minute and replaces the clipboard contents"]
fn a_thousand_captures_leak_no_handles_and_no_memory() {
    for _ in 0..WARM_UP {
        capture_to_clipboard();
    }
    let before = usage();

    for _ in 0..CAPTURES {
        capture_to_clipboard();
    }
    let after = usage();

    eprintln!("before: {before:?}\nafter:  {after:?}");
    assert_eq!(after.gdi_objects, before.gdi_objects, "GDI objects leaked");
    assert_eq!(
        after.user_objects, before.user_objects,
        "USER objects leaked"
    );
    assert!(
        after.handles <= before.handles + 2,
        "kernel handles leaked: {} -> {}",
        before.handles,
        after.handles
    );
    assert!(
        after.private_bytes <= before.private_bytes + MEMORY_GROWTH_LIMIT,
        "private memory grew from {} to {} bytes",
        before.private_bytes,
        after.private_bytes
    );
}
