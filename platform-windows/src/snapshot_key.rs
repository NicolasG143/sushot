use klip_core::{Action, resolve};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Transition {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Modifiers {
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    PassThrough,
    Swallow,
    Trigger(Action),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Hold {
    #[default]
    Idle,
    Swallowed,
    PassedThrough,
}

/// Decides the fate of every Print Screen transition. The verdict of the first
/// press is kept until the release so that the system never sees half a keystroke.
#[derive(Debug, Default)]
pub(crate) struct SnapshotKey {
    hold: Hold,
}

impl SnapshotKey {
    pub(crate) fn on_transition(
        &mut self,
        transition: Transition,
        modifiers: Modifiers,
    ) -> Verdict {
        match (transition, self.hold) {
            (Transition::Pressed, Hold::Idle) => {
                match resolve(modifiers.ctrl, modifiers.shift, modifiers.alt) {
                    Some(action) => {
                        self.hold = Hold::Swallowed;
                        Verdict::Trigger(action)
                    }
                    None => {
                        self.hold = Hold::PassedThrough;
                        Verdict::PassThrough
                    }
                }
            }
            (Transition::Pressed, Hold::Swallowed) => Verdict::Swallow,
            (Transition::Pressed, Hold::PassedThrough) => Verdict::PassThrough,
            (Transition::Released, hold) => {
                self.hold = Hold::Idle;
                if hold == Hold::Swallowed {
                    Verdict::Swallow
                } else {
                    Verdict::PassThrough
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use klip_core::{Mode, Output};

    use super::*;

    const NONE: Modifiers = Modifiers {
        ctrl: false,
        shift: false,
        alt: false,
    };
    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        shift: false,
        alt: false,
    };
    const CTRL_SHIFT: Modifiers = Modifiers {
        ctrl: true,
        shift: true,
        alt: false,
    };

    #[test]
    fn valid_combination_triggers_on_press_and_swallows_the_release() {
        let mut key = SnapshotKey::default();

        let pressed = key.on_transition(Transition::Pressed, CTRL);
        let released = key.on_transition(Transition::Released, CTRL);

        assert_eq!(
            pressed,
            Verdict::Trigger(Action {
                mode: Mode::FullScreen,
                output: Output::Clipboard
            })
        );
        assert_eq!(released, Verdict::Swallow);
    }

    #[test]
    fn undefined_combination_passes_through_on_press_and_release() {
        let mut key = SnapshotKey::default();

        assert_eq!(
            key.on_transition(Transition::Pressed, CTRL_SHIFT),
            Verdict::PassThrough
        );
        assert_eq!(
            key.on_transition(Transition::Released, CTRL_SHIFT),
            Verdict::PassThrough
        );
    }

    #[test]
    fn auto_repeat_is_swallowed_without_triggering_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE),
            Verdict::Swallow
        );
    }

    #[test]
    fn release_is_swallowed_even_if_the_modifiers_changed_while_held() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, CTRL);

        assert_eq!(
            key.on_transition(Transition::Released, CTRL_SHIFT),
            Verdict::Swallow
        );
    }

    #[test]
    fn auto_repeat_of_a_passed_through_press_never_triggers() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, CTRL_SHIFT);

        assert_eq!(
            key.on_transition(Transition::Pressed, CTRL),
            Verdict::PassThrough
        );
        assert_eq!(
            key.on_transition(Transition::Released, CTRL),
            Verdict::PassThrough
        );
    }

    #[test]
    fn next_press_after_a_release_triggers_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE);
        key.on_transition(Transition::Released, NONE);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE),
            Verdict::Trigger(Action {
                mode: Mode::Region,
                output: Output::Clipboard
            })
        );
    }
}
