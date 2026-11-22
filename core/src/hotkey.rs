use std::fmt;

/// Base key of the capture shortcuts. Ctrl, Shift and Alt keep their meaning whichever
/// key is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotkeyKey {
    #[default]
    PrintScreen,
    Pause,
    ScrollLock,
    Function(u8),
}

impl HotkeyKey {
    pub fn from_name(name: &str) -> Option<HotkeyKey> {
        match name {
            "PrintScreen" => Some(HotkeyKey::PrintScreen),
            "Pause" => Some(HotkeyKey::Pause),
            "ScrollLock" => Some(HotkeyKey::ScrollLock),
            _ => {
                let number = name.strip_prefix('F')?.parse::<u8>().ok()?;
                let canonical = number.to_string() == name[1..];
                (canonical && (1..=24).contains(&number)).then_some(HotkeyKey::Function(number))
            }
        }
    }

    /// Windows virtual-key code.
    pub fn windows_vk(self) -> u16 {
        match self {
            HotkeyKey::PrintScreen => 0x2C,
            HotkeyKey::Pause => 0x13,
            HotkeyKey::ScrollLock => 0x91,
            HotkeyKey::Function(number) => 0x70 + u16::from(number) - 1,
        }
    }

    /// X11 keysym.
    pub fn x11_keysym(self) -> u32 {
        match self {
            HotkeyKey::PrintScreen => 0xff61,
            HotkeyKey::Pause => 0xff13,
            HotkeyKey::ScrollLock => 0xff14,
            HotkeyKey::Function(number) => 0xffbe + u32::from(number) - 1,
        }
    }

    /// Key name in the trigger syntax of the XDG shortcuts specification.
    pub fn portal_name(self) -> String {
        match self {
            HotkeyKey::PrintScreen => "PRINT".to_owned(),
            HotkeyKey::Pause => "PAUSE".to_owned(),
            HotkeyKey::ScrollLock => "SCROLL_LOCK".to_owned(),
            HotkeyKey::Function(number) => format!("F{number}"),
        }
    }
}

impl fmt::Display for HotkeyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotkeyKey::PrintScreen => f.write_str("PrintScreen"),
            HotkeyKey::Pause => f.write_str("Pause"),
            HotkeyKey::ScrollLock => f.write_str("ScrollLock"),
            HotkeyKey::Function(number) => write!(f, "F{number}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_print_screen() {
        assert_eq!(HotkeyKey::default(), HotkeyKey::PrintScreen);
    }

    #[test]
    fn names_round_trip_through_display() {
        let keys = [
            HotkeyKey::PrintScreen,
            HotkeyKey::Pause,
            HotkeyKey::ScrollLock,
            HotkeyKey::Function(1),
            HotkeyKey::Function(12),
            HotkeyKey::Function(24),
        ];
        for key in keys {
            assert_eq!(HotkeyKey::from_name(&key.to_string()), Some(key));
        }
    }

    #[test]
    fn unknown_or_out_of_range_names_are_rejected() {
        for name in ["", "printscreen", "F0", "F25", "F01", "F", "Enter", "F+1"] {
            assert_eq!(HotkeyKey::from_name(name), None, "{name:?}");
        }
    }

    #[test]
    fn platform_codes_follow_each_systems_numbering() {
        assert_eq!(HotkeyKey::PrintScreen.windows_vk(), 0x2C);
        assert_eq!(HotkeyKey::Function(1).windows_vk(), 0x70);
        assert_eq!(HotkeyKey::Function(24).windows_vk(), 0x87);
        assert_eq!(HotkeyKey::PrintScreen.x11_keysym(), 0xff61);
        assert_eq!(HotkeyKey::Function(1).x11_keysym(), 0xffbe);
        assert_eq!(HotkeyKey::Function(12).x11_keysym(), 0xffc9);
    }

    #[test]
    fn portal_names_use_the_shortcut_trigger_syntax() {
        assert_eq!(HotkeyKey::PrintScreen.portal_name(), "PRINT");
        assert_eq!(HotkeyKey::ScrollLock.portal_name(), "SCROLL_LOCK");
        assert_eq!(HotkeyKey::Function(9).portal_name(), "F9");
    }
}
