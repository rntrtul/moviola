use crate::geometry::line::{distance_from_point_to_edge, Line};
use gtk4::graphene::Point;

#[derive(Debug)]
pub enum EdgeType {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub enum CornerType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub point: Point,
    pub corner_type: CornerType,
}

impl Corner {
    pub fn new(point: Point, corner_type: CornerType) -> Corner {
        Corner { point, corner_type }
    }
}

#[derive(Debug)]
pub struct Rectangle {
    pub(crate) top_left: Point,
    pub(crate) top_right: Point,
    pub(crate) bottom_left: Point,
    pub(crate) bottom_right: Point,
}

// todo: move into line
fn is_point_on_line(line_a: Point, line_b: Point, point: Point) -> bool {
    let line = Line::from_points(line_a, line_b);
    let y_from_x = match line.y_at_x(point.x()) {
        Err(_e) => return false,
        Ok(y) => y,
    };

    (point.y() - y_from_x).abs() <= 0.1
}

fn is_left_of_line(line_a: Point, line_b: Point, point: Point) -> f32 {
    (line_b.x() - line_a.x()) * (point.y() - line_a.y())
        - (point.x() - line_a.x()) * (line_b.y() - line_a.y())
}

impl Rectangle {
    pub fn contains(&self, point: Point) -> bool {
        is_left_of_line(self.top_left, self.top_right, point) >= 0.0
            && is_left_of_line(self.top_right, self.bottom_right, point) >= 0.0
            && is_left_of_line(self.bottom_right, self.bottom_left, point) >= 0.0
            && is_left_of_line(self.bottom_left, self.top_left, point) >= 0.0
    }

    pub fn is_point_on_boundary(&self, point: Point) -> bool {
        is_point_on_line(self.top_left, self.top_right, point)
            || is_point_on_line(self.top_right, self.bottom_right, point)
            || is_point_on_line(self.bottom_right, self.bottom_left, point)
            || is_point_on_line(self.bottom_left, self.top_left, point)
    }

    pub fn distance_to_edge(&self, point: Point, edge: EdgeType) -> (f32, f32) {
        let (x, y) = match edge {
            EdgeType::Left => distance_from_point_to_edge(self.bottom_left, self.top_left, point),
            EdgeType::Top => distance_from_point_to_edge(self.top_left, self.top_right, point),
            EdgeType::Right => {
                distance_from_point_to_edge(self.top_right, self.bottom_right, point)
            }
            EdgeType::Bottom => {
                distance_from_point_to_edge(self.bottom_left, self.bottom_right, point)
            }
        };

        let x = if x.abs() <= 0.001 { 0.0 } else { x };
        let y = if y.abs() <= 0.001 { 0.0 } else { y };

        (x, y)
    }

    pub fn line_intersection_to_edge(
        &self,
        start: Point,
        end: Point,
        edge: EdgeType,
    ) -> Option<Point> {
        let line = Line::from_points(start, end);

        let edge_line = match edge {
            EdgeType::Left => Line::from_points(self.bottom_left, self.top_left),
            EdgeType::Top => Line::from_points(self.top_left, self.top_right),
            EdgeType::Right => Line::from_points(self.top_right, self.bottom_right),
            EdgeType::Bottom => Line::from_points(self.bottom_left, self.bottom_right),
        };

        let intersection = line.intersect_point(edge_line);

        if intersection.is_ok() {
            Some(intersection.unwrap())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_rotated_rectangle() -> Rectangle {
        Rectangle {
            top_left: Point::new(10.1726, -14.2273),
            top_right: Point::new(105.278, 16.674),
            bottom_left: Point::new(-5.278, 33.326),
            bottom_right: Point::new(89.827, 64.227),
        }
    }

    #[test]
    fn rectangle_contains_points_inside() {
        let rect = fixed_rotated_rectangle();

        assert!(rect.contains(Point::new(20.0, 20.0)));
        assert!(rect.contains(Point::new(20.0, -2.0)));
        assert!(rect.contains(Point::new(-2.0, 30.0)));

        // contains border points
        assert!(rect.contains(rect.top_left));
        assert!(rect.contains(rect.top_right));
        assert!(rect.contains(rect.bottom_left));
        assert!(rect.contains(rect.bottom_right));
    }

    #[test]
    fn rectangle_contains_points_outside() {
        let rect = fixed_rotated_rectangle();

        assert!(!rect.contains(Point::new(106.0, 16.674)));
        assert!(!rect.contains(Point::new(-6.278, 33.326)));
        assert!(!rect.contains(Point::new(89.827, 65.227)));
        assert!(!rect.contains(Point::new(10.1726, -15.1)));
    }

    #[test]
    fn rectangle_check_point_on_boundary() {
        let rect = fixed_rotated_rectangle();

        assert!(rect.is_point_on_boundary(Point::new(-5.278, 33.326)));
        assert!(rect.is_point_on_boundary(Point::new(30.0, -7.7848)));
        assert!(!rect.is_point_on_boundary(Point::new(20.0, 20.0)));
    }

    #[test]
    fn rectangle_line_edge_intersection() {
        let rect = Rectangle {
            top_left: Point::new(10.0, 10.0),
            top_right: Point::new(110.0, 10.0),
            bottom_left: Point::new(10.0, 60.0),
            bottom_right: Point::new(110.0, 60.0),
        };

        assert_eq!(
            rect.line_intersection_to_edge(
                Point::new(90.0, 20.0),
                Point::new(120.0, 0.0),
                EdgeType::Top,
            ),
            Some(Point::new(105.0, 10.0))
        );

        assert_eq!(
            rect.line_intersection_to_edge(
                Point::new(90.0, 20.0),
                Point::new(90.0, 15.0),
                EdgeType::Top,
            ),
            Some(Point::new(90.0, 10.0))
        );

        assert_eq!(
            rect.line_intersection_to_edge(
                Point::new(90.0, 20.0),
                Point::new(85.0, 20.0),
                EdgeType::Left,
            ),
            Some(Point::new(10.0, 20.0))
        );

        // parallel lines (horizontally) do not intersect
        assert_eq!(
            rect.line_intersection_to_edge(
                Point::new(90.0, 20.0),
                Point::new(85.0, 20.0),
                EdgeType::Top,
            ),
            None
        );
    }
}
