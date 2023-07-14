use image::ColorType;

pub trait Pixel: From<[u8; 4]> + From<Rgba> {
    const TYPE: ColorType;
    const CHANNELS: usize;
    fn to_rgb(&self) -> Rgb;
    fn to_rgba(&self) -> Rgba;
    fn from_slice(val: &[u8]) -> &Self;
    fn from_slice_mut(val: &mut [u8]) -> &mut Self;
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rgb(pub [u8; 3]);
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rgba(pub [u8; 4]);

impl Pixel for Rgba {
    const TYPE: ColorType = ColorType::Rgba8;
    const CHANNELS: usize = 4;

    fn to_rgb(&self) -> Rgb {
        let [r, g, b, _] = self.0;
        Rgb([r, g, b])
    }

    fn to_rgba(&self) -> Rgba {
        *self
    }

    fn from_slice(val: &[u8]) -> &Self {
        assert_eq!(val.len(), Self::CHANNELS);
        unsafe { &*(val.as_ptr() as *const Rgba) }
    }

    fn from_slice_mut(val: &mut [u8]) -> &mut Self {
        assert_eq!(val.len(), Self::CHANNELS);
        unsafe { &mut *(val.as_ptr() as *mut Rgba) }
    }
}

impl From<[u8; 4]> for Rgba {
    fn from(value: [u8; 4]) -> Self {
        Rgba(value)
    }
}

impl From<Rgba> for image::Rgba<u8> {
    fn from(value: Rgba) -> Self {
        image::Rgba(value.0)
    }
}
impl From<Rgba> for image::Rgb<u8> {
    fn from(value: Rgba) -> Self {
        image::Rgb(value.to_rgb().0)
    }
}

impl Pixel for Rgb {
    const TYPE: ColorType = ColorType::Rgb8;
    const CHANNELS: usize = 3;

    fn to_rgb(&self) -> Rgb {
        *self
    }

    fn to_rgba(&self) -> Rgba {
        let [r, g, b] = self.0;
        Rgba([r, g, b, 255])
    }

    fn from_slice(val: &[u8]) -> &Self {
        assert_eq!(val.len(), Self::CHANNELS);
        unsafe { &*(val.as_ptr() as *const Rgb) }
    }

    fn from_slice_mut(val: &mut [u8]) -> &mut Self {
        assert_eq!(val.len(), Self::CHANNELS);
        unsafe { &mut *(val.as_ptr() as *mut Rgb) }
    }
}

impl From<[u8; 4]> for Rgb {
    fn from(value: [u8; 4]) -> Self {
        *Rgb::from_slice(&value[..Self::CHANNELS])
    }
}

impl From<Rgb> for image::Rgb<u8> {
    fn from(value: Rgb) -> Self {
        image::Rgb(value.0)
    }
}
impl From<Rgb> for image::Rgba<u8> {
    fn from(value: Rgb) -> Self {
        image::Rgba(value.to_rgba().0)
    }
}

impl From<Rgba> for Rgb {
    fn from(value: Rgba) -> Self {
        value.0.into()
    }
}