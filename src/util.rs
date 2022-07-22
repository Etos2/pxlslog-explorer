use num_traits::{Bounded, NumOps};

#[derive(Debug)]
pub struct Region<T> {
    start: (T, T),
    end: (T, T),
}

#[allow(dead_code)]
impl<T> Region<T>
where
    T: PartialOrd + Bounded + NumOps + Copy,
{
    pub fn new(x: T, y: T, width: T, height: T) -> Region<T> {
        let mut out = Region {
            start: (x, y),
            end: (x + width, y + height),
        };

        if out.start.0 > out.end.0 {
            std::mem::swap(&mut out.start.0, &mut out.end.0);
        }
        if out.start.1 > out.end.1 {
            std::mem::swap(&mut out.start.1, &mut out.end.1);
        }

        out
    }

    pub fn new_from_slice(region: &[T]) -> Option<Region<T>> {
        match region.len() {
            1 => Some(Region {
                start: (region[0], T::min_value()),
                end: (T::max_value(), T::max_value()),
            }),
            2 => Some(Region {
                start: (region[0], region[1]),
                end: (T::max_value(), T::max_value()),
            }),
            3 => Some(Region {
                start: (region[0], region[1]),
                end: (region[0] + region[2], region[1] + T::max_value()),
            }),
            4 => Some(Region {
                start: (region[0], region[1]),
                end: (region[0] + region[2], region[1] + region[3]),
            }),
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
        self.start.0 <= x && self.end.0 > x && self.start.1 <= y && self.end.1 > y
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