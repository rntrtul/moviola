use crate::geometry::point::point_distance;
use gtk4::graphene::Point;

// todo: refactor to accept Line or a Line Segment
//  (a wrapper around line which just binds between start and x)
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

// todo: move into rectangle and take edge enum as parameter
pub fn bounding_point_on_edges(a: Point, b: Point, c: Point, point: Point) -> Point {
    let ab_closest = closest_point_on_edge_from_point(a, b, point);
    let ac_closest = closest_point_on_edge_from_point(a, c, point);

    if point_distance(ab_closest, point) > point_distance(ac_closest, point) {
        ac_closest
    } else {
        ab_closest
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub is_vertical: bool,
    pub is_horizontal: bool,
    pub slope: f32,
    pub y_intercept: Option<f32>,
    pub x_intercept: Option<f32>,
}

#[derive(Debug, PartialEq)]
pub enum LineError {
    VerticalLine,
    HorizontalLine,
    NoIntersection,
    IdenticalLine,
}

fn line_slope_and_intercept(a: Point, b: Point) -> (f32, f32) {
    let slope = (b.y() - a.y()) / (b.x() - a.x());
    let intercept = (-slope * a.x()) + a.y();

    (slope, intercept)
}

impl Line {
    pub fn from_points(start: Point, end: Point) -> Self {
        let (slope, y_intercept) = line_slope_and_intercept(start, end);

        let is_vertical = start.x() == end.x();
        let is_horizontal = start.y() == end.y();

        let x_intercept = if is_vertical {
            Some(start.x())
        } else if is_horizontal {
            None
        } else {
            Some(-y_intercept / slope)
        };

        let y_intercept = if is_vertical {
            None
        } else if is_horizontal {
            Some(start.y())
        } else {
            Some(y_intercept)
        };

        Self {
            is_vertical,
            is_horizontal,
            slope,
            y_intercept,
            x_intercept,
        }
    }

    pub fn y_at_x(&self, x: f32) -> Result<f32, LineError> {
        if self.is_vertical {
            return Err(LineError::VerticalLine);
        } else if self.is_horizontal {
            return Ok(self.y_intercept.unwrap());
        }

        Ok(self.slope * x + self.y_intercept.unwrap())
    }

    pub fn x_at_y(&self, y: f32) -> Result<f32, LineError> {
        if self.is_horizontal {
            return Err(LineError::HorizontalLine);
        } else if self.is_vertical {
            return Ok(self.x_intercept.unwrap());
        }

        Ok((y - self.y_intercept.unwrap()) / self.slope)
    }

    pub fn intersect_point(&self, other: Line) -> Result<Point, LineError> {
        if self.slope == other.slope {
            if let (Some(self_y), Some(other_y)) = (self.y_intercept, other.y_intercept) {
                if self_y == other_y {
                    return Err(LineError::IdenticalLine);
                }
            }
            if let (Some(self_x), Some(other_x)) = (self.x_intercept, other.x_intercept) {
                if self_x == other_x {
                    return Err(LineError::IdenticalLine);
                }
            }

            return Err(LineError::NoIntersection);
        }

        if self.is_vertical && other.is_vertical {
            return Err(LineError::NoIntersection);
        } else if self.is_vertical && !other.is_vertical {
            let x = self.x_intercept.unwrap();
            let y = other.y_at_x(x)?;

            return Ok(Point::new(x, y));
        } else if !self.is_vertical && other.is_vertical {
            let x = other.x_intercept.unwrap();
            let y = self.y_at_x(x)?;

            return Ok(Point::new(x, y));
        }

        if self.is_horizontal && other.is_horizontal {
            return Err(LineError::NoIntersection);
        } else if !self.is_horizontal && other.is_horizontal {
            let y = other.y_intercept.unwrap();
            let x = self.x_at_y(y)?;

            return Ok(Point::new(x, y));
        } else if self.is_horizontal && !other.is_horizontal {
            let y = self.y_intercept.unwrap();
            let x = other.x_at_y(y)?;

            return Ok(Point::new(x, y));
        }

        let x =
            (other.y_intercept.unwrap() - self.y_intercept.unwrap()) / (self.slope - other.slope);
        let y = self.y_at_x(x)?;

        Ok(Point::new(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points_relatively_same(a: Point, b: Point, threshold: f32) -> bool {
        a.x().abs() - b.x().abs() < threshold && a.y().abs() - b.y().abs() < threshold
    }

    fn tuple_relatively_same(a: (f32, f32), b: (f32, f32), threshold: f32) -> bool {
        a.0.abs() - b.0.abs() < threshold && a.1.abs() - b.1.abs() < threshold
    }

    #[test]
    fn x_y_distance_to_edge() {
        let a = Point::new(1.0, 0.0);
        let b = Point::new(10.0, 10.0);
        let c = Point::new(1.0, 10.0);

        assert!(tuple_relatively_same(
            distance_from_point_to_edge(a, b, Point::new(8.0, 0.0)),
            (-7.0, 7.777),
            0.1
        ));
        assert!(tuple_relatively_same(
            distance_from_point_to_edge(a, b, Point::new(2.0, 2.0)),
            (0.8, -0.88),
            0.1
        ));

        assert!(tuple_relatively_same(
            distance_from_point_to_edge(a, b, Point::new(1.0, 0.0)),
            (0.0, 0.0),
            0.1
        ));

        // vertical line
        let (dist_x, dist_y) = distance_from_point_to_edge(a, c, Point::new(0.0, 0.0));
        assert!(dist_x - 1.0 < 0.1);
        assert_eq!(dist_y, f32::NEG_INFINITY);

        let (dist_x, dist_y) = distance_from_point_to_edge(b, c, Point::new(1.0, 0.0));
        assert_eq!(dist_x, f32::INFINITY);
        assert!(dist_y - 10.0 < 0.1);
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

    #[test]
    fn line_build_from_points() {
        let a = Point::new(0.0, 1.0);
        let b = Point::new(10.0, 11.0);
        let c = Point::new(0.0, 11.0);

        let sloped = Line::from_points(a, b);

        assert_eq!(sloped.is_vertical, false);
        assert_eq!(sloped.is_horizontal, false);
        assert_eq!(sloped.slope, 1.0);
        assert_eq!(sloped.y_intercept, Some(1.0));
        assert_eq!(sloped.x_intercept, Some(-1.0));

        let vertical = Line::from_points(a, c);

        assert_eq!(vertical.is_vertical, true);
        assert_eq!(vertical.is_horizontal, false);
        assert_eq!(vertical.slope, f32::INFINITY);
        assert_eq!(vertical.x_intercept, Some(0.0));
        assert_eq!(vertical.y_intercept, None);

        let horizontal = Line::from_points(b, c);

        assert_eq!(horizontal.is_vertical, false);
        assert_eq!(horizontal.is_horizontal, true);
        assert_eq!(horizontal.slope, 0.0);
        assert_eq!(horizontal.y_intercept, Some(11.0));
        assert_eq!(horizontal.x_intercept, None);
    }

    #[test]
    fn line_intersect() {
        let grow_a = Line::from_points(Point::new(0.0, 0.0), Point::new(5.0, 20.0));
        let grow_b = Line::from_points(Point::new(10.0, 5.0), Point::new(15.0, 35.0));

        assert!(grow_a.intersect_point(grow_b).is_ok());
        assert!(points_relatively_same(
            grow_a.intersect_point(grow_b).unwrap(),
            Point::new(27.5, 110.0),
            0.1
        ));
        assert!(points_relatively_same(
            grow_a.intersect_point(grow_b).unwrap(),
            grow_b.intersect_point(grow_a).unwrap(),
            0.1
        ));

        let shrink_a = Line::from_points(Point::new(0.0, 20.0), Point::new(10.0, 0.0));
        assert!(grow_a.intersect_point(shrink_a).is_ok());
        assert!(points_relatively_same(
            grow_a.intersect_point(shrink_a).unwrap(),
            Point::new(3.33, 13.33),
            0.1
        ));
        assert!(points_relatively_same(
            grow_a.intersect_point(shrink_a).unwrap(),
            shrink_a.intersect_point(grow_a).unwrap(),
            0.1
        ));

        let shrink_b = Line::from_points(Point::new(0.0, 5.0), Point::new(0.71429, 0.0));
        assert!(shrink_a.intersect_point(shrink_b).is_ok());
        assert!(points_relatively_same(
            shrink_a.intersect_point(shrink_b).unwrap(),
            Point::new(-3.0, 26.0),
            0.1
        ));
        assert!(points_relatively_same(
            shrink_a.intersect_point(shrink_b).unwrap(),
            shrink_b.intersect_point(shrink_a).unwrap(),
            0.1
        ));

        let vertical = Line::from_points(Point::new(-5.0, 0.0), Point::new(-5.0, 10.0));
        assert!(grow_b.intersect_point(vertical).is_ok());
        assert!(points_relatively_same(
            grow_b.intersect_point(vertical).unwrap(),
            Point::new(-5.0, -85.0),
            0.1
        ));

        let horizontal = Line::from_points(Point::new(0.0, 10.0), Point::new(1.0, 10.0));
        assert!(shrink_b.intersect_point(horizontal).is_ok());
        assert!(points_relatively_same(
            shrink_b.intersect_point(horizontal).unwrap(),
            Point::new(-0.71429, 10.0),
            0.1
        ));
        assert!(vertical.intersect_point(horizontal).is_ok());
        assert!(points_relatively_same(
            vertical.intersect_point(horizontal).unwrap(),
            Point::new(-5.0, 10.0),
            0.1
        ));
    }

    #[test]
    fn line_intersect_parallel_lines() {
        let parallel_a = Line::from_points(Point::new(0.0, 0.0), Point::new(2.0, 10.0));
        let parallel_b = Line::from_points(Point::new(0.0, 10.0), Point::new(2.0, 20.0));

        assert_eq!(
            parallel_a.intersect_point(parallel_b),
            Err(LineError::NoIntersection)
        );
        assert_eq!(
            parallel_a.intersect_point(parallel_a),
            Err(LineError::IdenticalLine)
        );

        let vertical_a = Line::from_points(Point::new(-1.0, 2.0), Point::new(-1.0, 5.0));
        let vertical_b = Line::from_points(Point::new(-10.0, 2.0), Point::new(-10.0, 5.0));
        assert_eq!(
            vertical_a.intersect_point(vertical_b),
            Err(LineError::NoIntersection)
        );
        assert_eq!(
            vertical_a.intersect_point(vertical_a),
            Err(LineError::IdenticalLine)
        );

        let horizontal_a = Line::from_points(Point::new(4.0, 6.0), Point::new(13.0, 6.0));
        let horizontal_b = Line::from_points(Point::new(13.0, 13.0), Point::new(17.0, 13.0));
        assert_eq!(
            horizontal_a.intersect_point(horizontal_b),
            Err(LineError::NoIntersection)
        );
        assert_eq!(
            horizontal_a.intersect_point(horizontal_a),
            Err(LineError::IdenticalLine)
        );
    }
}
