use crate::geometry::LineError::NoUniqueIntersection;
use gtk4::graphene;
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

fn line_slope_and_intercept(a: Point, b: Point) -> (f32, f32) {
    let slope = (b.y() - a.y()) / (b.x() - a.x());
    let intercept = (-slope * a.x()) + a.y();

    (slope, intercept)
}

fn is_point_on_line(line_a: Point, line_b: Point, point: Point) -> bool {
    let (px, py) = (point.x(), point.y());
    let (m, b) = line_slope_and_intercept(line_a, line_b);

    let y_from_x = (m * px) + b;

    (py - y_from_x).abs() <= 0.1
}

fn is_left_of_line(line_a: Point, line_b: Point, point: Point) -> f32 {
    (line_b.x() - line_a.x()) * (point.y() - line_a.y())
        - (point.x() - line_a.x()) * (line_b.y() - line_a.y())
}

impl Rectangle {
    pub fn contains(&self, point: graphene::Point) -> bool {
        is_left_of_line(self.top_left, self.top_right, point) >= 0.0
            && is_left_of_line(self.top_right, self.bottom_right, point) >= 0.0
            && is_left_of_line(self.bottom_right, self.bottom_left, point) >= 0.0
            && is_left_of_line(self.bottom_left, self.top_left, point) >= 0.0
    }

    pub fn is_point_on_boundary(&self, point: graphene::Point) -> bool {
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

pub(crate) fn rotate_point(point: graphene::Point, degrees: f32) -> graphene::Point {
    let angle = degrees.to_radians();

    let rotated_x = (point.x() * angle.cos()) - (point.y() * angle.sin());
    let rotated_y = (point.y() * angle.cos()) + (point.x() * angle.sin());

    graphene::Point::new(rotated_x, rotated_y)
}

pub(crate) fn rotate_point_around(
    point: graphene::Point,
    origin: Point,
    degrees: f32,
) -> graphene::Point {
    let rotated = rotate_point(
        Point::new(point.x() - origin.x(), point.y() - origin.y()),
        degrees,
    );

    Point::new(rotated.x() + origin.x(), rotated.y() + origin.y())
}

pub(crate) fn closest_point_on_edge_from_point(
    line_a: Point,
    line_b: Point,
    point: Point,
) -> Point {
    let a = line_a.y() - line_b.y();
    let b = line_b.x() - line_a.x();
    let c = (line_a.x() * line_b.y()) - (line_b.x() * line_a.y());

    let denom = a.powi(2) + b.powi(2);

    let x = ((b * ((b * point.x()) - (a * point.y()))) - (a * c)) / denom;
    let y = ((a * ((-b * point.x()) + (a * point.y()))) - (b * c)) / denom;

    // Need to ensure the point is within line bounds
    let min_x = line_a.x().min(line_b.x());
    let max_x = line_a.x().max(line_b.x());

    let min_y = line_a.y().min(line_b.y());
    let max_y = line_a.y().max(line_b.y());

    Point::new(x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

fn num_in_unordered_range(a: f32, b: f32, num: f32) -> bool {
    if a < b {
        num >= a && num <= b
    } else {
        num >= b && num <= a
    }
}

pub(crate) fn distance_from_point_to_edge(
    line_a: Point,
    line_b: Point,
    point: Point,
) -> (f32, f32) {
    let line = Line::from_points(line_a, line_b);

    let x = line.x_at_y(point.y()).unwrap_or(f32::INFINITY);
    let y = line.y_at_x(point.x()).unwrap_or(f32::INFINITY);

    // fixme: cleanup the if statements
    let x_dist = if num_in_unordered_range(line_a.y(), line_b.y(), point.y()) {
        x - point.x()
    } else {
        if point.y() < line_a.y() {
            if line.slope > 0.0 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            }
        } else {
            if line.slope > 0.0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        }
    };
    let y_dist = if num_in_unordered_range(line_b.x(), line_a.x(), point.x()) {
        y - point.y()
    } else {
        if point.x() < line_a.x() {
            if line.slope > 0.0 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            }
        } else {
            if line.slope > 0.0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        }
    };

    (x_dist, y_dist)
}

pub(crate) fn point_distance(a: Point, b: Point) -> f32 {
    ((b.x() - a.x()).powi(2) + (b.y() - a.y()).powi(2)).sqrt()
}

pub(crate) fn bounding_point_on_edges(a: Point, b: Point, c: Point, point: Point) -> Point {
    let ab_closest = closest_point_on_edge_from_point(a, b, point);
    let ac_closest = closest_point_on_edge_from_point(a, c, point);

    if point_distance(ab_closest, point) > point_distance(ac_closest, point) {
        ac_closest
    } else {
        ab_closest
    }
}

#[derive(Debug)]
pub struct Line {
    slope: f32,
    y_intercept: f32,
    is_vertical: bool,
    x_intercept: f32,
}

#[derive(Debug)]
enum LineError {
    PointOutOfBound,
    VerticalLine,
    HorizontalLine,
    NoIntersection,
    NoUniqueIntersection,
}

impl Line {
    pub fn from_points(start: Point, end: Point) -> Self {
        let (slope, y_intercept) = line_slope_and_intercept(start, end);
        let is_vertical = start.x() == end.x();

        let x_intercept = if is_vertical {
            start.x()
        } else {
            -y_intercept / slope
        };

        Self {
            slope,
            y_intercept,
            is_vertical,
            x_intercept,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.slope == 0.0
    }

    fn y_at_x(&self, x: f32) -> Result<f32, LineError> {
        if self.is_vertical {
            return Err(LineError::VerticalLine);
        } else if self.is_horizontal() {
            return Ok(self.y_intercept);
        }

        Ok(self.slope * x + self.y_intercept)
    }

    fn x_at_y(&self, y: f32) -> Result<f32, LineError> {
        if self.is_horizontal() {
            return Err(LineError::HorizontalLine);
        } else if self.is_vertical {
            return Ok(self.x_intercept);
        }

        Ok((y - self.y_intercept) / self.slope)
    }

    fn intersect_point(&self, other: Line) -> Result<Point, LineError> {
        if self.is_vertical && other.is_vertical {
            return Err(NoUniqueIntersection);
        } else if self.is_vertical && !other.is_vertical {
            let x = self.x_intercept;
            let y = other.y_at_x(x)?;

            return Ok(Point::new(x, y));
        } else if !self.is_vertical && other.is_vertical {
            let x = other.x_intercept;
            let y = self.y_at_x(x)?;

            return Ok(Point::new(x, y));
        }

        if self.is_horizontal() && other.is_horizontal() {
            return Err(NoUniqueIntersection);
        } else if !self.is_horizontal() && other.is_horizontal() {
            let y = other.y_intercept;
            let x = self.x_at_y(y)?;

            return Ok(Point::new(x, y));
        } else if self.is_horizontal() && !other.is_horizontal() {
            let y = self.y_intercept;
            let x = other.x_at_y(y)?;

            return Ok(Point::new(x, y));
        }

        let x = (other.y_intercept - self.y_intercept) / (self.slope - other.slope);
        let y = self.y_at_x(x)?;

        Ok(Point::new(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_inside_point() {
        let rect = Rectangle {
            top_left: Point::new(0.0, 0.0),
            top_right: Point::new(105.278, 16.674),
            bottom_left: Point::new(-5.278, 33.326),
            bottom_right: Point::new(89.827, 64.227),
        };

        assert!(rect.contains(Point::new(20.0, 20.0)));
    }

    #[test]
    fn contains_border() {
        let rect = Rectangle {
            top_left: Point::new(0.0, 0.0),
            top_right: Point::new(105.278, 16.674),
            bottom_left: Point::new(-5.278, 33.326),
            bottom_right: Point::new(89.827, 64.227),
        };

        assert!(rect.contains(Point::zero()));
        assert!(rect.contains(Point::new(105.278, 16.674)));
        assert!(rect.contains(Point::new(-5.278, 33.326)));
        assert!(rect.contains(Point::new(89.827, 64.227)));
    }

    #[test]
    fn point_on_rect_border() {
        let rect = Rectangle {
            top_left: Point::new(10.1726, -14.2273),
            top_right: Point::new(105.278, 16.674),
            bottom_left: Point::new(-5.278, 33.326),
            bottom_right: Point::new(89.827, 64.227),
        };

        assert!(rect.is_point_on_boundary(Point::new(-5.278, 33.326)));
        assert!(rect.is_point_on_boundary(Point::new(30.0, -7.7848)));
        assert!(!rect.is_point_on_boundary(Point::new(20.0, 20.0)));
    }

    #[test]
    fn line_edge_intersection() {
        let rect = Rectangle {
            top_left: Point::new(10.0, 10.0),
            top_right: Point::new(110.0, 10.0),
            bottom_left: Point::new(10.0, 60.0),
            bottom_right: Point::new(110.0, 60.0),
        };

        // todo: actually assert that these are returning what i Need
        let a = rect.line_intersection_to_edge(
            Point::new(90.0, 20.0),
            Point::new(120.0, 0.0),
            EdgeType::Top,
        );
        println!("{a:?}");

        let a = rect.line_intersection_to_edge(
            Point::new(90.0, 20.0),
            Point::new(90.0, 15.0),
            EdgeType::Top,
        );
        println!("{a:?}");

        let a = rect.line_intersection_to_edge(
            Point::new(90.0, 20.0),
            Point::new(85.0, 20.0),
            EdgeType::Top,
        );
        println!("{a:?}");

        let a = rect.line_intersection_to_edge(
            Point::new(90.0, 20.0),
            Point::new(85.0, 20.0),
            EdgeType::Left,
        );
        println!("{a:?}");
    }

    #[test]
    fn misses_outside() {
        let rect = Rectangle {
            top_left: Point::new(0.0, 0.0),
            top_right: Point::new(105.278, 16.674),
            bottom_left: Point::new(-5.278, 33.326),
            bottom_right: Point::new(89.827, 64.227),
        };

        assert!(!rect.contains(Point::new(106.0, 16.674)));
        assert!(!rect.contains(Point::new(-6.278, 33.326)));
        assert!(!rect.contains(Point::new(89.827, 65.227)));
        assert!(!rect.contains(Point::new(0.0, -0.1)));
    }

    // fixme: find better way of handling float comps
    #[test]
    fn x_y_distance_to_edge() {
        let a = Point::new(1.0, 0.0);
        let b = Point::new(10.0, 10.0);
        let c = Point::new(1.0, 10.0);

        assert_eq!(
            distance_from_point_to_edge(a, b, Point::new(8.0, 0.0)),
            (-7.0, 7.777778)
        );
        assert_eq!(
            distance_from_point_to_edge(a, b, Point::new(2.0, 2.0)),
            (0.79999995, -0.88888884)
        );

        assert_eq!(
            distance_from_point_to_edge(a, b, Point::new(1.0, 0.0)),
            (0.0, 0.0)
        );

        // vertical line
        assert_eq!(
            distance_from_point_to_edge(a, c, Point::new(0.0, 0.0)),
            (1.0, f32::NEG_INFINITY)
        );

        // horizontal line
        assert_eq!(
            distance_from_point_to_edge(b, c, Point::new(1.0, 0.0)),
            (f32::INFINITY, 10.0)
        );
    }

    #[test]
    fn x_y_edge_dist_out_of_bounds_positive_slope() {
        let a = Point::new(1.0, 0.0);
        let b = Point::new(10.0, 10.0);

        let above_left = Point::new(0.0, 12.0);
        let above_right = Point::new(18.0, 11.0);
        let below_left = Point::new(-5.0, -1.2);
        let below_right = Point::new(12.0, -12.2);

        //
        assert_eq!(
            distance_from_point_to_edge(a, b, Point::new(12.0, 2.0)),
            (-9.2, f32::INFINITY)
        );
        assert_eq!(
            distance_from_point_to_edge(a, b, above_right),
            (f32::INFINITY, f32::INFINITY)
        );
        assert_eq!(
            distance_from_point_to_edge(a, b, above_left),
            (f32::INFINITY, f32::NEG_INFINITY)
        );

        assert_eq!(
            distance_from_point_to_edge(a, b, below_left),
            (f32::NEG_INFINITY, f32::NEG_INFINITY)
        );

        assert_eq!(
            distance_from_point_to_edge(a, b, below_right),
            (f32::NEG_INFINITY, f32::INFINITY)
        );
    }
}
