/// Area in physical pixels. On the virtual desktop the origin can be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right <= i64::from(left) || bottom <= i64::from(top) {
            return None;
        }

        Some(Rect {
            x: left,
            y: top,
            w: (right - i64::from(left)) as u32,
            h: (bottom - i64::from(top)) as u32,
        })
    }

    fn right(self) -> i64 {
        i64::from(self.x) + i64::from(self.w)
    }

    fn bottom(self) -> i64 {
        i64::from(self.y) + i64::from(self.h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_of_overlapping_rects_is_the_shared_area() {
        let left = Rect {
            x: -10,
            y: -10,
            w: 15,
            h: 15,
        };
        let right = Rect {
            x: 0,
            y: -5,
            w: 20,
            h: 5,
        };

        assert_eq!(
            left.intersection(right),
            Some(Rect {
                x: 0,
                y: -5,
                w: 5,
                h: 5
            })
        );
    }

    #[test]
    fn intersection_of_touching_rects_is_none() {
        let left = Rect {
            x: -10,
            y: 0,
            w: 10,
            h: 10,
        };
        let right = Rect {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        };

        assert_eq!(left.intersection(right), None);
    }
}
