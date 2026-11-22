#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Region,
    FullScreen,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Clipboard,
    SaveDialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub mode: Mode,
    pub output: Output,
}

pub fn resolve(ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
    let mode = match (ctrl, shift) {
        (false, false) => Mode::Region,
        (true, false) => Mode::FullScreen,
        (false, true) => Mode::Window,
        (true, true) => return None,
    };
    let output = if alt {
        Output::SaveDialog
    } else {
        Output::Clipboard
    };
    Some(Action { mode, output })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(mode: Mode, output: Output) -> Option<Action> {
        Some(Action { mode, output })
    }

    #[test]
    fn resolve_maps_every_modifier_combination() {
        let cases = [
            (
                (false, false, false),
                action(Mode::Region, Output::Clipboard),
            ),
            (
                (true, false, false),
                action(Mode::FullScreen, Output::Clipboard),
            ),
            (
                (false, true, false),
                action(Mode::Window, Output::Clipboard),
            ),
            (
                (false, false, true),
                action(Mode::Region, Output::SaveDialog),
            ),
            (
                (true, false, true),
                action(Mode::FullScreen, Output::SaveDialog),
            ),
            (
                (false, true, true),
                action(Mode::Window, Output::SaveDialog),
            ),
            ((true, true, false), None),
            ((true, true, true), None),
        ];

        for ((ctrl, shift, alt), expected) in cases {
            assert_eq!(
                resolve(ctrl, shift, alt),
                expected,
                "ctrl={ctrl} shift={shift} alt={alt}"
            );
        }
    }
}
