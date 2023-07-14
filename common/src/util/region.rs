use num_traits::{Bounded, NumOps, Unsigned, Zero};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region<T> {
    start: (T, T),
    end: (T, T),
}

// TODO: Signed? (i32::MAX - i32::MIN in width() overflows)
#[allow(dead_code)]
impl<T> Region<T>
where
    T: PartialOrd + Bounded + NumOps + Copy + Zero + Unsigned,
{
    pub fn new(x1: T, y1: T, x2: T, y2: T) -> Option<Region<T>> {
        if x1 <= x2 && y1 <= y2 {
            Some(Region {
                start: (x1, y1),
                end: (x2, y2),
            })
        } else {
            None
        }
    }

    pub fn from_slice(region: &[T]) -> Option<Region<T>> {
        match region.len() {
            1 => Region::new(region[0], T::min_value(), T::max_value(), T::max_value()),
            2 => Region::new(region[0], region[1], T::max_value(), T::max_value()),
            3 => Region::new(region[0], region[1], region[2], T::max_value()),
            4 => Region::new(region[0], region[1], region[2], region[3]),
            _ => None,
        }
    }

    pub fn all() -> Region<T> {
        Region {
            start: (T::min_value(), T::min_value()),
            end: (T::max_value(), T::max_value()),
        }
    }

    pub fn contains(&self, x: T, y: T) -> bool {
        self.start.0 <= x && self.end.0 >= x && self.start.1 <= y && self.end.1 >= y
    }

    pub fn start(&self) -> (T, T) {
        self.start
    }

    pub fn end(&self) -> (T, T) {
        self.end
    }

    pub fn width(&self) -> T {
        self.end.0 - self.start.0
    }

    pub fn height(&self) -> T {
        self.end.1 - self.start.1
    }
}

impl<T> Default for Region<T>
where
    T: Bounded + PartialOrd + NumOps + Copy + Zero + Unsigned,
{
    fn default() -> Self {
        Region::all()
    }
}