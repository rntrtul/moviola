mod line;
mod point;
mod rectangle;

pub use line::bounding_point_on_edges;
pub use point::rotate_point_around;
pub use rectangle::{Corner, CornerType, EdgeType, Rectangle};
