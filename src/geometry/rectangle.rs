use crate::geometry::line::{component_distance_from_point_to_edge, Line};
use crate::geometry::{line, rotate_point_around};
use relm4::gtk::graphene;
use relm4::gtk::graphene::Point;

#[derive(Clone, Copy, Debug)]
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

impl CornerType {
    pub fn edges(&self) -> (EdgeType, EdgeType) {
        match self {
            CornerType::TopLeft => (EdgeType::Left, EdgeType::Top),
            CornerType::TopRight => (EdgeType::Right, EdgeType::Top),
            CornerType::BottomLeft => (EdgeType::Left, EdgeType::Bottom),
            CornerType::BottomRight => (EdgeType::Right, EdgeType::Bottom),
        }
    }

    pub fn vertically_adjacent(&self) -> CornerType {
        match self {
            CornerType::TopLeft => CornerType::BottomLeft,
            CornerType::TopRight => CornerType::BottomRight,
            CornerType::BottomLeft => CornerType::TopLeft,
            CornerType::BottomRight => CornerType::TopRight,
        }
    }

    pub fn horizontally_adjacent(&self) -> CornerType {
        match self {
            CornerType::TopLeft => CornerType::TopRight,
            CornerType::TopRight => CornerType::TopLeft,
            CornerType::BottomLeft => CornerType::BottomRight,
            CornerType::BottomRight => CornerType::BottomLeft,
        }
    }
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
    pub(crate) angle: f32,
}

fn is_left_of_line(line_a: Point, line_b: Point, point: Point) -> f32 {
    (line_b.x() - line_a.x()) * (point.y() - line_a.y())
        - (point.x() - line_a.x()) * (line_b.y() - line_a.y())
}

impl Rectangle {
    pub fn new(base_rectangle: graphene::Rect, angle: f32) -> Self {
        // todo: if angle 0 skip this maybe. just rotate all 4 corners?
        let (sin, cos) = angle.to_radians().sin_cos();

        let horizontal_run = base_rectangle.width() * cos;
        let horizontal_rise = base_rectangle.width() * sin;
        let vertical_run = base_rectangle.height() * sin;
        let vertical_rise = base_rectangle.height() * cos;

        let top_left =
            rotate_point_around(base_rectangle.top_left(), base_rectangle.center(), angle);

        let top_right = Point::new(
            top_left.x() + horizontal_run,
            top_left.y() + horizontal_rise,
        );

        let bottom_left = Point::new(top_left.x() - vertical_run, top_left.y() + vertical_rise);

        let bottom_right = Point::new(
            bottom_left.x() + horizontal_run,
            bottom_left.y() + horizontal_rise,
        );

        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            angle,
        }
    }

    fn vertices_for_edge(&self, edge_type: EdgeType) -> (Point, Point) {
        match edge_type {
            EdgeType::Left => (self.bottom_left, self.top_left),
            EdgeType::Top => (self.top_left, self.top_right),
            EdgeType::Right => (self.top_right, self.bottom_right),
            EdgeType::Bottom => (self.bottom_right, self.bottom_left),
        }
    }

    fn edge_line(&self, edge_type: EdgeType) -> Line {
        let (a, b) = self.vertices_for_edge(edge_type);
        Line::from_points(a, b)
    }

    fn is_point_on_edge(&self, point: Point, edge_type: EdgeType) -> bool {
        self.edge_line(edge_type).is_point_on_line(point)
    }

    pub fn contains(&self, point: Point) -> bool {
        is_left_of_line(self.top_left, self.top_right, point) >= 0.0
            && is_left_of_line(self.top_right, self.bottom_right, point) >= 0.0
            && is_left_of_line(self.bottom_right, self.bottom_left, point) >= 0.0
            && is_left_of_line(self.bottom_left, self.top_left, point) >= 0.0
    }

    pub fn is_point_on_boundary(&self, point: Point) -> bool {
        self.is_point_on_edge(point, EdgeType::Left)
            || self.is_point_on_edge(point, EdgeType::Right)
            || self.is_point_on_edge(point, EdgeType::Bottom)
            || self.is_point_on_edge(point, EdgeType::Top)
    }

    pub fn distance_to_edge(&self, point: Point, edge: EdgeType) -> (f32, f32) {
        let (vertex_a, vertex_b) = self.vertices_for_edge(edge);
        let (x, y) = component_distance_from_point_to_edge(vertex_a, vertex_b, point);

        let x = if x.abs() <= 0.001 { 0.0 } else { x };
        let y = if y.abs() <= 0.001 { 0.0 } else { y };

        (x, y)
    }

    pub fn closest_point_on_edge(&self, edge: EdgeType, point: Point) -> Point {
        let (start, end) = self.vertices_for_edge(edge);
        let edge_line = self.edge_line(edge);

        // guranteed to find a point on the line.
        let closest = edge_line.closest_point(point).unwrap();
        line::clamp_point_to_line_segment(start, end, closest)
    }

    pub fn line_intersection_to_edge(
        &self,
        start: Point,
        end: Point,
        edge: EdgeType,
    ) -> Option<Point> {
        let line = Line::from_points(start, end);
        let edge_line = self.edge_line(edge);

        match line.intersect_point(edge_line) {
            Ok(point) => Some(point),
            Err(_) => None,
        }
    }

    pub fn corner_constraining_edge(&self, corner: CornerType) -> EdgeType {
        if self.angle <= 0.0 {
            match corner {
                CornerType::TopLeft => EdgeType::Top,
                CornerType::TopRight => EdgeType::Right,
                CornerType::BottomLeft => EdgeType::Left,
                CornerType::BottomRight => EdgeType::Bottom,
            }
        } else {
            match corner {
                CornerType::TopLeft => EdgeType::Left,
                CornerType::TopRight => EdgeType::Top,
                CornerType::BottomLeft => EdgeType::Bottom,
                CornerType::BottomRight => EdgeType::Right,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_contains_points_inside() {
        let rect = Rectangle::new(graphene::Rect::new(0.0, 0.0, 100.0, 50.0), 22.0);

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
        let rect = Rectangle::new(graphene::Rect::new(0.0, -10.0, 100.0, 50.0), 35.0);

        assert!(!rect.contains(Point::new(106.0, 16.674)));
        assert!(!rect.contains(Point::new(-6.278, 33.326)));
        assert!(!rect.contains(Point::new(89.827, 65.227)));
        assert!(!rect.contains(Point::new(10.1726, -25.1)));
    }

    #[test]
    fn rectangle_check_point_on_boundary() {
        let rect = Rectangle::new(graphene::Rect::new(5.0, 0.0, 60.0, 50.0), 32.0);

        assert!(rect.is_point_on_boundary(Point::new(42.186, 0.0))); // bottom edge
        assert!(rect.is_point_on_boundary(Point::new(60.0, 41.6043))); // right edge
        assert!(rect.is_point_on_boundary(Point::new(10.0, 8.3963))); // left edge
        assert!(rect.is_point_on_boundary(Point::new(20.0, 45.10643))); // top edge
    }

    #[test]
    fn rectangle_line_edge_intersection() {
        let rect = Rectangle::new(graphene::Rect::new(10.0, 10.0, 100.0, 50.0), 0.0);

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
