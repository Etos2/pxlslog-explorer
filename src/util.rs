use num_traits::{Bounded, NumOps};

#[derive(Debug)]
pub struct Region<T>
{
    x1: T,
    y1: T,
    x2: T,
    y2: T,
}

#[allow(dead_code)]
impl<T> Region<T>
where
    T: PartialOrd + Bounded + NumOps + Copy,
{
    pub fn new(x1: T, y1: T, x2: T, y2: T) -> Region<T> {
        let mut out = Region { x1, y1, x2, y2 };

        if out.x1 > out.x2 {
            std::mem::swap(&mut out.x1, &mut out.x2);
        }
        if out.y1 > out.y2 {
            std::mem::swap(&mut out.y1, &mut out.y2);
        }

        out
    }

    pub fn new_from_slice(region: &[T]) -> Region<T> {
        match region.len() {
            0 => Region::all(),
            1 => Region::new(region[0], T::min_value(), T::max_value(), T::max_value()),
            2 => Region::new(region[0], region[1], T::max_value(), T::max_value()),
            3 => Region::new(region[0], region[1], region[2], T::max_value()),
            4 => Region::new(region[0], region[1], region[2], region[3]),
            _ => panic!("Region only contains 4 values"),
        }
    }

    pub fn all() -> Region<T> {
        Region {
            x1: T::min_value(),
            y1: T::min_value(),
            x2: T::max_value(),
            y2: T::max_value(),
        }
    }

    pub fn contains(&self, x: T, y: T) -> bool {
        self.x1 <= x && self.x2 > x && self.y1 <= y && self.y2 > y
    }

    pub fn top_left(&self) -> (T, T) {
        (self.x1, self.y1)
    }

    pub fn bottom_right(&self) -> (T, T) {
        (self.x2, self.y2)
    }

    pub fn width(&self) -> T {
        self.x2 - self.x1
    }

    pub fn height(&self) -> T {
        self.y2 - self.y1
    }
}
