//! Geometry primitives shared by the game definition and the runtime.

use serde::{Deserialize, Serialize};

/// A point in virtual (game design) pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Point) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Moves this point towards `target` by at most `max_step` pixels.
    /// Returns `true` when the target has been reached.
    pub fn step_towards(&mut self, target: Point, max_step: f32) -> bool {
        let dx = target.x - self.x;
        let dy = target.y - self.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= max_step || distance <= f32::EPSILON {
            *self = target;
            true
        } else {
            self.x += dx / distance * max_step;
            self.y += dy / distance * max_step;
            false
        }
    }
}

/// An axis aligned rectangle in virtual pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// The shape covered by a hotspot: either a rectangle or an arbitrary polygon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Shape {
    Rect(Rect),
    Polygon { polygon: Vec<Point> },
}

impl Shape {
    pub fn contains(&self, point: Point) -> bool {
        match self {
            Shape::Rect(rect) => rect.contains(point),
            Shape::Polygon { polygon } => point_in_polygon(point, polygon),
        }
    }

    /// Bounding box of the shape, used to place labels and fall back walk-to points.
    pub fn bounds(&self) -> Rect {
        match self {
            Shape::Rect(rect) => *rect,
            Shape::Polygon { polygon } => bounding_box(polygon),
        }
    }
}

/// Bounding box of a polygon. Empty polygons yield a zero sized rectangle.
pub fn bounding_box(polygon: &[Point]) -> Rect {
    if polygon.is_empty() {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for point in polygon {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Standard even-odd point in polygon test.
pub fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let intersects = (pi.y > point.y) != (pj.y > point.y)
            && point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y) + pi.x;
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Closest point to `point` lying on the segment `a`-`b`.
pub fn closest_point_on_segment(point: Point, a: Point, b: Point) -> Point {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let length_squared = abx * abx + aby * aby;
    if length_squared <= f32::EPSILON {
        return a;
    }
    let t = (((point.x - a.x) * abx + (point.y - a.y) * aby) / length_squared).clamp(0.0, 1.0);
    Point::new(a.x + abx * t, a.y + aby * t)
}

/// Closest point to `point` that lies inside (or on the border of) the polygon.
pub fn clamp_to_polygon(point: Point, polygon: &[Point]) -> Point {
    if polygon.len() < 3 || point_in_polygon(point, polygon) {
        return point;
    }
    let mut best = polygon[0];
    let mut best_distance = f32::MAX;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let candidate = closest_point_on_segment(point, a, b);
        let distance = candidate.distance_to(point);
        if distance < best_distance {
            best_distance = distance;
            best = candidate;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_points() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(rect.contains(Point::new(15.0, 15.0)));
        assert!(!rect.contains(Point::new(5.0, 15.0)));
    }

    #[test]
    fn polygon_contains_points() {
        let polygon = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        assert!(point_in_polygon(Point::new(5.0, 5.0), &polygon));
        assert!(!point_in_polygon(Point::new(15.0, 5.0), &polygon));
    }

    #[test]
    fn clamping_moves_point_inside_polygon() {
        let polygon = vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let clamped = clamp_to_polygon(Point::new(20.0, 5.0), &polygon);
        assert_eq!(clamped, Point::new(10.0, 5.0));
        let inside = clamp_to_polygon(Point::new(4.0, 5.0), &polygon);
        assert_eq!(inside, Point::new(4.0, 5.0));
    }

    #[test]
    fn stepping_towards_a_target_reaches_it() {
        let mut point = Point::new(0.0, 0.0);
        assert!(!point.step_towards(Point::new(10.0, 0.0), 4.0));
        assert_eq!(point, Point::new(4.0, 0.0));
        assert!(point.step_towards(Point::new(10.0, 0.0), 100.0));
        assert_eq!(point, Point::new(10.0, 0.0));
    }
}
