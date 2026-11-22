use std::path::PathBuf;

use klip_core::LocalTime;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Shell::{
    FOLDERID_Pictures, FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};
use windows::core::GUID;

pub(crate) fn local_time() -> LocalTime {
    let now = unsafe { GetLocalTime() };
    LocalTime {
        year: now.wYear,
        month: now.wMonth as u8,
        day: now.wDay as u8,
        hour: now.wHour as u8,
        minute: now.wMinute as u8,
        second: now.wSecond as u8,
    }
}

pub(crate) fn config_path() -> Option<PathBuf> {
    known_folder(&FOLDERID_RoamingAppData).map(|dir| dir.join("klip").join("config.toml"))
}

pub(crate) fn pictures_dir() -> Option<PathBuf> {
    known_folder(&FOLDERID_Pictures)
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    let path = match unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) } {
        Ok(path) => path,
        Err(error) => {
            log::warn!("known folder lookup failed: {error}");
            return None;
        }
    };
    let folder = unsafe { path.to_string() };
    unsafe { CoTaskMemFree(Some(path.0.cast_const().cast())) };
    match folder {
        Ok(folder) => Some(PathBuf::from(folder)),
        Err(error) => {
            log::warn!("known folder path is not valid UTF-16: {error}");
            None
        }
    }
}
