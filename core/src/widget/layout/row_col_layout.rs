use super::flex::{Axis, FlexContainer};
use crate::render::default_box_impl;

use crate::prelude::*;
use crate::render::render_ctx::RenderCtx;
use crate::render::render_tree::*;
use crate::render::LayoutConstraints;

///  a stupid implement for develope the framework.
#[derive(Debug)]
pub struct RowColumn {
  axis: Axis,
  children: Vec<Box<dyn Widget>>,
}

impl RowColumn {
  pub fn column(children: Vec<Box<dyn Widget>>) -> RowColumn {
    RowColumn {
      axis: Axis::Vertical,
      children,
    }
  }

  pub fn row(children: Vec<Box<dyn Widget>>) -> RowColumn {
    RowColumn {
      axis: Axis::Horizontal,
      children,
    }
  }
}

impl Widget for RowColumn {
  multi_child_widget_base_impl!();
}

#[derive(Debug)]
pub struct RowColRender {
  pub flex: FlexContainer,
}

impl RenderWidget for RowColumn {
  type RO = RowColRender;
  fn create_render_object(&self) -> Self::RO {
    RowColRender {
      flex: FlexContainer::new(self.axis, LayoutConstraints::EFFECTED_BY_CHILDREN),
    }
  }
}

impl MultiChildWidget for RowColumn {
  fn take_children(&mut self) -> Vec<Box<dyn Widget>> { std::mem::take(&mut self.children) }
}

impl RenderObject for RowColRender {
  type Owner = RowColumn;
  fn update(&mut self, _owner_widget: &RowColumn) {}

  fn perform_layout(&mut self, id: RenderId, ctx: &mut RenderCtx) -> Size {
    let size = self.flex.flex_layout(id, ctx);
    ctx.update_size(id, size);
    size
  }
  #[inline]
  fn paint<'b>(&'b self, _ctx: &mut PaintingContext<'b>) {}

  default_box_impl!({ flex.bound });
}
