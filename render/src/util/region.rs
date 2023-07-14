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

#[cfg(test)]
mod test {
    use super::*;
    use arbitrary::*;

    impl<'a, T> Arbitrary<'a> for Region<T>
    where
        T: Arbitrary<'a>
            + PartialOrd
            + Bounded
            + NumOps
            + Copy
            + Zero
            + Unsigned
            + unstructured::Int,
    {
        fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
            let x1 = u.int_in_range(T::zero()..=T::max_value() - T::one())?;
            let y1 = u.int_in_range(T::zero()..=T::max_value() - T::one())?;
            let x2 = u.int_in_range(x1 + T::one()..=T::max_value())?;
            let y2 = u.int_in_range(y1 + T::one()..=T::max_value())?;

            assert!(x1 < x2);
            assert!(y1 < y2);

            Ok(Region {
                start: (x1, y1),
                end: (x2, y2),
            })
        }
    }

    #[test]
    fn region_new() {
        arbtest::builder().run(|u| {
            let (x1, y1, x2, y2) = <(u32, u32, u32, u32)>::arbitrary(u)?;
            let region = Region::new(x1, y1, x2, y2);

            if x1 <= x2 && y1 <= y2 {
                assert!(region.is_some());
                let r = region.unwrap();
                assert!(r.start == (x1, y1));
                assert!(r.end == (x2, y2));
            } else {
                assert!(region.is_none());
            }

            Ok(())
        });
    }

    #[test]
    fn region_new_point() {
        arbtest::builder().run(|u| {
            let (x, y) = <(u32, u32)>::arbitrary(u)?;
            let region = Region::new(x, y, x, y);

            assert!(region.is_some());
            let r = region.unwrap();
            assert!(r.start == (x, y));
            assert!(r.end == (x, y));
            assert!(r.contains(x, y));

            Ok(())
        });
    }

    #[test]
    fn region_trifecta() {
        let region_a = Region::new(u32::MIN, u32::MIN, u32::MAX, u32::MAX).unwrap();
        let region_b = Region::default();
        let region_c = Region::all();

        assert_eq!(region_a, region_b);
        assert_eq!(region_b, region_c);
        assert_eq!(region_c, region_a);
    }

    #[test]
    fn region_from_slice() {
        arbtest::builder().run(|u| {
            let items = <Vec<u32>>::arbitrary(u)?;

            if (1..=4).contains(&items.len()) {
                assert_eq!(
                    Region::from_slice(&items),
                    Region::new(
                        *items.first().unwrap_or(&u32::MIN),
                        *items.get(1).unwrap_or(&u32::MIN),
                        *items.get(2).unwrap_or(&u32::MAX),
                        *items.get(3).unwrap_or(&u32::MAX),
                    )
                );
            } else {
                assert_eq!(Region::from_slice(&items), None);
            }
            Ok(())
        });
    }

    #[test]
    fn region_fields() {
        arbtest::builder().run(|u| {
            let region = Region::<u32>::arbitrary(u)?;
            assert_eq!(region.start(), region.start);
            assert_eq!(region.end(), region.end);
            assert_eq!(region.width(), region.end.0 - region.start.0);
            assert_eq!(region.height(), region.end.1 - region.start.1);
            Ok(())
        });
    }

    #[test]
    fn region_contains() {
        arbtest::builder().run(|u| {
            let region = Region::<u32>::arbitrary(u)?;
            let (x, y) = <(u32, u32)>::arbitrary(u)?;
            if x >= region.start.0 && x <= region.end.0 && y >= region.start.1 && y <= region.end.1
            {
                assert!(region.contains(x, y));
            } else {
                assert!(!region.contains(x, y));
            }
            Ok(())
        });
    }
}
