/// A Wayland session is declared by `XDG_SESSION_TYPE`; without it, a Wayland display
/// and no X display is the next best evidence.
pub(crate) fn is_wayland(
    session_type: Option<&str>,
    wayland_display: Option<&str>,
    x_display: Option<&str>,
) -> bool {
    let present = |value: Option<&str>| value.is_some_and(|text| !text.is_empty());
    match session_type {
        Some(kind) if !kind.is_empty() => kind.eq_ignore_ascii_case("wayland"),
        _ => present(wayland_display) && !present(x_display),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_session_type_wins() {
        assert!(is_wayland(Some("wayland"), None, Some(":0")));
        assert!(is_wayland(Some("Wayland"), None, None));
        assert!(!is_wayland(Some("x11"), Some("wayland-0"), None));
    }

    #[test]
    fn without_a_declared_type_a_lone_wayland_display_decides() {
        assert!(is_wayland(None, Some("wayland-0"), None));
        assert!(!is_wayland(None, Some("wayland-0"), Some(":0")));
        assert!(!is_wayland(None, None, Some(":0")));
        assert!(!is_wayland(Some(""), None, None));
    }
}
