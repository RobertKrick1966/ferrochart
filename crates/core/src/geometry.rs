/// A 2D point in pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// An axis-aligned rectangle in pixel coordinates (origin + size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[must_use]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    #[must_use]
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_dimensions() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!((r.right() - 110.0).abs() < f64::EPSILON);
        assert!((r.bottom() - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_center() {
        let r = Rect::new(0.0, 0.0, 200.0, 100.0);
        let c = r.center();
        assert!((c.x - 100.0).abs() < f64::EPSILON);
        assert!((c.y - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_center_with_offset() {
        let r = Rect::new(10.0, 20.0, 100.0, 60.0);
        let c = r.center();
        assert!((c.x - 60.0).abs() < f64::EPSILON);
        assert!((c.y - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rect_contains_interior() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Point { x: 50.0, y: 50.0 }));
    }

    #[test]
    fn rect_contains_edges() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Point { x: 0.0, y: 0.0 }));
        assert!(r.contains(Point { x: 100.0, y: 100.0 }));
        assert!(r.contains(Point { x: 0.0, y: 100.0 }));
        assert!(r.contains(Point { x: 100.0, y: 0.0 }));
    }

    #[test]
    fn rect_does_not_contain_exterior() {
        let r = Rect::new(10.0, 10.0, 80.0, 80.0);
        assert!(!r.contains(Point { x: 5.0, y: 50.0 }));
        assert!(!r.contains(Point { x: 50.0, y: 5.0 }));
        assert!(!r.contains(Point { x: 95.0, y: 50.0 }));
        assert!(!r.contains(Point { x: 50.0, y: 95.0 }));
    }

    #[test]
    fn rect_zero_size() {
        let r = Rect::new(5.0, 5.0, 0.0, 0.0);
        assert!((r.right() - 5.0).abs() < f64::EPSILON);
        assert!((r.bottom() - 5.0).abs() < f64::EPSILON);
        assert!(r.contains(Point { x: 5.0, y: 5.0 }));
        assert!(!r.contains(Point { x: 5.1, y: 5.0 }));
    }
}
