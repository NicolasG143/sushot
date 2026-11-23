use klip_core::PlatformError;

pub(crate) fn os_error(operation: &'static str) -> impl Fn(windows::core::Error) -> PlatformError {
    move |error| PlatformError::new(operation, error.to_string())
}

pub(crate) fn last_os_error(operation: &'static str) -> PlatformError {
    os_error(operation)(windows::core::Error::from_thread())
}
