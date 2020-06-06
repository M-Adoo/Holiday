use super::{
  Color, Command, CommandInfo, FillStyle, GlyphStatistics, HorizontalAlign, RenderAttr,
  RenderCommand, Rendering2DLayer, Text, TextLayout, Transform, Vertex, VerticalAlign,
};
use crate::{
  canvas::{surface::Surface, Canvas},
  text_brush::Section,
  Point, Rect, Size,
};
pub use lyon::{
  path::{builder::PathBuilder, traits::PathIterator, Path, Winding},
  tessellation::*,
};

const TOLERANCE: f32 = 0.5;

pub(crate) struct ProcessLayer2d<'a, S: Surface> {
  stroke_tess: StrokeTessellator,
  fill_tess: FillTessellator,
  geometry: VertexBuffers<Vertex, u32>,
  attrs: Vec<RenderAttr>,
  unready_text_attrs: Vec<UnReadyTextAttr>,
  texture_updated: bool,
  queued_text: bool,
  canvas: &'a mut Canvas<S>,
}

struct UnReadyTextAttr {
  count: GlyphStatistics,
  transform: Transform,
  style: FillStyle,
  align_bounds: Rect,
}

impl<'a, S: Surface> ProcessLayer2d<'a, S> {
  pub(crate) fn new(canvas: &'a mut Canvas<S>) -> Self {
    Self {
      stroke_tess: <_>::default(),
      fill_tess: FillTessellator::new(),
      geometry: VertexBuffers::new(),
      attrs: vec![],
      unready_text_attrs: vec![],
      texture_updated: false,
      queued_text: false,
      canvas,
    }
  }

  pub(crate) fn process_layer(mut self, layer: Rendering2DLayer<'a>) -> Option<RenderCommand> {
    layer
      .commands
      .into_iter()
      .for_each(|Command { transform, info }| {
        match info {
          CommandInfo::Path {
            path,
            style,
            stroke_width,
          } => {
            self.tessellate_path(path, style, stroke_width, transform);
          }
          CommandInfo::SimpleText {
            text,
            style,
            max_width,
          } => {
            self.queue_simple_text(text, style, max_width, transform);
          }
          CommandInfo::ComplexTexts {
            texts,
            bounds,
            layout,
          } => {
            self.queue_complex_texts(texts, transform, bounds, layout);
          }
          CommandInfo::ComplexTextsByStyle {
            style,
            texts,
            bounds,
            layout,
          } => {
            self.queue_complex_texts_by_style(texts, transform, style, bounds, layout);
          }
        };
      });

    self.process_queued_text();

    let cmd = RenderCommand {
      geometry: self.geometry,
      attrs: self.attrs,
    };
    if self.texture_updated {
      self.canvas.upload_render_command(&cmd);
      None
    } else {
      Some(cmd)
    }
  }

  fn process_queued_text(&mut self) {
    if !self.queued_text {
      return;
    }

    let Self {
      canvas, geometry, ..
    } = self;

    canvas.process_queued(geometry);

    self.queued_text = false;
    self.process_text_attrs();
  }

  fn tessellate_path(
    &mut self,
    path: Path,
    style: FillStyle,
    stroke_width: Option<f32>,
    transform: Transform,
  ) {
    // ensure all queued text has be processed.
    self.process_queued_text();

    let count = if let Some(line_width) = stroke_width {
      self
        .stroke_tess
        .tessellate_path(
          &path,
          &StrokeOptions::tolerance(TOLERANCE).with_line_width(line_width),
          &mut BuffersBuilder::new(&mut self.geometry, Vertex::from_stroke_vertex),
        )
        .unwrap()
    } else {
      self
        .fill_tess
        .tessellate_path(
          &path,
          &FillOptions::tolerance(TOLERANCE),
          &mut BuffersBuilder::new(&mut self.geometry, Vertex::from_fill_vertex),
        )
        .unwrap()
    };
    let bounds = path_bounds_to_align_texture(&style, &path);
    self.add_attr_and_try_merge(count, transform, style, bounds);
  }

  fn queue_simple_text(
    &mut self,
    text: Text<'a>,
    style: FillStyle,
    max_width: Option<f32>,
    transform: Transform,
  ) {
    let text = text.to_glyph_text(self.canvas);
    let count = text.extra.clone();
    let mut sec = Section::new().add_text(text);
    if let Some(max_width) = max_width {
      sec.bounds = (max_width, f32::INFINITY).into()
    }
    let align_bounds = section_bounds_to_align_texture(self.canvas, &style, &sec);
    if !align_bounds.is_empty_or_negative() {
      self.unready_text_attrs.push(UnReadyTextAttr {
        count,
        transform,
        align_bounds,
        style,
      });
      self.queue_section(&sec);
    }
  }
  fn queue_complex_texts(
    &mut self,
    texts: Vec<(Text<'a>, Color)>,
    transform: Transform,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let (texts, mut attrs) = texts
      .into_iter()
      .map(|(t, color)| {
        let text = t.to_glyph_text(self.canvas);
        let attr = UnReadyTextAttr {
          count: text.extra.clone(),
          transform,
          align_bounds: COLOR_BOUNDS_TO_ALIGN_TEXTURE,
          style: FillStyle::Color(color),
        };
        (text, attr)
      })
      .unzip();
    self.unready_text_attrs.append(&mut attrs);
    let mut sec = Section::new().with_text(texts);
    sec = section_with_layout_bounds(sec, bounds, layout);

    self.queue_section(&sec);
  }

  fn queue_complex_texts_by_style(
    &mut self,
    texts: Vec<Text<'a>>,
    transform: Transform,
    style: FillStyle,
    bounds: Option<Rect>,
    layout: Option<TextLayout>,
  ) {
    let texts = texts
      .into_iter()
      .map(|t| t.to_glyph_text(self.canvas))
      .collect();
    let mut sec = Section::new().with_text(texts);
    let align_bounds = section_bounds_to_align_texture(self.canvas, &style, &sec);
    if !align_bounds.is_empty_or_negative() {
      sec = section_with_layout_bounds(sec, bounds, layout);

      let attrs = sec.text.iter().map(|t| UnReadyTextAttr {
        count: t.extra.clone(),
        transform,
        align_bounds,
        style: style.clone(),
      });
      self.unready_text_attrs.extend(attrs);
      self.queue_section(&sec);
    }
  }

  fn process_text_attrs(&mut self) {
    let Self {
      attrs: render_attrs,
      unready_text_attrs,
      ..
    } = self;

    let attrs = unready_text_attrs.drain(..).map(
      |UnReadyTextAttr {
         transform,
         style,
         count,
         align_bounds,
       }| {
        RenderAttr {
          transform,
          style,
          align_bounds,
          count: count.into(),
        }
      },
    );
    render_attrs.extend(attrs);
  }

  fn queue_section(&mut self, sec: &Section) {
    self.canvas.queue(&sec);
    self.queued_text = true;
  }

  fn add_attr_and_try_merge(
    &mut self,
    count: Count,
    transform: Transform,
    style: FillStyle,
    bounds: Rect,
  ) {
    if let Some(last) = self.attrs.last_mut() {
      if last.align_bounds == bounds && last.style == style && last.transform == transform {
        last.count.vertices += count.vertices;
        last.count.indices += count.indices;
        return;
      }
    }

    self.attrs.push(RenderAttr {
      transform,
      align_bounds: bounds,
      count,
      style: style.clone(),
    });
  }
}

fn section_with_layout_bounds(
  mut sec: Section,
  bounds: Option<Rect>,
  layout: Option<TextLayout>,
) -> Section {
  if let Some(layout) = layout {
    sec = sec.with_layout(layout);
  }
  if let Some(bounds) = bounds {
    sec = section_with_bounds(sec, bounds);
  }
  sec
}

fn section_with_bounds(mut sec: Section, bounds: Rect) -> Section {
  sec = sec.with_bounds(bounds.size);

  let (h_align, v_align) = match &sec.layout {
    glyph_brush::Layout::SingleLine {
      h_align, v_align, ..
    } => (h_align, v_align),
    glyph_brush::Layout::Wrap {
      h_align, v_align, ..
    } => (h_align, v_align),
  };

  let mut pos = bounds.min();
  match h_align {
    HorizontalAlign::Left => {}
    HorizontalAlign::Center => pos.x = bounds.center().x,
    HorizontalAlign::Right => pos.x = bounds.max_x(),
  }
  match v_align {
    VerticalAlign::Top => {}
    VerticalAlign::Center => pos.y = bounds.center().y,
    VerticalAlign::Bottom => pos.y = bounds.max_y(),
  }
  sec.with_screen_position(pos)
}

// Pure color just one pixel in texture, and always use repeat pattern, so
// zero min is ok, no matter what really bounding it is.
const COLOR_BOUNDS_TO_ALIGN_TEXTURE: Rect = Rect::new(Point::new(0., 0.), Size::new(1., 1.));

fn path_bounds_to_align_texture(style: &FillStyle, path: &Path) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    let rect = lyon::algorithms::aabb::bounding_rect(path.iter());
    Rect::from_untyped(&rect)
  }
}

fn section_bounds_to_align_texture<S: Surface>(
  canvas: &mut Canvas<S>,
  style: &FillStyle,
  sec: &Section,
) -> Rect {
  if let FillStyle::Color(_) = style {
    COLOR_BOUNDS_TO_ALIGN_TEXTURE
  } else {
    canvas
      .glyph_brush
      .section_bounds(sec)
      .unwrap_or(Rect::zero())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bounding_align() {
    let mut path = Path::builder();
    path.add_rectangle(&lyon::geom::rect(100., 100., 50., 50.), Winding::Positive);
    let path = path.build();

    let rect = path_bounds_to_align_texture(&FillStyle::Color(Color::BLACK), &path);
    assert_eq!(rect, Rect::from_size(Size::new(1., 1.)));

    let rect = path_bounds_to_align_texture(&FillStyle::Image, &path);
    assert_eq!(rect.min(), Point::new(100., 100.));
    assert_eq!(rect.size, Size::new(50., 50.));
  }

  #[test]
  fn section_bounds_layout() {
    let section = Section::new();
    let rect = euclid::rect(10., 20., 40., 30.);
    let layout = TextLayout::default();

    let l = layout.clone();
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    assert_eq!(s.screen_position, rect.min().into());
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.center().x, rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.h_align = HorizontalAlign::Right;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.max_x(), rect.min().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.v_align = VerticalAlign::Center;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.center().y);
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());

    let mut l = layout.clone();
    l.v_align = VerticalAlign::Bottom;
    let s = section_with_layout_bounds(section.clone(), Some(rect), Some(l));
    let pos = (rect.min().x, rect.max_y());
    assert_eq!(s.screen_position, pos);
    assert_eq!(s.bounds, rect.size.into());
  }
}
