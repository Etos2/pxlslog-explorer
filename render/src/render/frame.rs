use crate::config::PixelFormat;

use super::pixel::{Pixel, Rgb, Rgba};
use image::{DynamicImage, RgbImage, RgbaImage};
use num_integer::Roots;
use rayon::prelude::*;

pub trait VideoFrame {
    type Format: Pixel + Send;
    fn dimensions(&self) -> (u32, u32);
    fn from_pixel(width: u32, height: u32, pixel: impl Pixel) -> Self;
    fn get_pixel_checked(&self, x: u32, y: u32) -> Option<&Self::Format>;
    fn put_pixel(&mut self, x: u32, y: u32, val: Self::Format);
    fn put_from_iter(&mut self, pixels: impl Iterator<Item = Self::Format>);
    fn put_from_par_iter(
        &mut self,
        pixels: impl ParallelIterator<Item = Self::Format> + rayon::iter::IndexedParallelIterator,
    );
    fn as_formatted_raw(&mut self) -> &[u8];
}
#[derive(Debug, Clone)]
pub enum DynamicFrame {
    Rgba(RgbaFrame),
    Rgb(RgbFrame),
    Yuv420p(Yuv420pFrame),
}

// TODO (Etos2): Consider replacing get_pixel_checked()
impl DynamicFrame {
    pub fn from_image(format: PixelFormat, image: DynamicImage) -> DynamicFrame {
        match format {
            PixelFormat::Rgba => DynamicFrame::Rgba(image.to_rgba8().into()),
            PixelFormat::Rgb => DynamicFrame::Rgb(image.to_rgb8().into()),
            PixelFormat::Yuv420p => DynamicFrame::Yuv420p(image.to_rgb8().into()),
        }
    }

    pub fn from_pixel(
        format: PixelFormat,
        width: u32,
        height: u32,
        pixel: impl Pixel,
    ) -> DynamicFrame {
        match format {
            PixelFormat::Rgba => DynamicFrame::Rgba(RgbaFrame::from_pixel(width, height, pixel)),
            PixelFormat::Rgb => DynamicFrame::Rgb(RgbFrame::from_pixel(width, height, pixel)),
            PixelFormat::Yuv420p => {
                DynamicFrame::Yuv420p(Yuv420pFrame::from_pixel(width, height, pixel))
            }
        }
    }

    pub fn get_pixel_checked(&self, x: u32, y: u32) -> Option<Rgba> {
        match self {
            DynamicFrame::Rgba(frame) => frame.get_pixel_checked(x, y).copied(),
            DynamicFrame::Rgb(frame) => frame.get_pixel_checked(x, y).map(Rgb::to_rgba),
            DynamicFrame::Yuv420p(frame) => frame.get_pixel_checked(x, y).map(Rgb::to_rgba),
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            DynamicFrame::Rgba(frame) => frame.dimensions(),
            DynamicFrame::Rgb(frame) => frame.dimensions(),
            DynamicFrame::Yuv420p(frame) => frame.dimensions(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RgbaFrame {
    data: Vec<u8>,
    size: (u32, u32),
}

impl VideoFrame for RgbaFrame {
    type Format = Rgba;

    fn dimensions(&self) -> (u32, u32) {
        self.size
    }

    fn from_pixel(width: u32, height: u32, pixel: impl Pixel) -> Self {
        let frame_size = (width * height) as usize;
        RgbaFrame {
            data: pixel.to_rgba().0.repeat(frame_size),
            size: (width, height),
        }
    }

    fn get_pixel_checked(&self, x: u32, y: u32) -> Option<&Rgba> {
        let i = Self::Format::CHANNELS * get_index_checked(self.size, x, y)?;
        Some(Self::Format::from_slice(
            &self.data[i..i + Self::Format::CHANNELS],
        ))
    }

    fn put_pixel(&mut self, x: u32, y: u32, val: Self::Format) {
        let i = Self::Format::CHANNELS * get_index(self.size, x, y);
        let data = Self::Format::from_slice_mut(&mut self.data[i..i + Self::Format::CHANNELS]);
        *data = val;
    }

    fn put_from_iter(&mut self, pixels: impl Iterator<Item = Self::Format>) {
        self.data.clear();
        self.data.extend(pixels.flat_map(|val| val.0));
    }

    fn put_from_par_iter(&mut self, pixels: impl ParallelIterator<Item = Self::Format>) {
        self.data.clear();
        self.data.par_extend(pixels.flat_map(|val| val.0));
    }

    fn as_formatted_raw(&mut self) -> &[u8] {
        &self.data
    }
}

impl From<RgbaImage> for RgbaFrame {
    fn from(value: RgbaImage) -> Self {
        RgbaFrame {
            size: value.dimensions(),
            data: value.into_raw(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RgbFrame {
    data: Vec<u8>,
    size: (u32, u32),
}

impl VideoFrame for RgbFrame {
    type Format = Rgb;

    fn dimensions(&self) -> (u32, u32) {
        self.size
    }

    fn from_pixel(width: u32, height: u32, pixel: impl Pixel) -> Self {
        let frame_size = (width * height) as usize;
        RgbFrame {
            data: pixel.to_rgb().0.repeat(frame_size),
            size: (width, height),
        }
    }

    fn get_pixel_checked(&self, x: u32, y: u32) -> Option<&Self::Format> {
        let i = Self::Format::CHANNELS * get_index_checked(self.size, x, y)?;
        let rgb = Self::Format::from_slice(&self.data[i..i + Self::Format::CHANNELS]);
        Some(rgb)
    }

    fn put_pixel(&mut self, x: u32, y: u32, val: Self::Format) {
        let i = Self::Format::CHANNELS * get_index(self.size, x, y);
        let data = Self::Format::from_slice_mut(&mut self.data[i..i + Self::Format::CHANNELS]);
        *data = val;
    }

    fn put_from_iter(&mut self, pixels: impl Iterator<Item = Self::Format>) {
        self.data.clear();
        self.data.extend(pixels.flat_map(|val| val.0));
    }

    fn put_from_par_iter(&mut self, pixels: impl ParallelIterator<Item = Self::Format>) {
        self.data.clear();
        self.data.par_extend(pixels.flat_map(|val| val.0));
    }

    fn as_formatted_raw(&mut self) -> &[u8] {
        &self.data
    }
}

impl From<RgbImage> for RgbFrame {
    fn from(value: RgbImage) -> Self {
        RgbFrame {
            size: value.dimensions(),
            data: value.into_raw(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Yuv420pFrame {
    rgb_data: Vec<u8>,
    yuv_data: Vec<u8>,
    size: (u32, u32),
}

impl Yuv420pFrame {
    fn get_yuv_index(&self, x: u32, y: u32) -> (usize, usize, usize) {
        assert!(x < self.size.0);
        assert!(y < self.size.1);

        // Note (Etos2): x & !1 makes number even (e.g. 1 = 0, 2 = 2, 3 = 2)
        //               Will return top left index of 2x2 block
        let offset = get_index(self.size, x & !1, y & !1);
        let offset_color = get_index((self.size.0 / 2, self.size.1 / 2), x / 2, y / 2);
        let frame_size = (self.size.0 * self.size.1) as usize;
        (
            offset,
            frame_size + offset_color,
            frame_size + frame_size / 4 + offset_color,
        )
    }

    fn generate_yuv420p_block(&mut self, x: u32, y: u32) {
        let (mut y, u, v) = self.get_yuv_index(x, y);

        let r1 = self.rgb_data[Rgb::CHANNELS * y] as i32;
        let b1 = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
        let g1 = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
        let r2 = self.rgb_data[Rgb::CHANNELS * y + 3] as i32;
        let g2 = self.rgb_data[Rgb::CHANNELS * y + 4] as i32;
        let b2 = self.rgb_data[Rgb::CHANNELS * y + 5] as i32;
        self.yuv_data[y] = (((66 * r1 + 129 * g1 + 25 * b1) >> 8) + 16) as u8;
        self.yuv_data[y + 1] = (((66 * r2 + 129 * g2 + 25 * b2) >> 8) + 16) as u8;

        y += self.size.0 as usize;

        let r3 = self.rgb_data[Rgb::CHANNELS * y] as i32;
        let g3 = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
        let b3 = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
        let r4 = self.rgb_data[Rgb::CHANNELS * y + 3] as i32;
        let g4 = self.rgb_data[Rgb::CHANNELS * y + 4] as i32;
        let b4 = self.rgb_data[Rgb::CHANNELS * y + 5] as i32;
        self.yuv_data[y] = (((66 * r3 + 129 * g3 + 25 * b3) >> 8) + 16) as u8;
        self.yuv_data[y + 1] = (((66 * r4 + 129 * g4 + 25 * b4) >> 8) + 16) as u8;

        let r = ((r1 * r1 + r2 * r2 + r3 * r3 + r4 * r4) / 4).sqrt();
        let g = ((g1 * g1 + g2 * g2 + g3 * g3 + g4 * g4) / 4).sqrt();
        let b = ((b1 * b1 + b2 * b2 + b3 * b3 + b4 * b4) / 4).sqrt();

        self.yuv_data[u] = (((-38 * r + -74 * g + 112 * b) >> 8) + 128) as u8;
        self.yuv_data[v] = (((112 * r + -94 * g + -18 * b) >> 8) + 128) as u8;
    }

    fn generate_yuv420p(&mut self) {
        let frame_size = (self.size.0 * self.size.1) as usize;
        let mut y = 0;
        let mut u = frame_size;
        let mut v = frame_size + frame_size / 4;

        let mut uv_buf = vec![(0, 0); self.size.0 as usize / 2];

        for line in 0..self.size.1 {
            if line % 2 == 0 {
                for (u_buf, v_buff) in uv_buf.iter_mut().take(self.size.0 as usize / 2) {
                    let r = self.rgb_data[Rgb::CHANNELS * y] as i32;
                    let g = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
                    let b = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
                    self.yuv_data[y] = (((66 * r + 129 * g + 25 * b) >> 8) + 16) as u8;
                    y += 1;

                    let temp_u = ((-38 * r + -74 * g + 112 * b) >> 8) + 128;
                    let temp_v = ((112 * r + -94 * g + -18 * b) >> 8) + 128;
                    *u_buf = temp_u * temp_u;
                    *v_buff = temp_v * temp_v;

                    let r = self.rgb_data[Rgb::CHANNELS * y] as i32;
                    let g = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
                    let b = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
                    self.yuv_data[y] = (((66 * r + 129 * g + 25 * b) >> 8) + 16) as u8;
                    y += 1;

                    let temp_u = ((-38 * r + -74 * g + 112 * b) >> 8) + 128;
                    let temp_v = ((112 * r + -94 * g + -18 * b) >> 8) + 128;
                    *u_buf += temp_u * temp_u;
                    *v_buff += temp_v * temp_v;
                }
            } else {
                for (u_buf, v_buf) in uv_buf.iter_mut().take(self.size.0 as usize / 2) {
                    let r = self.rgb_data[Rgb::CHANNELS * y] as i32;
                    let g = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
                    let b = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
                    self.yuv_data[y] = (((66 * r + 129 * g + 25 * b) >> 8) + 16) as u8;
                    y += 1;

                    let temp_u = ((-38 * r + -74 * g + 112 * b) >> 8) + 128;
                    let temp_v = ((112 * r + -94 * g + -18 * b) >> 8) + 128;
                    *u_buf += temp_u * temp_u;
                    *v_buf += temp_v * temp_v;

                    let r = self.rgb_data[Rgb::CHANNELS * y] as i32;
                    let g = self.rgb_data[Rgb::CHANNELS * y + 1] as i32;
                    let b = self.rgb_data[Rgb::CHANNELS * y + 2] as i32;
                    self.yuv_data[y] = (((66 * r + 129 * g + 25 * b) >> 8) + 16) as u8;
                    y += 1;

                    let temp_u = ((-38 * r + -74 * g + 112 * b) >> 8) + 128;
                    let temp_v = ((112 * r + -94 * g + -18 * b) >> 8) + 128;
                    *u_buf += temp_u * temp_u;
                    *v_buf += temp_v * temp_v;

                    self.yuv_data[u] = (*u_buf / 4).sqrt() as u8;
                    self.yuv_data[v] = (*v_buf / 4).sqrt() as u8;
                    u += 1;
                    v += 1;
                }
            }
        }
    }

    // TODO (Etos2): Implement multithreaded alternative
    fn generate_yuv420p_par(&mut self) {
        self.generate_yuv420p();
    }
}

impl VideoFrame for Yuv420pFrame {
    type Format = Rgb;

    fn dimensions(&self) -> (u32, u32) {
        self.size
    }

    fn from_pixel(width: u32, height: u32, pixel: impl Pixel) -> Self {
        let frame_size = (width * height) as usize;

        let mut frame = Yuv420pFrame {
            rgb_data: pixel.to_rgb().0.repeat(frame_size),
            yuv_data: vec![0; frame_size + frame_size / 2],
            size: (width, height),
        };

        frame.generate_yuv420p_par();
        frame
    }

    fn get_pixel_checked(&self, x: u32, y: u32) -> Option<&Self::Format> {
        let i = Self::Format::CHANNELS * get_index_checked(self.size, x, y)?;
        let rgb = Self::Format::from_slice(&self.rgb_data[i..i + Self::Format::CHANNELS]);
        Some(rgb)
    }

    fn put_pixel(&mut self, x: u32, y: u32, val: Self::Format) {
        let i = Self::Format::CHANNELS * get_index(self.size, x, y);
        let data = Self::Format::from_slice_mut(&mut self.rgb_data[i..i + Self::Format::CHANNELS]);
        *data = val;

        self.generate_yuv420p_block(x, y);
    }

    fn put_from_iter(&mut self, pixels: impl Iterator<Item = Self::Format>) {
        self.rgb_data
            .chunks_exact_mut(3)
            .zip(pixels)
            .for_each(|(dst, src)| *Rgb::from_slice_mut(dst) = src);
        self.generate_yuv420p();
    }

    fn put_from_par_iter(
        &mut self,
        pixels: impl ParallelIterator<Item = Self::Format> + rayon::iter::IndexedParallelIterator,
    ) {
        self.rgb_data
            .par_chunks_exact_mut(3)
            .zip(pixels)
            .for_each(|(dst, src)| *Rgb::from_slice_mut(dst) = src);
        self.generate_yuv420p_par();
    }

    fn as_formatted_raw(&mut self) -> &[u8] {
        &self.yuv_data
    }
}

impl From<RgbImage> for Yuv420pFrame {
    fn from(value: RgbImage) -> Self {
        let (w, h) = value.dimensions();
        let frame_size = (w * h) as usize;
        let yuv_size = frame_size + frame_size / 2;
        let mut frame = Yuv420pFrame {
            size: value.dimensions(),
            rgb_data: value.into_raw(),
            yuv_data: vec![0; yuv_size],
        };

        frame.generate_yuv420p();
        frame
    }
}

fn get_index(bounds: (u32, u32), x: u32, y: u32) -> usize {
    assert!(x < bounds.0, "{:?} < {:?}", (x, y), bounds);
    assert!(y < bounds.1, "{:?} < {:?}", (x, y), bounds);
    (x + y * bounds.0) as usize
}

fn get_index_checked(bounds: (u32, u32), x: u32, y: u32) -> Option<usize> {
    if x < bounds.0 && y < bounds.1 {
        Some((x + y * bounds.0) as usize)
    } else {
        None
    }
}
