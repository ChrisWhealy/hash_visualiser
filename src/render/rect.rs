use svg_dom::root::utils::{Point, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Axis-aligned box giving a node's placement, in user units, with its top-left corner at `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }
    
    pub fn centre(&self) -> Point {
        Point { x: self.top_left.x + self.size.width / 2.0, y: self.top_left.y + self.size.height / 2.0 }
    }
}

impl From<Rect> for Point {
    fn from(r: Rect) -> Self {
        Point { x: r.top_left.x, y: r.top_left.y }
    }
}

impl From<Rect> for Size {
    fn from(r: Rect) -> Self {
        Size { width: r.size.width, height: r.size.height }
    }
}

