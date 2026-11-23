use sushot_core::{Action, resolve};

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

/// Auto-repeat sends the next press within the keyboard delay, at most one second.
const REPEAT_WINDOW_MS: u32 = 1_000;

/// Decides the fate of every Print Screen transition. The verdict of the first
/// press is kept until the release so that the system never sees half a keystroke.
///
/// The hook does not always receive the release (it went missing whenever the
/// overlay opened while the key was still held), so a press arriving later than any
/// auto-repeat could is treated as a new keystroke.
#[derive(Debug, Default)]
pub(crate) struct SnapshotKey {
    hold: Hold,
    last_press_ms: u32,
}

impl SnapshotKey {
    pub(crate) fn on_transition(
        &mut self,
        transition: Transition,
        modifiers: Modifiers,
        time_ms: u32,
    ) -> Verdict {
        if transition == Transition::Pressed {
            if self.hold != Hold::Idle
                && time_ms.wrapping_sub(self.last_press_ms) > REPEAT_WINDOW_MS
            {
                self.hold = Hold::Idle;
            }
            self.last_press_ms = time_ms;
        }
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
    use sushot_core::{Mode, Output};

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

        let pressed = key.on_transition(Transition::Pressed, CTRL, 0);
        let released = key.on_transition(Transition::Released, CTRL, 0);

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
            key.on_transition(Transition::Pressed, CTRL_SHIFT, 0),
            Verdict::PassThrough
        );
        assert_eq!(
            key.on_transition(Transition::Released, CTRL_SHIFT, 0),
            Verdict::PassThrough
        );
    }

    #[test]
    fn auto_repeat_is_swallowed_without_triggering_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE, 0);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE, 0),
            Verdict::Swallow
        );
    }

    #[test]
    fn release_is_swallowed_even_if_the_modifiers_changed_while_held() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, CTRL, 0);

        assert_eq!(
            key.on_transition(Transition::Released, CTRL_SHIFT, 0),
            Verdict::Swallow
        );
    }

    #[test]
    fn auto_repeat_of_a_passed_through_press_never_triggers() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, CTRL_SHIFT, 0);

        assert_eq!(
            key.on_transition(Transition::Pressed, CTRL, 0),
            Verdict::PassThrough
        );
        assert_eq!(
            key.on_transition(Transition::Released, CTRL, 0),
            Verdict::PassThrough
        );
    }

    #[test]
    fn next_press_after_a_release_triggers_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE, 0);
        key.on_transition(Transition::Released, NONE, 0);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE, 0),
            Verdict::Trigger(Action {
                mode: Mode::Region,
                output: Output::Clipboard
            })
        );
    }

    #[test]
    fn a_press_long_after_a_lost_release_triggers_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE, 1_000);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE, 4_000),
            Verdict::Trigger(Action {
                mode: Mode::Region,
                output: Output::Clipboard
            })
        );
        assert_eq!(
            key.on_transition(Transition::Released, NONE, 4_100),
            Verdict::Swallow
        );
    }

    #[test]
    fn a_held_key_repeating_for_seconds_never_triggers_again() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE, 0);

        let repeats: Vec<Verdict> = (1..=100)
            .map(|step| key.on_transition(Transition::Pressed, NONE, 500 + step * 33))
            .collect();

        assert!(repeats.iter().all(|&verdict| verdict == Verdict::Swallow));
    }

    #[test]
    fn the_tick_counter_wrapping_around_does_not_look_like_a_new_press() {
        let mut key = SnapshotKey::default();
        key.on_transition(Transition::Pressed, NONE, u32::MAX - 10);

        assert_eq!(
            key.on_transition(Transition::Pressed, NONE, 20),
            Verdict::Swallow
        );
    }
}
