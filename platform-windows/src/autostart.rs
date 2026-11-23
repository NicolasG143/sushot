use std::env;

use klip_core::PlatformError;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegSetValueExW,
};
use windows::core::PCWSTR;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "klip";
const OPERATION: &str = "update autostart entry";

pub(crate) fn set(enabled: bool) -> Result<(), PlatformError> {
    let command = if enabled {
        let exe =
            env::current_exe().map_err(|error| PlatformError::new(OPERATION, error.to_string()))?;
        Some(format!("\"{}\"", exe.display()))
    } else {
        None
    };
    write(RUN_KEY, VALUE_NAME, command.as_deref())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// Stores `command` under `name`, or removes the value when `command` is `None`.
fn write(subkey: &str, name: &str, command: Option<&str>) -> Result<(), PlatformError> {
    let subkey = wide(subkey);
    let name = wide(name);
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    }
    .ok()
    .map_err(|error| PlatformError::new(OPERATION, error.to_string()))?;

    let status = match command {
        Some(command) => {
            let data: Vec<u8> = wide(command)
                .iter()
                .flat_map(|unit| unit.to_le_bytes())
                .collect();
            unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(&data)) }
        }
        None => match unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) } {
            ERROR_FILE_NOT_FOUND => Default::default(),
            other => other,
        },
    };
    if let Err(error) = unsafe { RegCloseKey(key) }.ok() {
        log::warn!("registry key was not closed: {error}");
    }
    status
        .ok()
        .map_err(|error| PlatformError::new(OPERATION, error.to_string()))
}

#[cfg(test)]
mod tests {
    use windows::Win32::System::Registry::{RRF_RT_REG_SZ, RegDeleteKeyW, RegGetValueW};

    use super::*;

    struct ScratchKey(String);

    impl ScratchKey {
        fn new(name: &str) -> Self {
            Self(format!("Software\\klip-test-{name}-{}", std::process::id()))
        }
    }

    impl Drop for ScratchKey {
        fn drop(&mut self) {
            let subkey = wide(&self.0);
            let _ = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr())) };
        }
    }

    fn read(subkey: &str, name: &str) -> Option<String> {
        let subkey = wide(subkey);
        let name = wide(name);
        let mut buffer = [0u16; 512];
        let mut size = size_of_val(&buffer) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        status.ok().ok()?;
        let units = size as usize / 2;
        Some(String::from_utf16_lossy(&buffer[..units.saturating_sub(1)]))
    }

    #[test]
    fn enabling_stores_the_command_and_disabling_removes_it() {
        let scratch = ScratchKey::new("toggle");
        let command = "\"C:\\Program Files\\klip\\klip.exe\"";

        write(&scratch.0, "klip", Some(command)).unwrap();
        assert_eq!(read(&scratch.0, "klip").as_deref(), Some(command));

        write(&scratch.0, "klip", None).unwrap();
        assert_eq!(read(&scratch.0, "klip"), None);
    }

    #[test]
    fn disabling_a_missing_value_is_not_an_error() {
        let scratch = ScratchKey::new("missing");

        write(&scratch.0, "klip", None).unwrap();
    }

    #[test]
    fn enabling_twice_keeps_the_latest_command() {
        let scratch = ScratchKey::new("overwrite");

        write(&scratch.0, "klip", Some("old")).unwrap();
        write(&scratch.0, "klip", Some("new")).unwrap();

        assert_eq!(read(&scratch.0, "klip").as_deref(), Some("new"));
    }
}
