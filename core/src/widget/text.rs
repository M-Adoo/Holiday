use ::text::ArcStr;

use crate::prelude::*;

/// The text widget display text with a single style.
#[derive(Debug, Declare, Clone, PartialEq)]
pub struct Text {
  pub text: ArcStr,
  #[declare(default = "ctx.theme().typography_theme.body1.text.clone()")]
  pub style: TextStyle,
}

impl RenderWidget for Text {
  fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let shaper = ctx.text_shaper();
    let ids = shaper.font_db_mut().select_all_match(&self.style.font_face);
    let reorder = ctx.text_reorder();
    // let info = reorder.reorder_text(&self.text.substr(..));
    // let glyphs = shaper.shape_text(&self.text, &ids);
    // ::text::layouter::glyphs_box(&self.text, &glyphs, self.style.font_size,
    // None, 0.).cast_unit()
    todo!();
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { false }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    // tod: fill_text should have a bounds
    // ctx.painter().fill_text(self.text.clone());
  }
}
