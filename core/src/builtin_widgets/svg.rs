use crate::{impl_query_self_only, prelude::*};

impl Render for Svg {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size { self.size }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    let painter = ctx.painter();
    self.paths.iter().for_each(|c| {
      painter
        .apply_transform(&c.transform)
        .set_brush(c.brush.clone());
      match &c.style {
        PathPaintStyle::Fill => painter.fill_path(c.path.clone()),
        PathPaintStyle::Stroke(options) => painter
          .set_strokes(options.clone())
          .stroke_path(c.path.clone()),
      };
    });
  }
}

impl Query for Svg {
  impl_query_self_only!();
}
