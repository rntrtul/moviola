use gtk4::graphene;
use gtk4::graphene::Point;

pub enum EdgeType {
    Left,
    Top,
    Right,
    Bottom,
}

pub enum CornerType {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
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

    (py - y_from_x).abs() <= 0.01
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

pub(crate) fn x_y_distance_to_edge_from_point(
    line_a: Point,
    line_b: Point,
    point: Point,
) -> (f32, f32) {
    let slope = (line_b.y() - line_a.y()) / (line_b.x() - line_a.x());
    let b = (-slope * line_a.x()) + line_a.y();

    // println!("slope: {slope}, y-intercept: {b}");
    let x = (point.y() - b) / slope;
    let y = (slope * point.x()) + b;

    // fixme: cleanup the if statements
    let x_dist = if num_in_unordered_range(line_a.y(), line_b.y(), point.y()) {
        x - point.x()
    } else {
        if point.y() < line_a.y() {
            if slope > 0.0 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            }
        } else {
            if slope > 0.0 {
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
            if slope > 0.0 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            }
        } else {
            if slope > 0.0 {
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

        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, Point::new(8.0, 0.0)),
            (-7.0, 7.777778)
        );
        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, Point::new(2.0, 2.0)),
            (0.79999995, -0.88888884)
        );

        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, Point::new(1.0, 0.0)),
            (0.0, 0.0)
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
            x_y_distance_to_edge_from_point(a, b, Point::new(12.0, 2.0)),
            (-9.2, f32::INFINITY)
        );
        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, above_right),
            (f32::INFINITY, f32::INFINITY)
        );
        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, above_left),
            (f32::INFINITY, f32::NEG_INFINITY)
        );

        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, below_left),
            (f32::NEG_INFINITY, f32::NEG_INFINITY)
        );

        assert_eq!(
            x_y_distance_to_edge_from_point(a, b, below_right),
            (f32::NEG_INFINITY, f32::INFINITY)
        );
    }
}
