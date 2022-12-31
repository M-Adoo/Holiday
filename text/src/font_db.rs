use ribir_algo::FrameCache;
use fontdb::{Database, Query};
pub use fontdb::{FaceInfo, Family, ID};
use lyon_path::math::{Point, Transform};
use std::sync::Arc;
use ttf_parser::{GlyphId, OutlineBuilder};

use crate::{FontFace, FontFamily};

/// A wrapper of fontdb and cache font data.
pub struct FontDB {
  data_base: fontdb::Database,
  cache: FrameCache<ID, Option<Face>>,
}

#[derive(Clone)]
pub struct Face {
  pub face_id: ID,
  pub source_data: Arc<dyn AsRef<[u8]> + Sync + Send>,
  pub face_data_index: u32,
  pub rb_face: rustybuzz::Face<'static>,
}

impl FontDB {
  pub fn try_get_face_data(&self, face_id: ID) -> Option<&Face> {
    self.cache.get(&face_id)?.as_ref()
  }

  pub fn face_data_or_insert(&mut self, face_id: ID) -> Option<&Face> {
    self
      .cache
      .get_or_insert_with(&face_id, || {
        self
          .data_base
          .face_source(face_id)
          .and_then(|(src, face_index)| {
            let source_data = match src {
              fontdb::Source::Binary(data) => Some(data),
              fontdb::Source::File(_) => {
                let mut source_data = None;
                self.data_base.with_face_data(face_id, |data, index| {
                  assert_eq!(face_index, index);
                  let data: Arc<dyn AsRef<[u8]> + Sync + Send> = Arc::new(data.to_owned());
                  source_data = Some(data);
                });
                source_data
              }
              fontdb::Source::SharedFile(_, data) => Some(data),
            }?;
            Face::from_data(face_id, source_data, face_index)
          })
      })
      .as_ref()
  }

  /// Selects a `FaceInfo` by `id`.
  ///
  /// Returns `None` if a face with such ID was already removed,
  /// or this ID belong to the other `Database`.
  #[inline]
  pub fn face_info(&self, id: ID) -> Option<&FaceInfo> { self.data_base.face(id) }

  /// Returns a reference to an internal storage.
  ///
  /// This can be used for manual font matching.
  #[inline]
  pub fn faces(&self) -> &[FaceInfo] { self.data_base.faces() }

  #[inline]
  pub fn load_from_bytes(&mut self, data: Vec<u8>) { self.data_base.load_font_data(data); }

  /// Loads a font file into the `Database`.
  ///
  /// Will load all font faces in case of a font collection.
  #[inline]
  pub fn load_font_file<P: AsRef<std::path::Path>>(
    &mut self,
    path: P,
  ) -> Result<(), std::io::Error> {
    self.data_base.load_font_file(path)
  }

  /// Attempts to load system fonts.
  ///
  /// Supports Windows, Linux and macOS.
  ///
  /// System fonts loading is a surprisingly complicated task,
  /// mostly unsolvable without interacting with system libraries.
  /// And since `fontdb` tries to be small and portable, this method
  /// will simply scan some predefined directories.
  /// Which means that fonts that are not in those directories must
  /// be added manually.
  pub fn load_system_fonts(&mut self) {
    self.data_base.load_system_fonts();
    self.static_generic_families();
  }

  /// Performs a CSS-like query and returns the best matched font face id.
  pub fn select_best_match(&self, face: &FontFace) -> Option<ID> {
    let FontFace { families, stretch, style, weight } = face;
    let families = families.iter().map(to_db_family).collect::<Vec<_>>();
    self.data_base.query(&Query {
      families: &families,
      weight: *weight,
      stretch: *stretch,
      style: *style,
    })
  }

  /// Performs a CSS-like query and returns the all matched font face ids
  pub fn select_all_match(&self, face: &FontFace) -> Vec<ID> {
    let FontFace { families, stretch, style, weight } = face;
    families
      .iter()
      .filter_map(|f| {
        self.data_base.query(&Query {
          families: &[to_db_family(f)],
          weight: *weight,
          stretch: *stretch,
          style: *style,
        })
      })
      .collect()
  }

  pub fn end_frame(&mut self) { self.cache.end_frame("Font DB") }

  fn static_generic_families(&mut self) {
    // We don't like to depends on some system library and not make the fallback
    // font too complicated. So here are some default fonts collect from web.
    let init_data: [(&[Family], _); 5] = [
      (
        &[
          #[cfg(any(target_os = "windows", target_os = "linux", target_os = "ios"))]
          Family::Name("Times New Roman"),
          #[cfg(target_os = "macos")]
          Family::Name("Times"),
          #[cfg(target_os = "linux")]
          Family::Name("Free Serif"),
          #[cfg(any(target_os = "linux", target_os = "android"))]
          Family::Name("Noto Serif"),
        ],
        Database::set_serif_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "windows")]
          Family::Name("Segoe UI"),
          #[cfg(target_os = "windows")]
          Family::Name("Tahoma"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("San Francisco"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Helvetica"),
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Helvetica Neue"),
          #[cfg(target_os = "android")]
          Family::Name("Roboto"),
          #[cfg(target_os = "android")]
          Family::Name("Droid Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Ubuntu"),
          #[cfg(target_os = "linux")]
          Family::Name("Red Hat"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Sans"),
          #[cfg(target_os = "linux")]
          Family::Name("Liberation Sans"),
        ],
        Database::set_sans_serif_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "macos")]
          Family::Name("Apple Chancery"),
          #[cfg(target_os = "ios")]
          Family::Name("Snell Roundhand"),
          #[cfg(target_os = "windows")]
          Family::Name("Comic Sans MS"),
          #[cfg(target_os = "android")]
          Family::Name("Dancing Script"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Serif"),
        ],
        Database::set_cursive_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(any(target_os = "macos", target_os = "ios"))]
          Family::Name("Papyrus"),
          #[cfg(target_os = "windows")]
          Family::Name("Microsoft Sans Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("Free Serif"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Serif"),
          #[cfg(any(target_os = "linux", target_os = "android"))]
          Family::Name("Noto Serif"),
        ],
        Database::set_fantasy_family as fn(&mut Database, String),
      ),
      (
        &[
          #[cfg(target_os = "macos")]
          Family::Name("Andale Mono"),
          #[cfg(target_os = "ios")]
          Family::Name("Courier"),
          #[cfg(target_os = "windows")]
          Family::Name("Courier New"),
          #[cfg(target_os = "android")]
          Family::Name("Droid Sans Mono"),
          #[cfg(target_os = "linux")]
          Family::Name("DejaVu Sans Mono"),
          #[cfg(target_os = "linux")]
          Family::Name("Noto Sans Mono"),
        ],
        Database::set_monospace_family as fn(&mut Database, String),
      ),
    ];

    init_data.iter().for_each(|(families, set_fn)| {
      let name = self
        .data_base
        .query(&Query {
          families: *families,
          ..<_>::default()
        })
        .and_then(|id| self.data_base.face(id))
        .map(|f| f.family.clone());
      if let Some(name) = name {
        set_fn(&mut self.data_base, name);
      }
    });
  }
}

impl Default for FontDB {
  fn default() -> FontDB {
    FontDB {
      data_base: fontdb::Database::new(),
      cache: <_>::default(),
    }
  }
}

impl Face {
  pub fn from_data(
    face_id: ID,
    source_data: Arc<dyn AsRef<[u8]> + Sync + Send>,
    face_index: u32,
  ) -> Option<Self> {
    let ptr_data = source_data.as_ref().as_ref() as *const [u8];
    // Safety: we know the ptr_data has some valid lifetime with source data, and
    // hold them in same struct.
    let rb_face = rustybuzz::Face::from_slice(unsafe { &*ptr_data }, face_index)?;
    Some(Face {
      source_data,
      face_data_index: face_index,
      rb_face,
      face_id,
    })
  }

  #[inline]
  pub fn has_char(&self, c: char) -> bool { self.rb_face.as_ref().glyph_index(c).is_some() }

  pub fn as_rb_face(&self) -> &rustybuzz::Face { &self.rb_face }

  pub fn outline_glyph(&self, glyph_id: GlyphId) -> Option<lyon_path::Path> {
    let mut builder = GlyphOutlineBuilder::default();
    self
      .rb_face
      .outline_glyph(glyph_id, &mut builder as &mut dyn OutlineBuilder)?;

    // By default, outlie glyphs is an mirror.
    let units_per_em = self.units_per_em() as f32;
    let mirror =
      Transform::scale(1. / units_per_em, -1. / units_per_em).then_translate((0., 1.).into());
    Some(builder.into_path().transformed(&mirror))
  }

  #[inline]
  pub fn units_per_em(&self) -> i32 { self.rb_face.units_per_em() }
}

fn to_db_family(f: &FontFamily) -> Family {
  match f {
    FontFamily::Name(name) => Family::Name(name),
    FontFamily::Serif => Family::Serif,
    FontFamily::SansSerif => Family::SansSerif,
    FontFamily::Cursive => Family::Cursive,
    FontFamily::Fantasy => Family::Fantasy,
    FontFamily::Monospace => Family::Monospace,
  }
}

#[derive(Default)]
struct GlyphOutlineBuilder {
  builder: lyon_path::path::Builder,
  closed: bool,
}

impl GlyphOutlineBuilder {
  fn into_path(mut self) -> lyon_path::Path {
    if !self.closed {
      self.builder.end(false);
    }
    self.builder.build()
  }
}

impl OutlineBuilder for GlyphOutlineBuilder {
  fn move_to(&mut self, x: f32, y: f32) {
    self.closed = false;
    self.builder.begin(Point::new(x, y));
  }

  fn line_to(&mut self, x: f32, y: f32) {
    self.closed = false;
    self.builder.line_to(Point::new(x, y));
  }

  fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
    self.closed = false;
    self
      .builder
      .quadratic_bezier_to(Point::new(x1, y1), Point::new(x, y));
  }

  fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
    self.closed = false;
    self
      .builder
      .cubic_bezier_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x, y));
  }

  fn close(&mut self) {
    if !self.closed {
      self.closed = true;
      self.builder.close()
    }
  }
}

impl std::ops::Deref for Face {
  type Target = ttf_parser::Face<'static>;

  #[inline]
  fn deref(&self) -> &Self::Target { &*self.rb_face }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::FontWeight;

  #[test]
  fn load_font_from_path() {
    let mut db = FontDB::default();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    db.load_font_file(path).unwrap();
    let face_id = db.select_best_match(&FontFace {
      families: vec![FontFamily::Name("DejaVu Sans".into())].into_boxed_slice(),
      ..<_>::default()
    });
    assert!(face_id.is_some());

    let info = db.face_info(face_id.unwrap()).unwrap();

    assert_eq!(info.family, "DejaVu Sans");
  }

  #[test]
  fn load_font_from_bytes() {
    let mut db = FontDB::default();
    let bytes = include_bytes!("../../fonts/GaramondNo8-Reg.ttf");
    db.load_from_bytes(bytes.to_vec());

    let face_id = db.select_best_match(&FontFace {
      families: vec![FontFamily::Name("GaramondNo8".into())].into_boxed_slice(),
      ..<_>::default()
    });
    assert!(face_id.is_some());
  }

  #[test]
  fn load_sys_fonts() {
    let mut db = FontDB::default();
    db.load_system_fonts();
    assert!(!db.faces().is_empty())
  }

  #[test]
  fn match_font() {
    let mut fonts = FontDB::default();
    fonts.load_system_fonts();
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + "/../fonts/DejaVuSans.ttf";
    let _ = fonts.load_font_file(path);

    let mut face = FontFace {
      families: vec![
        FontFamily::Name("DejaVu Sans".into()),
        FontFamily::SansSerif,
      ]
      .into_boxed_slice(),
      ..<_>::default()
    };

    // match custom load fonts.
    let id = fonts.select_best_match(&face).unwrap();
    let info = fonts.face_info(id).unwrap();
    assert_eq!(info.family, "DejaVu Sans");
    fonts.data_base.remove_face(id);

    face.weight = FontWeight::BOLD;

    let id = fonts.select_best_match(&face);
    assert!(id.is_some());
    let info = fonts.face_info(id.unwrap()).unwrap();
    assert_eq!(info.weight, FontWeight::BOLD);
  }
}
