use ribir_core::{impl_query_self_only, prelude::*};

use super::ConstrainedBox;

/// A widget that expanded a child of `Flex`, so that the child fills the
/// available space. If multiple children are expanded, the available space is
/// divided among them according to the flex factor.
#[derive(Clone, PartialEq, Declare, Declare2)]
pub struct Expanded {
  pub flex: f32,
}

impl ComposeChild for Expanded {
  type Child = Widget;
  #[inline]
  fn compose_child(this: State<Self>, child: Self::Child) -> Widget {
    let w = widget! {
      ConstrainedBox {
        clamp: BoxClamp {
          min: Size::new(0., 0.),
          max: Size::new(f32::INFINITY, f32::INFINITY)
        },
        DynWidget {
          dyns: child
        }
      }
    };
    DataWidget::attach_state(w.into(), this)
  }
}

impl_query_self_only!(Expanded);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::prelude::*;
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  fn expand_child_size_zero() -> Widget {
    let size = Size::new(100., 50.);
    widget! {
      Row {
        Expanded {
          flex: 1.,
          SizedBox { size }
        }
        SizedBox { size }
        Expanded {
          flex: 2.,
          SizedBox { size: Size::new(0., 50.) }
        }
      }
    }
    .into()
  }
  widget_layout_test!(
    expand_child_size_zero,
    wnd_size = Size::new(500., 500.),
    { path = [0, 0], width == 400., height == 50.,}
    { path = [0, 2], width == 0., height == 50.,}
  );

  fn one_line_expanded() -> Widget {
    let size = Size::new(100., 50.);
    widget! {
      Row {
        Expanded {
          flex: 1.,
          SizedBox { size }
        }
        SizedBox { size }
        SizedBox { size }
        Expanded {
          flex: 2.,
          SizedBox { size }
        }
      }
    }
    .into()
  }
  widget_layout_test!(
    one_line_expanded,
    wnd_size = Size::new(500., 500.),
    { path = [0], width == 500., height == 50.,}
    { path = [0, 0], width == 100., height == 50., }
    { path = [0, 1], rect == ribir_geom::rect(100., 0., 100., 50.),}
    { path = [0, 2], rect == ribir_geom::rect(200., 0., 100., 50.),}
    { path = [0, 3], rect == ribir_geom::rect(300., 0., 200., 50.),}
  );

  fn wrap_expanded() -> Widget {
    let size = Size::new(100., 50.);
    widget! {
      Row {
        wrap: true,
        Expanded {
          flex: 1. ,
          SizedBox { size }
        }
        SizedBox { size }
        SizedBox { size }
        SizedBox { size }
        SizedBox { size }
        Expanded {
          flex: 1. ,
          SizedBox { size, }
        }
        Expanded {
          flex: 4.,
          SizedBox { size, }
        }
      }
    }
    .into()
  }
  widget_layout_test!(
    wrap_expanded,
    wnd_size = Size::new(350., 500.),
    { path = [0], rect == ribir_geom::rect(0., 0., 350., 100.),}
    { path = [0, 0], rect == ribir_geom::rect(0., 0., 50., 50.),}
    { path = [0, 1], rect == ribir_geom::rect(50., 0., 100., 50.),}
    { path = [0, 2], rect == ribir_geom::rect(150., 0., 100., 50.),}
    { path = [0, 3], rect == ribir_geom::rect(250., 0., 100., 50.),}
    { path = [0, 4], rect == ribir_geom::rect(0., 50., 100., 50.),}
    { path = [0, 5], rect == ribir_geom::rect(100., 50., 50., 50.),}
    { path = [0, 6], rect == ribir_geom::rect(150., 50., 200., 50.),}
  );
}
