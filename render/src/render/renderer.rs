use std::num::NonZeroI64;

use common::data::action::Index;
use common::data::actions::ActionsView;
use rayon::prelude::*;

use super::frame::{DynamicFrame, VideoFrame};
use super::gradient::Gradient;
use super::pixel::{Pixel, Rgba};
use crate::palette::Palette;
use common::data::actionkind::ActionKind;

const ACTIVITY_GRADIENT: [Rgba; 9] = [
    Rgba([11, 21, 97, 255]),
    Rgba([32, 156, 194, 255]),
    Rgba([122, 222, 142, 255]),
    Rgba([245, 250, 212, 255]),
    Rgba([247, 151, 45, 255]),
    Rgba([211, 17, 34, 255]),
    Rgba([0, 0, 0, 255]),
    Rgba([131, 22, 161, 255]),
    Rgba([240, 101, 243, 255]),
];

const ACTIVITY_WEIGHTS: [f32; 9] = [
    0.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0,
];

pub trait ActionRenderer {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel + Send,
        V: VideoFrame<Format = P>;
}

#[derive(Debug, Clone)]
pub struct RendererNormal {
    background: DynamicFrame,
    palette: Palette,
}

impl RendererNormal {
    pub fn new(background: DynamicFrame, palette: Palette) -> Self {
        RendererNormal {
            background,
            palette,
        }
    }
}

impl ActionRenderer for RendererNormal {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            let pixel = match action.index {
                Some(index) => match index {
                    Index::Color(index) => match self.palette.get(index) {
                        Some(color) => *color,
                        None => match self
                            .background
                            .get_pixel_checked(action.coord.0, action.coord.1)
                        {
                            Some(color) => color,
                            None => [0, 0, 0, 255].into(),
                        },
                    },
                    Index::Transparent => match self
                        .background
                        .get_pixel_checked(action.coord.0, action.coord.1)
                    {
                        Some(color) => color,
                        None => [0, 0, 0, 255].into(),
                    },
                },
                None => unreachable!(),
            };

            frame.put_pixel(action.coord.0, action.coord.1, pixel.into());
        }
    }
}

// TODO: Remove map?
// TODO: Replace with grid?
#[derive(Debug, Clone)]
pub struct RendererActivity {
    totals_map: Vec<u32>,
    width: u32,
    height: u32,
    gradient: Gradient,
}

impl RendererActivity {
    pub fn new(width: u32, height: u32) -> Self {
        let gradient = Gradient::builder()
            .push_slice(&ACTIVITY_GRADIENT, &ACTIVITY_WEIGHTS)
            .build();

        RendererActivity {
            totals_map: vec![0; width as usize * height as usize],
            width,
            height,
            gradient,
        }
    }
}

impl ActionRenderer for RendererActivity {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            let index = (action.coord.0 + action.coord.1 * self.width) as usize;
            self.totals_map[index] += 1;
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let index = (x + y * self.width) as usize;
                let total = self.totals_map[index] as f32;

                frame.put_pixel(x, y, self.gradient.at(total).into());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RendererVirgin;

impl ActionRenderer for RendererVirgin {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            frame.put_pixel(action.coord.0, action.coord.1, [0, 0, 0, 255].into());
        }
    }
}

#[derive(Debug, Clone)]
pub struct RendererHeat {
    heat_map: Vec<Option<NonZeroI64>>,
    width: u32,
    step: NonZeroI64,
    current_step: i64,
    window: f64,
}

impl RendererHeat {
    pub fn new(width: u32, height: u32, step: NonZeroI64, window: i64) -> Self {
        RendererHeat {
            heat_map: vec![None; width as usize * height as usize],
            width,
            step,
            current_step: 1,
            window: window as f64,
        }
    }
}

impl ActionRenderer for RendererHeat {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel + Send,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            let index = action.coord.0 + action.coord.1 * self.width;
            self.heat_map[index as usize] = action.time.timestamp_millis().try_into().ok();

            if action.time.timestamp_millis() > self.step.get() * self.current_step {
                self.current_step = action.time.timestamp_millis() / self.step.get() + 1;
            }
        }

        frame.put_from_par_iter(self.heat_map.par_iter().map(|heat| {
            if let Some(delta) = heat {
                let diff = (self.step.get() * self.current_step - delta.get()) as f64 / self.window; //10800000.0;
                if diff < 1.0 {
                    let val = 1.0 - diff;
                    let r = (val * 205.0) as u8;
                    let g = (val * 92.0) as u8;
                    let b = (val * 92.0) as u8;
                    [r, g, b, 255].into()
                } else {
                    [0, 0, 0, 255].into()
                }
            } else {
                [0, 0, 0, 255].into()
            }
        }));
    }
}

#[derive(Debug, Clone)]
pub struct RendererAction;

impl ActionRenderer for RendererAction {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            frame.put_pixel(
                action.coord.0,
                action.coord.1,
                match action.kind {
                    Some(kind) => match kind {
                        ActionKind::Undo => [255, 0, 255, 255].into(),
                        ActionKind::Place => [0, 0, 255, 255].into(),
                        ActionKind::Overwrite => [0, 255, 255, 255].into(),
                        ActionKind::Rollback => [0, 255, 0, 255].into(),
                        ActionKind::RollbackUndo => [255, 255, 0, 255].into(),
                        ActionKind::Nuke => [255, 0, 0, 255].into(),
                    },
                    None => unreachable!(),
                },
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct RendererPlacement {
    step: i64,
    color: Rgba,
}

impl RendererPlacement {
    pub fn new(color: Rgba, step: i64) -> Self {
        RendererPlacement { color, step }
    }
}

impl ActionRenderer for RendererPlacement {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            let val = ((action.time.timestamp_millis() - 1) % self.step) as f32 / self.step as f32;
            let color = color_lerp(self.color, val);
            frame.put_pixel(action.coord.0, action.coord.1, color.into());
        }
    }
}

#[derive(Debug, Clone)]
pub struct RendererCombined;

impl ActionRenderer for RendererCombined {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            let r = (((action.time.timestamp_millis() - 1) % 1000) as f32 / 1000.0 * 255.0) as u8;
            let g = (((action.time.timestamp_millis() - 1) % 60000) as f32 / 60000.0 * 255.0) as u8;
            let b =
                (((action.time.timestamp_millis() - 1) % 3600000) as f32 / 3600000.0 * 255.0) as u8;

            frame.put_pixel(action.coord.0, action.coord.1, [r, g, b, 255].into());
        }
    }
}

#[derive(Debug, Clone)]
pub struct RendererAge {
    age_map: Vec<i64>,
    width: u32,
    min: Option<i64>,
    max: i64,
}

impl RendererAge {
    pub fn new(width: u32, height: u32) -> Self {
        RendererAge {
            age_map: vec![0; width as usize * height as usize],
            width,
            min: None,
            max: i64::MIN,
        }
    }
}

impl ActionRenderer for RendererAge {
    fn update<'a, P, V>(&mut self, actions: impl Iterator<Item = ActionsView<'a>>, frame: &mut V)
    where
        P: Pixel + Send,
        V: VideoFrame<Format = P>,
    {
        for action in actions {
            self.max = action.time.timestamp_millis();
            if self.min.is_none() {
                self.min = Some(action.time.timestamp_millis());
            }

            let index = action.coord.0 + action.coord.1 * self.width;
            self.age_map[index as usize] = action.time.timestamp_millis();
        }

        frame.put_from_par_iter(self.age_map.par_iter().map(|age| {
            if *age == 0 {
                [0, 0, 0, 255].into()
            } else {
                // SAFETY: Initialised above
                let dividend = (age - self.min.unwrap()) as f32;
                let divisor = (self.max - self.min.unwrap()) as f32;
                let color = color_lerp([0, 0, 255, 255].into(), dividend / divisor);
                color.into()
            }
        }));
    }
}

// TODO: integer lerp?
// TODO: Remove function?
fn color_lerp(color: Rgba, val: f32) -> Rgba {
    if val < 0.5 {
        let val = val * 2.0;
        let r = (color.0[0] as f32 * val) as u8;
        let g = (color.0[1] as f32 * val) as u8;
        let b = (color.0[2] as f32 * val) as u8;
        [r, g, b, 255].into()
    } else {
        let val = (val - 0.5) * 2.0;
        let r = (color.0[0] as f32 + (255 - color.0[0]) as f32 * val) as u8;
        let g = (color.0[1] as f32 + (255 - color.0[1]) as f32 * val) as u8;
        let b = (color.0[2] as f32 + (255 - color.0[2]) as f32 * val) as u8;
        [r, g, b, 255].into()
    }
}

#[cfg(test)]
mod test {
    use arbitrary::*;

    use super::*;

    #[test]
    fn color_interpolation() {
        arbtest::builder().run(|u| {
            let color = Rgba::from(<[u8; 4]>::arbitrary(u)?);
            let r = color.0[0];
            let g = color.0[1];
            let b = color.0[2];

            let mut expected = color;
            expected.0[3] = 255;

            assert_eq!(color_lerp(color, 0.0), *Rgba::from_slice(&[0, 0, 0, 255]));
            assert_eq!(color_lerp(color, 0.5), expected);
            assert_eq!(
                color_lerp(color, 1.0),
                *Rgba::from_slice(&[255, 255, 255, 255])
            );

            assert_eq!(
                color_lerp(color, 0.25),
                *Rgba::from_slice(&[r / 2, g / 2, b / 2, 255])
            );
            assert_eq!(
                color_lerp(color, 0.75),
                *Rgba::from_slice(&[r + (255 - r) / 2, g + (255 - g) / 2, b + (255 - b) / 2, 255])
            );

            Ok(())
        });
    }
}
