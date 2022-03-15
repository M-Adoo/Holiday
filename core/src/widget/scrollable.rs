use crate::prelude::*;

/// A widget let its child horizontal scrollable and the scroll view is as large
/// as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableX {
  pos: f32,
}

/// A widget let its child vertical scrollable and the scroll view is as large
/// as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableY {
  pos: f32,
}

/// A widget let its child both scrollable in horizontal and vertical, and the
/// scroll view is as large as its parent allow.
#[derive(SingleChildWidget, Default, Clone, PartialEq)]
pub struct ScrollableBoth {
  pos: Point,
}

impl ScrollableX {
  #[inline]
  pub fn x_scroll(pos: f32) -> Stateful<ScrollableX> {
    let scroll = ScrollableX { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = validate_pos(view.width(), content.width(), old - event.delta_x);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.pos = new;
      }
    })
  }
}

impl ScrollableY {
  #[inline]
  pub fn y_scroll(pos: f32) -> Stateful<ScrollableY> {
    let scroll = ScrollableY { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = validate_pos(view.height(), content.height(), old - event.delta_y);
      if (new - old).abs() > f32::EPSILON {
        scroll_ref.pos = new;
      }
    })
  }
}

impl ScrollableBoth {
  #[inline]
  pub fn both_scroll(pos: Point) -> Stateful<ScrollableBoth> {
    let scroll = ScrollableBoth { pos }.into_stateful();
    let mut scroll_ref = unsafe { scroll.state_ref() };
    scroll.on_wheel(move |event| {
      let (view, content) = view_content(event);
      let old = scroll_ref.pos;
      let new = Point::new(
        validate_pos(view.width(), content.width(), old.x - event.delta_x),
        validate_pos(view.height(), content.height(), old.y - event.delta_y),
      );
      if new != old {
        scroll_ref.pos = new;
      }
    })
  }
}

macro scroll_render_widget_impl($widget: ty, $state: ty) {
  impl RenderWidget for $widget {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
      let size = clamp.max;
      if let Some(child) = ctx.single_child() {
        let content_clamp = self.content_clamp(clamp);
        let content = ctx.perform_render_child_layout(child, content_clamp);
        let pos = self.content_pos(content, &size);
        ctx.update_position(child, pos);
      }

      size
    }

    fn only_sized_by_parent(&self) -> bool { true }

    fn paint(&self, _: &mut PaintingCtx) {}
  }
}

scroll_render_widget_impl!(ScrollableX, ScrollableXState);
scroll_render_widget_impl!(ScrollableY, ScrollableYState);
scroll_render_widget_impl!(ScrollableBoth, ScrollableBothState);

#[inline]
fn validate_pos(view: f32, content: f32, pos: f32) -> f32 { pos.min(0.).max(view - content) }

pub trait ScrollWorker {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp;

  fn content_pos(&self, content: Size, view: &Size) -> Point;
}

impl ScrollWorker for ScrollableX {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    let min = Size::zero();
    let mut max = clamp.max;
    max.width = f32::MAX;

    BoxClamp { min, max }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(validate_pos(view.width, content.width, self.pos), 0.)
  }
}

impl ScrollWorker for ScrollableY {
  fn content_clamp(&self, clamp: BoxClamp) -> BoxClamp {
    let min = Size::zero();
    let mut max = clamp.max;
    max.height = f32::MAX;

    BoxClamp { min, max }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(0., validate_pos(view.height, content.height, self.pos))
  }
}

impl ScrollWorker for ScrollableBoth {
  fn content_clamp(&self, _: BoxClamp) -> BoxClamp {
    BoxClamp {
      min: Size::zero(),
      max: Size::new(f32::MAX, f32::MAX),
    }
  }

  fn content_pos(&self, content: Size, view: &Size) -> Point {
    Point::new(
      validate_pos(view.width, content.width, self.pos.x),
      validate_pos(view.height, content.height, self.pos.y),
    )
  }
}

fn view_content(event: &WheelEvent) -> (Rect, Rect) {
  let ctx = event.context();

  let view = ctx.box_rect().unwrap();
  let child = ctx.single_child().unwrap();
  let content = ctx.widget_box_rect(child).unwrap();

  (view, content)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::root_and_children_rect;
  use winit::event::{DeviceId, ModifiersState, MouseScrollDelta, TouchPhase, WindowEvent};

  fn test_assert(widget: BoxedWidget, delta_x: f32, delta_y: f32, child_pos: Point) {
    let mut wnd = Window::without_render(widget, Size::new(100., 100.));

    wnd.render_ready();

    let device_id = unsafe { DeviceId::dummy() };
    wnd.processes_native_event(WindowEvent::MouseWheel {
      device_id,
      delta: MouseScrollDelta::LineDelta(delta_x, delta_y),
      phase: TouchPhase::Started,
      modifiers: ModifiersState::default(),
    });
    wnd.render_ready();

    let (_, children) = root_and_children_rect(&mut wnd);
    assert_eq!(children[0].origin, child_pos);
  }

  #[test]
  fn x_scroll() {
    #[derive(Debug)]
    struct X;

    impl CombinationWidget for X {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableX::x_scroll(0.)
          .have_child(SizedBox { size: Size::new(1000., 1000.) }.box_it())
          .box_it()
      }
    }

    test_assert(X.box_it(), 10., 10., Point::new(-10., 0.));
    test_assert(X.box_it(), 10000., 10., Point::new(-900., 0.));
    test_assert(X.box_it(), -100., 10., Point::new(0., 0.));
  }

  #[test]
  fn y_scroll() {
    #[derive(Debug)]
    struct Y;

    impl CombinationWidget for Y {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableY::y_scroll(0.)
          .have_child(SizedBox { size: Size::new(1000., 1000.) }.box_it())
          .box_it()
      }
    }

    test_assert(Y.box_it(), 10., 10., Point::new(0., -10.));
    test_assert(Y.box_it(), 10., 10000., Point::new(0., -900.));
    test_assert(Y.box_it(), -10., -100., Point::new(0., 0.));
  }

  #[test]
  fn both_scroll() {
    #[derive(Debug)]
    struct Both;

    impl CombinationWidget for Both {
      fn build(&self, _: &mut BuildCtx) -> BoxedWidget {
        ScrollableBoth::both_scroll(Point::default())
          .have_child(SizedBox { size: Size::new(1000., 1000.) }.box_it())
          .box_it()
      }
    }

    test_assert(Both.box_it(), 10., 10., Point::new(-10., -10.));
    test_assert(Both.box_it(), 10000., 10000., Point::new(-900., -900.));
    test_assert(Both.box_it(), -100., -100., Point::new(0., 0.));
  }
}
