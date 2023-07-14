pub mod frame;
pub mod gradient;
pub mod pixel;
mod renderer;

use std::fmt::Display;
use std::io::{BufWriter, Write};
use std::num::NonZeroI64;
use std::path::Path;

use crate::config::{DestinationKind, MethodKind, PaletteSource, RenderConfig};
use crate::error::RuntimeError;
use crate::palette::{Palette, PaletteParser, DEFAULT_PALETTE};
use crate::render::pixel::Pixel;

use common::data::action::Action;
use image::io::Reader as ImageReader;
use image::{imageops, ImageBuffer};
use itertools::Itertools;
use nonzero_ext::nonzero;

use self::frame::{DynamicFrame, VideoFrame};
use self::pixel::Rgb;
use self::renderer::{
    ActionRenderer, RendererAction, RendererActivity, RendererAge, RendererCombined, RendererHeat,
    RendererNormal, RendererPlacement, RendererVirgin,
};

#[derive(Debug)]
pub struct RenderCommand {
    destination: DestinationKind,
    step: Step,
    skip: usize,
    offset: (u32, u32),
    background: DynamicFrame,
    palette: Palette,
    method: MethodKind,
}

impl RenderCommand {
    // TODO: Respect image type
    pub fn new(config: RenderConfig, bounds: (u32, u32, u32, u32)) -> Result<Self, RuntimeError> {
        let size = config.canvas.size.unwrap_or(bounds);

        let (background, offset) = match config.canvas.source {
            Some(path) => {
                let image = ImageReader::open(path)?.decode()?;
                if image.width() < size.2 - size.0 || image.height() < size.3 - size.1 {
                    let mut temp = ImageBuffer::from_pixel(
                        size.2 - size.0,
                        size.3 - size.1,
                        Rgb([255, 255, 255]).into(),
                    );
                    imageops::overlay(&mut temp, &image, 0, 0);
                    (
                        DynamicFrame::from_image(config.destination.format, temp.into()),
                        (0, 0),
                    )
                } else {
                    (
                        DynamicFrame::from_image(config.destination.format, image),
                        (0, 0),
                    )
                }
            }
            None => (
                DynamicFrame::from_pixel(
                    config.destination.format,
                    size.2 - size.0,
                    size.3 - size.1,
                    Rgb([255, 255, 255]),
                ),
                (size.0, size.1),
            ),
        };

        let palette = if let Some(palette) = config.method.palette {
            match palette {
                // TODO: Redo palette parser error handling
                PaletteSource::File(path) => PaletteParser::try_parse(&path).unwrap(),
                PaletteSource::Array(p) => p,
            }
        } else {
            DEFAULT_PALETTE.to_vec()
        };

        eprintln!("{:?}", background.dimensions());
        eprintln!("{:?}", offset);

        Ok(Self {
            destination: config.destination.kind,
            step: config.step,
            skip: 0,
            offset,
            background,
            palette,
            method: config.method.kind,
        })
    }

    // TODO (Etos2): Consider data oriented design
    // TODO (Etos2): Optional parsing (ignore unneeded/ expensive data)
    // TODO (Etos2): Dumber parsing (use strict mode for "smarter" parser?)
    // TODO (Etos2): Replace reader with smarter type (IntoActionBatch?)
    // TODO (Etos2): Replace format with appriorate enum
    pub fn run<'a>(&self, actions: impl Iterator<Item = &'a Action>) -> anyhow::Result<()> {
        let actions_iter = actions.cloned().map(|mut a| {
            a.x -= self.offset.0;
            a.y -= self.offset.1;
            a
        });

        // TODO (Etos2): Reduce boilerplate
        match self.method {
            MethodKind::Normal => {
                // TODO: Remove clones?
                let renderer = RendererNormal::new(self.background.clone(), self.palette.clone());
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Heatmap(window) => {
                let (width, height) = self.background.dimensions();
                let renderer = RendererHeat::new(width, height, self.step.get(), window.into());
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Virgin => {
                let renderer = RendererVirgin {};
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Activity => {
                let (width, height) = self.background.dimensions();
                let renderer = RendererActivity::new(width, height);
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Action => {
                let renderer = RendererAction {};
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Milliseconds => {
                let renderer = RendererPlacement::new([255, 0, 0, 255].into(), 1000);
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Seconds => {
                let renderer = RendererPlacement::new([0, 255, 0, 255].into(), 60000);
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Minutes => {
                let renderer = RendererPlacement::new([0, 0, 255, 255].into(), 3600000);
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Combined => {
                let renderer = RendererCombined {};
                self.render(renderer, actions_iter)?;
            }
            MethodKind::Age => {
                let (width, height) = self.background.dimensions();
                let renderer = RendererAge::new(width, height);
                self.render(renderer, actions_iter)?;
            }
        }

        Ok(())
    }

    fn render(
        &self,
        renderer: impl ActionRenderer,
        actions: impl Iterator<Item = Action>,
    ) -> anyhow::Result<()> {
        let mut background = self.background.clone();
        // TODO: implement some form of Into<> for DestinationKind for output to avoid match
        // TODO: This may involve checking if destination has trait Seek
        match (&mut background, &self.destination) {
            (DynamicFrame::Rgba(rgba_frame), DestinationKind::Stdout) => {
                Self::render_to_raw(renderer, actions, rgba_frame, self.step)
            }
            (DynamicFrame::Rgba(rgba_frame), DestinationKind::File(dst)) => {
                Self::render_to_file(renderer, actions, dst, rgba_frame, self.step)
            }
            (DynamicFrame::Rgb(rgb_frame), DestinationKind::Stdout) => {
                Self::render_to_raw(renderer, actions, rgb_frame, self.step)
            }
            (DynamicFrame::Rgb(rgb_frame), DestinationKind::File(dst)) => {
                Self::render_to_file(renderer, actions, dst, rgb_frame, self.step)
            }
            (DynamicFrame::Yuv420p(yuv420p_frame), DestinationKind::Stdout) => {
                Self::render_to_raw(renderer, actions, yuv420p_frame, self.step)
            }
            (DynamicFrame::Yuv420p(yuv420p_frame), DestinationKind::File(dst)) => {
                Self::render_to_file(renderer, actions, dst, yuv420p_frame, self.step)
            }
            (_, _) => unimplemented!(),
        }
    }

    // TODO (Etos2): Generic writing of pixels to frame (YUV420p, RGBA, RGB, etc)
    fn render_to_raw<V: VideoFrame>(
        mut renderer: impl ActionRenderer,
        actions: impl Iterator<Item = Action>,
        frame: &mut V,
        step: Step,
    ) -> anyhow::Result<()> {
        let stdout = std::io::stdout();
        let handle = stdout.lock();
        // TODO (Etos2): Frame.write_size()
        // TODO (Etos2): Use iter to control if background is drawn first (--skip)
        let mut handle = BufWriter::with_capacity(1024, handle);

        match step {
            Step::Time(millis_per_frame) => actions
                .group_by(|a| a.time.timestamp_millis() / millis_per_frame.get())
                .into_iter()
                .try_for_each(|(_, action_group)| -> anyhow::Result<()> {
                    renderer.update(action_group, frame);
                    handle.write_all(frame.as_formatted_raw())?;
                    handle.flush()?;
                    Ok(())
                })?,
            Step::Pixels(pixels_per_frame) => actions
                .chunks(pixels_per_frame.get().try_into()?)
                .into_iter()
                .try_for_each(|action_group| -> anyhow::Result<()> {
                    renderer.update(action_group, frame);
                    handle.write_all(frame.as_formatted_raw())?;
                    handle.flush()?;
                    Ok(())
                })?,
        }

        Ok(())
    }

    // TODO (Etos2): Handle file IO manually to avoid overwriting files
    fn render_to_file<V: VideoFrame>(
        mut renderer: impl ActionRenderer,
        actions: impl Iterator<Item = Action>,
        path: impl AsRef<Path>,
        frame: &mut V,
        step: Step,
    ) -> anyhow::Result<()> {
        let (width, height) = frame.dimensions();

        eprintln!("Rendering");

        match step {
            Step::Time(millis_per_frame) => {
                for (_, action_group) in
                    &actions.group_by(|a| a.time.timestamp_millis() / millis_per_frame.get())
                {
                    renderer.update(action_group, frame);
                    image::save_buffer(
                        path.as_ref(),
                        frame.as_formatted_raw(),
                        width,
                        height,
                        V::Format::TYPE,
                    )?;
                }
            }
            Step::Pixels(pixels_per_frame) => {
                for action_group in &actions.chunks(pixels_per_frame.get().try_into()?) {
                    renderer.update(action_group, frame);
                    image::save_buffer(
                        path.as_ref(),
                        frame.as_formatted_raw(),
                        width,
                        height,
                        V::Format::TYPE,
                    )?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RenderMethod {
    Normal,
    Heat(NonZeroI64),
    Virgin,
    Activity,
    Action,
    Milliseconds,
    Seconds,
    Minutes,
    Combined,
    Age,
}

impl Display for RenderMethod {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderMethod::Normal => write!(fmt, "normal"),
            RenderMethod::Heat(_) => write!(fmt, "heat"),
            RenderMethod::Virgin => write!(fmt, "virgin"),
            RenderMethod::Activity => write!(fmt, "activity"),
            RenderMethod::Action => write!(fmt, "action"),
            RenderMethod::Milliseconds => write!(fmt, "milliseconds"),
            RenderMethod::Seconds => write!(fmt, "seconds"),
            RenderMethod::Minutes => write!(fmt, "minutes"),
            RenderMethod::Combined => write!(fmt, "combined"),
            RenderMethod::Age => write!(fmt, "age"),
        }
    }
}

impl TryFrom<String> for RenderMethod {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl<'a> TryFrom<&'a str> for RenderMethod {
    type Error = ();

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "normal" => Ok(RenderMethod::Normal),
            "heat" => Ok(RenderMethod::Heat(nonzero!(10800000_i64))),
            "virgin" => Ok(RenderMethod::Virgin),
            "activity" => Ok(RenderMethod::Activity),
            "action" => Ok(RenderMethod::Action),
            "milliseconds" => Ok(RenderMethod::Milliseconds),
            "seconds" => Ok(RenderMethod::Seconds),
            "minutes" => Ok(RenderMethod::Minutes),
            "combined" => Ok(RenderMethod::Combined),
            "age" => Ok(RenderMethod::Age),
            _ => Err(()),
        }
    }
}

impl Default for RenderMethod {
    fn default() -> Self {
        RenderMethod::Normal
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Step {
    // TODO: Duration
    Time(NonZeroI64),
    Pixels(NonZeroI64),
}

impl Step {
    pub fn get(&self) -> NonZeroI64 {
        match self {
            Step::Time(step) => *step,
            Step::Pixels(step) => *step,
        }
    }
}

impl Default for Step {
    fn default() -> Self {
        Step::Time(nonzero!(900000i64)) // 15 minutes
    }
}

fn rgba_to_yuv420_size(width: u32, height: u32) -> usize {
    (width * height * 3 / 2) as usize
}

fn rgba_data_to_yuv420_data(yuv: &mut [u8], img: &[u8], width: u32, height: u32) {
    assert_eq!(height % 2, 0);
    assert_eq!(width % 2, 0);

    let frame_size = (width * height) as usize;
    let mut index = (0, width as usize);
    let mut u_index = frame_size;
    let mut v_index = frame_size + frame_size / 4;

    for _ in (0..height).step_by(2) {
        for _ in (0..width).step_by(2) {
            let r = i32::from(img[4 * index.0]);
            let g = i32::from(img[4 * index.0 + 1]);
            let b = i32::from(img[4 * index.0 + 2]);
            yuv[index.0] = clamp(((66 * r + 129 * g + 25 * b) >> 8) + 16);
            let mut r_total = r;
            let mut g_total = g;
            let mut b_total = b;
            index.0 += 1;

            let r = i32::from(img[4 * index.0]);
            let g = i32::from(img[4 * index.0 + 1]);
            let b = i32::from(img[4 * index.0 + 2]);
            yuv[index.0] = clamp(((66 * r + 129 * g + 25 * b) >> 8) + 16);
            r_total += r;
            g_total += g;
            b_total += b;
            index.0 += 1;

            let r = i32::from(img[4 * index.1]);
            let g = i32::from(img[4 * index.1 + 1]);
            let b = i32::from(img[4 * index.1 + 2]);
            yuv[index.1] = clamp(((66 * r + 129 * g + 25 * b) >> 8) + 16);
            r_total += r;
            g_total += g;
            b_total += b;
            index.1 += 1;

            let r = i32::from(img[4 * index.1]);
            let g = i32::from(img[4 * index.1 + 1]);
            let b = i32::from(img[4 * index.1 + 2]);
            yuv[index.1] = clamp(((66 * r + 129 * g + 25 * b) >> 8) + 16);
            r_total += r;
            g_total += g;
            b_total += b;
            index.1 += 1;

            let r = r_total / 4;
            let g = g_total / 4;
            let b = b_total / 4;
            yuv[u_index] = clamp(((-38 * r + -74 * g + 112 * b) >> 8) + 128);
            yuv[v_index] = clamp(((112 * r + -94 * g + -18 * b) >> 8) + 128);
            u_index += 1;
            v_index += 1;
        }

        index.0 += width as usize;
        index.1 += width as usize;
    }
}

fn clamp(val: i32) -> u8 {
    match val {
        ref v if *v < 0 => 0,
        ref v if *v > 255 => 255,
        v => v as u8,
    }
}
