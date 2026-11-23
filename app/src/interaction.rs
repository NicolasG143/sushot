use klip_core::{Mode, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Pending,
    Confirmed(Rect),
    Cancelled,
}

/// Pointer handling of the selection overlay, in overlay-local coordinates.
pub(crate) struct Interaction {
    mode: Mode,
    cursor: (i32, i32),
    anchor: Option<(i32, i32)>,
    hovered: Option<Rect>,
}

impl Interaction {
    pub(crate) fn new(mode: Mode) -> Self {
        Self {
            mode,
            cursor: (0, 0),
            anchor: None,
            hovered: None,
        }
    }

    pub(crate) fn mode(&self) -> Mode {
        self.mode
    }

    pub(crate) fn cursor(&self) -> (i32, i32) {
        self.cursor
    }

    pub(crate) fn move_to(&mut self, position: (i32, i32)) {
        self.cursor = position;
    }

    pub(crate) fn hover(&mut self, window: Option<Rect>) {
        self.hovered = window;
    }

    pub(crate) fn press(&mut self) -> Outcome {
        match self.mode {
            Mode::Region => {
                self.anchor = Some(self.cursor);
                Outcome::Pending
            }
            Mode::Window => match self.hovered {
                Some(window) => Outcome::Confirmed(window),
                None => Outcome::Pending,
            },
            Mode::FullScreen => Outcome::Cancelled,
        }
    }

    pub(crate) fn release(&mut self) -> Outcome {
        let Some(anchor) = self.anchor.take() else {
            return Outcome::Pending;
        };
        let region = Rect::from_corners(anchor, self.cursor);
        if region.is_selection() {
            Outcome::Confirmed(region)
        } else {
            Outcome::Cancelled
        }
    }

    /// Area to highlight right now: the region being dragged or the window under the cursor.
    pub(crate) fn highlight(&self) -> Option<Rect> {
        match self.mode {
            Mode::Region => self
                .anchor
                .map(|anchor| Rect::from_corners(anchor, self.cursor)),
            Mode::Window => self.hovered,
            Mode::FullScreen => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> Rect {
        Rect {
            x: 10,
            y: 10,
            w: 100,
            h: 80,
        }
    }

    #[test]
    fn dragging_a_region_highlights_it_and_confirms_on_release() {
        let mut interaction = Interaction::new(Mode::Region);
        interaction.move_to((50, 40));
        assert_eq!(interaction.press(), Outcome::Pending);
        interaction.move_to((20, 10));

        assert_eq!(
            interaction.highlight(),
            Some(Rect {
                x: 20,
                y: 10,
                w: 30,
                h: 30
            })
        );
        assert_eq!(
            interaction.release(),
            Outcome::Confirmed(Rect {
                x: 20,
                y: 10,
                w: 30,
                h: 30
            })
        );
    }

    #[test]
    fn nothing_is_highlighted_before_the_drag_starts() {
        let mut interaction = Interaction::new(Mode::Region);
        interaction.move_to((50, 40));

        assert_eq!(interaction.highlight(), None);
    }

    #[test]
    fn a_click_without_dragging_cancels() {
        let mut interaction = Interaction::new(Mode::Region);
        interaction.move_to((50, 40));
        interaction.press();

        assert_eq!(interaction.release(), Outcome::Cancelled);
    }

    #[test]
    fn a_drag_smaller_than_two_pixels_on_a_side_cancels() {
        let mut interaction = Interaction::new(Mode::Region);
        interaction.move_to((50, 40));
        interaction.press();
        interaction.move_to((51, 90));

        assert_eq!(interaction.release(), Outcome::Cancelled);
    }

    #[test]
    fn a_release_without_a_press_changes_nothing() {
        let mut interaction = Interaction::new(Mode::Region);

        assert_eq!(interaction.release(), Outcome::Pending);
    }

    #[test]
    fn window_mode_highlights_the_hovered_window_and_confirms_on_press() {
        let mut interaction = Interaction::new(Mode::Window);
        interaction.hover(Some(window()));

        assert_eq!(interaction.highlight(), Some(window()));
        assert_eq!(interaction.press(), Outcome::Confirmed(window()));
    }

    #[test]
    fn window_mode_ignores_a_press_over_no_window() {
        let mut interaction = Interaction::new(Mode::Window);

        assert_eq!(interaction.highlight(), None);
        assert_eq!(interaction.press(), Outcome::Pending);
    }

    #[test]
    fn window_mode_ignores_the_drag_of_region_mode() {
        let mut interaction = Interaction::new(Mode::Window);
        interaction.hover(Some(window()));
        interaction.move_to((5, 5));
        interaction.press();

        assert_eq!(interaction.release(), Outcome::Pending);
    }
}
