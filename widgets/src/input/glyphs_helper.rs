use std::ops::Range;

use ribir_core::prelude::*;

#[derive(Default)]
pub(crate) struct GlyphsHelper {
  pub(crate) glyphs: Option<VisualGlyphs>,
}

impl GlyphsHelper {
  pub(crate) fn cluster_from_pos(&self, x: f32, y: f32) -> u32 {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.nearest_glyph(x, y);
    return glyphs.position_to_cluster(para, offset);
  }

  pub(crate) fn cursor(&self, cursor: usize) -> (Point, f32) {
    let glyphs = self.glyphs.as_ref().unwrap();
    let (para, offset) = glyphs.position_by_cluster(cursor as u32);
    let (glphy, line_height) = glyphs.glyph_rect(para, offset);

    (Point::new(glphy.min_x(), glphy.max_y()), line_height)
  }

  pub(crate) fn selection(&self, (start, end): (usize, usize)) -> Vec<Rect> {
    if start == end {
      return vec![];
    }
    self
      .glyphs
      .as_ref()
      .unwrap()
      .select_range(Range { start, end })
      .iter()
      .map(|r| {
        Rect::new(
          Point::new(r.min_x().value(), r.min_y().value()),
          Size::new(r.width().value(), r.height().value()),
        )
      })
      .collect()
  }
}
