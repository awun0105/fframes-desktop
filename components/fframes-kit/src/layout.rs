//! Placing things relative to each other: [`Anchor`], the nine points of a box, also used as
//! a direction (Manim's `UP`, `UR`, `DL`, ...).

/// Where `(x, y)` is on a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// The anchor point of a `width` x `height` box whose top left is at the origin.
    pub fn point(self, width: f64, height: f64) -> (f64, f64) {
        let (dx, dy) = self.direction();
        (width * (dx + 1.0) / 2.0, height * (dy + 1.0) / 2.0)
    }

    /// The direction from the center to this point of a box on screen (`y` down), each
    /// component -1, 0 or 1: `TopRight` is Manim's `UR`, `(1, -1)`.
    pub fn direction(self) -> (f64, f64) {
        use Anchor::*;
        let x = match self {
            TopLeft | Left | BottomLeft => -1.0,
            Top | Center | Bottom => 0.0,
            TopRight | Right | BottomRight => 1.0,
        };
        let y = match self {
            TopLeft | Top | TopRight => -1.0,
            Left | Center | Right => 0.0,
            BottomLeft | Bottom | BottomRight => 1.0,
        };
        (x, y)
    }

    /// The point on the opposite side: the side of a label that faces what it labels.
    pub fn opposite(self) -> Self {
        use Anchor::*;
        match self {
            TopLeft => BottomRight,
            Top => Bottom,
            TopRight => BottomLeft,
            Left => Right,
            Center => Center,
            Right => Left,
            BottomLeft => TopRight,
            Bottom => Top,
            BottomRight => TopLeft,
        }
    }

    /// The top left of a `width` x `height` box placed next to `(x, y)` in this direction,
    /// `buff` pixels away (Manim's `next_to(point, direction, buff)`).
    pub fn next_to(self, x: f64, y: f64, width: f64, height: f64, buff: f64) -> (f64, f64) {
        let (dx, dy) = self.direction();
        let (ax, ay) = self.opposite().point(width, height);
        (x + dx * buff - ax, y + dy * buff - ay)
    }
}

#[cfg(test)]
mod tests {
    use super::Anchor;

    #[test]
    fn points_and_next_to() {
        assert_eq!(Anchor::TopLeft.point(10.0, 4.0), (0.0, 0.0));
        assert_eq!(Anchor::Center.point(10.0, 4.0), (5.0, 2.0));
        assert_eq!(Anchor::BottomRight.point(10.0, 4.0), (10.0, 4.0));
        // above the point: the box's bottom center is `buff` above it
        assert_eq!(
            Anchor::Top.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (95.0, 94.0)
        );
        // up right: the box's bottom left corner is diagonally away
        assert_eq!(
            Anchor::TopRight.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (102.0, 94.0)
        );
        assert_eq!(
            Anchor::Center.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (95.0, 98.0)
        );
    }
}
