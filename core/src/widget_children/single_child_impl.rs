use super::*;
use crate::widget::{StrictBuilder, WidgetBuilder};

/// Trait specify what child a widget can have, and the target type is the
/// result of widget compose its child.
pub trait SingleWithChild<C> {
  type Target;
  fn with_child(self, child: C, ctx: &BuildCtx) -> Self::Target;
}

/// A node of widget with not compose its child.
pub struct SinglePair<W, C> {
  pub(crate) widget: W,
  pub(crate) child: C,
}

impl<W, C> SinglePair<W, C> {
  #[inline]
  pub fn unzip(self) -> (W, C) {
    let Self { widget, child } = self;
    (widget, child)
  }
  #[inline]
  pub fn child(self) -> C { self.child }
  #[inline]
  pub fn parent(self) -> W { self.widget }
}

impl<W: SingleChild> SingleChild for Option<W> {}

impl<W: SingleChild, C> SingleWithChild<C> for W {
  type Target = SinglePair<W, C>;

  #[inline]
  fn with_child(self, child: C, _: &BuildCtx) -> Self::Target { SinglePair { widget: self, child } }
}

impl<W, C1: SingleChild, C2> SingleWithChild<C2> for SinglePair<W, C1> {
  type Target = SinglePair<W, SinglePair<C1, C2>>;

  fn with_child(self, c: C2, ctx: &BuildCtx) -> Self::Target {
    let SinglePair { widget, child } = self;
    SinglePair {
      widget,
      child: child.with_child(c, ctx),
    }
  }
}

impl<W, C> StrictBuilder for SinglePair<W, C>
where
  W: SingleParent,
  C: WidgetBuilder,
{
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    let child = child.build(ctx);
    widget.append_child(child, ctx)
  }
}

impl<W, C> StrictBuilder for SinglePair<Option<W>, C>
where
  W: SingleParent,
  C: WidgetBuilder,
{
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      SinglePair { widget, child }.strict_build(ctx)
    } else {
      child.build(ctx)
    }
  }
}

impl<W, C> StrictBuilder for SinglePair<W, Option<C>>
where
  W: SingleParent + WidgetBuilder,
  SinglePair<W, C>: StrictBuilder,
{
  fn strict_build(self, ctx: &BuildCtx) -> WidgetId {
    let Self { widget, child } = self;
    if let Some(child) = child {
      SinglePair { widget, child }.strict_build(ctx)
    } else {
      widget.build(ctx)
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::test_helper::MockBox;

  use super::*;

  #[test]
  fn pair_with_child() {
    let mock_box = MockBox { size: ZERO_SIZE };
    let _ = FnWidget::new(|ctx| {
      mock_box
        .clone()
        .with_child(mock_box.clone(), ctx)
        .with_child(mock_box, ctx)
        .strict_build(ctx)
    });
  }
}
