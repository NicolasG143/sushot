use std::io::{self, Write};

use log::{LevelFilter, Log, Metadata, Record};

struct Logger;

static LOGGER: Logger = Logger;

pub(crate) fn init() {
    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "[sushot] {:<5} {}: {}\n",
            record.level(),
            record.target(),
            record.args()
        );

        // Release builds have no console, so a failed write here is the normal case
        // and the debugger channel below is the one that carries the line.
        let _ = io::stderr().write_all(line.as_bytes());
        #[cfg(windows)]
        debugger::write(&line);
    }

    fn flush(&self) {}
}

#[cfg(windows)]
mod debugger {
    use std::iter;

    use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
    use windows::core::PCWSTR;

    pub(super) fn write(line: &str) {
        let wide: Vec<u16> = line.encode_utf16().chain(iter::once(0)).collect();
        unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
    }
}
