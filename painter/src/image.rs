use std::{borrow::Cow, fmt::Debug, hash::Hash, rc::Rc};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum ColorFormat {
  Rgba8,
}

impl ColorFormat {
  /// return have many bytes per pixel need
  pub const fn pixel_per_bytes(&self) -> u8 {
    match self {
      ColorFormat::Rgba8 => 4,
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct PixelImage {
  data: Cow<'static, [u8]>,
  size: (u16, u16),
  format: ColorFormat,
}

impl PixelImage {
  #[inline]
  pub fn new(data: Cow<'static, [u8]>, width: u16, height: u16, format: ColorFormat) -> Self {
    PixelImage { data, size: (width, height), format }
  }

  #[cfg(feature = "png")]
  pub fn from_png(bytes: &[u8]) -> Self {
    let img = ::image::load(std::io::Cursor::new(bytes), image::ImageFormat::Png)
      .unwrap()
      .to_rgba8();
    let width = img.width();
    let height = img.height();
    PixelImage::new(
      std::borrow::Cow::Owned(img.into_raw()),
      width as u16,
      height as u16,
      ColorFormat::Rgba8,
    )
  }
  #[inline]
  pub fn color_format(&self) -> ColorFormat { self.format }
  #[inline]
  pub fn size(&self) -> (u16, u16) { self.size }
  #[inline]
  pub fn pixel_bytes(&self) -> &[u8] { &self.data }
}

/// A image wrap for shallow compare.
#[derive(Clone, Serialize, Deserialize)]
pub struct ShallowImage(Rc<PixelImage>);

impl Hash for ShallowImage {
  #[inline]
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let ptr = Rc::as_ptr(&self.0);
    ptr.hash(state);
  }
}

impl PartialEq for ShallowImage {
  #[inline]
  fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
}

impl Eq for ShallowImage {}

impl Debug for ShallowImage {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let (width, height) = self.size;
    f.debug_tuple("ShallowImage")
      .field(&format!("{width}x{height}"))
      .finish()
  }
}

impl ShallowImage {
  #[inline]
  pub fn new(img: PixelImage) -> Self { Self(Rc::new(img)) }

  #[inline]
  #[cfg(feature = "png")]
  pub fn from_png(bytes: &[u8]) -> Self { ShallowImage::new(PixelImage::from_png(bytes)) }
}

impl std::ops::Deref for ShallowImage {
  type Target = Rc<PixelImage>;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.0 }
}
