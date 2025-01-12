use gtk4::graphene::Point;

fn rotate_point(point: Point, degrees: f32) -> Point {
    let angle = degrees.to_radians();

    let rotated_x = (point.x() * angle.cos()) - (point.y() * angle.sin());
    let rotated_y = (point.y() * angle.cos()) + (point.x() * angle.sin());

    Point::new(rotated_x, rotated_y)
}

pub fn rotate_point_around(point: Point, origin: Point, degrees: f32) -> Point {
    let rotated = rotate_point(
        Point::new(point.x() - origin.x(), point.y() - origin.y()),
        degrees,
    );

    Point::new(rotated.x() + origin.x(), rotated.y() + origin.y())
}

pub(crate) fn point_distance(a: Point, b: Point) -> f32 {
    ((b.x() - a.x()).powi(2) + (b.y() - a.y()).powi(2)).sqrt()
}
