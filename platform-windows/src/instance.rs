use sushot_core::PlatformError;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

use crate::error::os_error;

pub(crate) const SINGLE_INSTANCE_NAME: &str = "Local\\sushot-single-instance";

pub(crate) struct NamedMutex(HANDLE);

impl Drop for NamedMutex {
    fn drop(&mut self) {
        if let Err(error) = unsafe { CloseHandle(self.0) } {
            log::warn!("instance mutex was not released: {error}");
        }
    }
}

/// `None` means another process already owns `name`.
pub(crate) fn claim(name: &str) -> Result<Option<NamedMutex>, PlatformError> {
    let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) }
        .map_err(os_error("create instance mutex"))?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        if let Err(error) = unsafe { CloseHandle(handle) } {
            log::warn!("duplicate mutex handle was not closed: {error}");
        }
        return Ok(None);
    }
    Ok(Some(NamedMutex(handle)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique(name: &str) -> String {
        format!("Local\\sushot-test-{name}-{}", std::process::id())
    }

    #[test]
    fn second_claim_of_the_same_name_is_refused_until_the_first_is_released() {
        let name = unique("claim");

        let first = claim(&name).unwrap();
        assert!(first.is_some());
        assert!(claim(&name).unwrap().is_none());

        drop(first);
        assert!(claim(&name).unwrap().is_some());
    }

    #[test]
    fn different_names_do_not_collide() {
        let _first = claim(&unique("a")).unwrap().unwrap();

        assert!(claim(&unique("b")).unwrap().is_some());
    }
}
