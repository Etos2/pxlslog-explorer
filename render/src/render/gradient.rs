use super::pixel::{Rgba, Pixel};

#[derive(Debug, Clone)]
struct ColorStep {
    color: Rgba,
    weight: f32,
}

// TODO: Generic weights type (i32, etc)
#[derive(Debug, Clone)]
pub struct Gradient {
    colors: Vec<ColorStep>,
    domain: (f32, f32),
}

impl Gradient {
    pub fn builder() -> GradientBuilder {
        GradientBuilder::default()
    }

    pub fn at(&self, weight: f32) -> Rgba {
        assert!(!weight.is_nan() && !weight.is_infinite());

        match self
            .colors
            .windows(2)
            .find(|c| c[0].weight <= weight && c[1].weight >= weight)
        {
            Some(steps) => {
                let mut current = steps[0].color;
                let previous = steps[1].color.0;

                if weight - steps[0].weight != 0.0 {
                    let interp = (weight - steps[0].weight) / (steps[1].weight - steps[0].weight);

                    for (curr, prev) in current.0.iter_mut().zip(previous.iter()) {
                        *curr =
                            ((*prev as f32 - *curr as f32) * interp + *curr as f32).floor() as u8;
                    }
                }

                current
            }
            None => {
                // SAFETY: Cannot build with empty vec
                if weight < self.domain.0 {
                    self.colors.first().unwrap().color
                } else if weight > self.domain.1 {
                    self.colors.last().unwrap().color
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn _domain(&self) -> (f32, f32) {
        self.domain
    }
}

#[derive(Debug, Default)]
pub struct GradientBuilder {
    colors: Vec<ColorStep>,
}

impl GradientBuilder {
    pub fn _push(mut self, color: impl Into<Rgba>, weight: f32) -> Self {
        assert!(weight >= 0.0);

        self.colors.push(ColorStep {
            color: color.into(),
            weight,
        });
        self
    }

    pub fn push_slice(mut self, colors: &[impl Pixel + Copy], weights: &[f32]) -> Self {
        assert!(colors.len() == weights.len());

        for (color, weight) in colors.iter().zip(weights.iter()) {
            assert!(*weight >= 0.0);

            self.colors.push(ColorStep {
                color: color.to_rgba(),
                weight: *weight,
            });
        }

        self
    }

    pub fn build(mut self) -> Gradient {
        assert!(!self.colors.is_empty());

        self.colors.sort_by(|a, b| a.weight.total_cmp(&b.weight));

        let first = self.colors.first().unwrap().weight;
        let last = self.colors.last().unwrap().weight;

        // Safe unwrap()
        Gradient {
            colors: self.colors,
            domain: (first, last),
        }
    }
}

#[cfg(test)]
mod tests_gradient {

    use crate::render::pixel::Rgba;

    use super::Gradient;

    const COLORS: [[u8; 4]; 7] = [
        [0, 0, 0, 255],
        [0, 0, 255, 255],
        [0, 255, 255, 255],
        [0, 255, 0, 255],
        [255, 255, 0, 255],
        [255, 0, 0, 255],
        [255, 255, 255, 255],
    ];

    #[test]
    fn test_interp_colors() {
        let gradient = init_gradient();

        assert_eq!(
            gradient.at(0.0),
            Rgba::from([0, 0, 0, 255]),
            "color @ 0.0"
        );
        assert_eq!(
            gradient.at(0.5),
            Rgba::from([0, 0, 127, 255]),
            "color @ 0.5"
        );
        assert_eq!(
            gradient.at(1.0),
            Rgba::from([0, 0, 255, 255]),
            "color @ 1.0"
        );
        assert_eq!(
            gradient.at(5.5),
            Rgba::from([0, 127, 255, 255]),
            "color @ 5.5"
        );
        assert_eq!(
            gradient.at(10.0),
            Rgba::from([0, 255, 255, 255]),
            "color @ 10.0"
        );
        assert_eq!(
            gradient.at(55.0),
            Rgba::from([0, 255, 127, 255]),
            "color @ 55.0"
        );
        assert_eq!(
            gradient.at(100.0),
            Rgba::from([0, 255, 0, 255]),
            "color @ 100.0"
        );
        assert_eq!(
            gradient.at(550.0),
            Rgba::from([127, 255, 0, 255]),
            "color @ 550.0"
        );
        assert_eq!(
            gradient.at(1000.0),
            Rgba::from([255, 255, 0, 255]),
            "color @ 1000.0"
        );
        assert_eq!(
            gradient.at(5500.0),
            Rgba::from([255, 127, 0, 255]),
            "color @ 5500.0"
        );
        assert_eq!(
            gradient.at(10000.0),
            Rgba::from([255, 0, 0, 255]),
            "color @ 10000.0"
        );
        assert_eq!(
            gradient.at(55000.0),
            Rgba::from([255, 127, 127, 255]),
            "color @ 55000.0"
        );
        assert_eq!(
            gradient.at(100000.0),
            Rgba::from([255, 255, 255, 255]),
            "color @ 100000.0"
        );
    }

    #[test]
    fn test_domain() {
        let gradient = init_gradient();

        assert_eq!(gradient.at(f32::MIN), Rgba::from([0, 0, 0, 255]));
        assert_eq!(
            gradient.at(f32::MAX),
            Rgba::from([255, 255, 255, 255])
        );
    }

    #[test]
    #[should_panic]
    fn test_invalid_weight() {
        let gradient = init_gradient();
        gradient.at(f32::NAN);
    }

    fn init_gradient() -> Gradient {
        Gradient::builder()
            ._push(COLORS[0], 0.0)
            ._push(COLORS[1], 1.0)
            ._push(COLORS[2], 10.0)
            ._push(COLORS[3], 100.0)
            ._push(COLORS[4], 1000.0)
            ._push(COLORS[5], 10000.0)
            ._push(COLORS[6], 100000.0)
            .build()
    }
}

#[cfg(test)]
mod tests_gradient_builder {

    use crate::render::pixel::Rgba;

    use super::Gradient;

    const COLORS: [Rgba; 7] = [
        Rgba([0, 0, 0, 255]),
        Rgba([0, 0, 255, 255]),
        Rgba([0, 255, 255, 255]),
        Rgba([0, 255, 0, 255]),
        Rgba([255, 255, 0, 255]),
        Rgba([255, 0, 0, 255]),
        Rgba([255, 255, 255, 255]),
    ];

    const WEIGHTS: [f32; 7] = [0.0, 1.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0];

    #[test]
    #[should_panic]
    fn test_empty_gradient() {
        Gradient::builder().build();
    }

    #[test]
    #[should_panic]
    fn test_negative_gradient() {
        Gradient::builder()
            ._push(COLORS[0], -0.0)
            ._push(COLORS[1], -1.0)
            ._push(COLORS[2], -10.0)
            ._push(COLORS[3], -100.0)
            ._push(COLORS[4], -1000.0)
            ._push(COLORS[5], -10000.0)
            ._push(COLORS[6], -100000.0)
            .build();
    }

    #[test]
    fn test_equilavence() {
        let a = Gradient::builder()
            ._push(COLORS[0], 0.0)
            ._push(COLORS[1], 1.0)
            ._push(COLORS[2], 10.0)
            ._push(COLORS[3], 100.0)
            ._push(COLORS[4], 1000.0)
            ._push(COLORS[5], 10000.0)
            ._push(COLORS[6], 100000.0)
            .build();

        let b = Gradient::builder().push_slice(&COLORS, &WEIGHTS).build();
        
        for (a, b) in a.colors.iter().zip(b.colors.iter()) {
            assert_eq!(a.color, b.color);
            assert_eq!(a.weight, b.weight);
        }
    }
}
