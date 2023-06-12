use crate::prelude::*;
use std::cell::Cell;

pub enum BoxFit {
  /// Widget will not be scale.
  None,
  /// The entire widget will completely fill its container. If the widget's
  /// aspect ratio does not match the aspect ratio of its box, then the widget
  /// will be stretched to fit.
  Fill,
  /// Widget is scaled to maintain its aspect ratio while fitting within the
  /// container box. The entire widget is made to fill the box, while preserving
  /// its aspect ratio,
  Contain,
  /// Widget is scale to maintain its aspect ratio while filling to full cover
  /// its container box. If the widget's aspect ratio does not the aspect ratio
  /// of its box, then the widget will be clipped to fit.
  Cover,
}

/// Widget set how its child should be scale to fit its box.
#[derive(Declare, SingleChild)]
pub struct FittedBox {
  #[declare(builtin)]
  pub box_fit: BoxFit,
  #[declare(default)]
  scale_cache: Cell<Transform>,
}

impl Render for FittedBox {
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let container_size = clamp.max;
    clamp.max = INFINITY_SIZE;
    let child_size = ctx.assert_perform_single_child_layout(clamp);

    if child_size.is_empty() {
      self.scale_cache.set(Transform::default());
    }
    let scale_x = container_size.width / child_size.width;
    let scale_y = container_size.height / child_size.height;
    match self.box_fit {
      BoxFit::None => self.scale_cache.set(Transform::scale(1., 1.)),
      BoxFit::Fill => self.scale_cache.set(Transform::scale(scale_x, scale_y)),
      BoxFit::Contain => {
        let scale = scale_x.min(scale_y);
        self.scale_cache.set(Transform::scale(scale, scale));
      }
      BoxFit::Cover => {
        let scale = scale_x.max(scale_y);
        self.scale_cache.set(Transform::scale(scale, scale));
      }
    }
    let Transform { m11: x, m22: y, .. } = self.scale_cache.get();
    Size::new(child_size.width * x, child_size.height * y)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    let Transform { m11: x, m22: y, .. } = self.scale_cache.get();
    if matches!(self.box_fit, BoxFit::Cover) {
      let size = ctx.box_size().unwrap();
      let child_size = ctx
        .single_child_box()
        .expect("Should always have a single child")
        .size;
      if size.width < child_size.width * x || size.height < child_size.height * y {
        let path = Path::rect(&Rect::from(size));
        ctx.painter().clip(path);
      }
    }

    ctx.painter().scale(x, y);
  }

  fn get_transform(&self) -> Option<Transform> { Some(self.scale_cache.get()) }
}

crate::impl_query_self_only!(FittedBox);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use ribir_dev_helper::*;

  const WND_SIZE: Size = Size::new(300., 300.);

  struct FitTestCase {
    box_fit: BoxFit,
    size: Size,
    expect: Size,
    expected_scale: Transform,
  }

  impl FitTestCase {
    fn test(self) {
      let Self {
        box_fit,
        size,
        expect,
        expected_scale,
      } = self;
      let fit = Stateful::new(FittedBox { box_fit, scale_cache: <_>::default() });
      let c_fit = fit.clone();
      let w = widget! {
        DynWidget {
          dyns: fit,
          MockBox { size }
        }
      };
      let mut wnd = TestWindow::new_with_size(w, WND_SIZE);
      wnd.draw_frame();

      assert_layout_result_by_path!(wnd, {path = [0], size == expect,} );
      assert_eq!(c_fit.shallow_ref().scale_cache.get(), expected_scale);
    }
  }

  #[test]
  fn fit_test() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let small_size: Size = Size::new(100., 150.);

    FitTestCase {
      box_fit: BoxFit::None,
      size: small_size,
      expect: small_size,
      expected_scale: Transform::scale(1., 1.),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Fill,
      size: small_size,
      expect: WND_SIZE,
      expected_scale: Transform::scale(3., 2.),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Cover,
      size: small_size,
      expect: WND_SIZE,
      expected_scale: Transform::scale(3., 3.),
    }
    .test();

    let big_size_clip = Size::new(600., 900.);
    FitTestCase {
      box_fit: BoxFit::Cover,
      size: big_size_clip,
      expect: WND_SIZE,
      expected_scale: Transform::scale(0.5, 0.5),
    }
    .test();

    FitTestCase {
      box_fit: BoxFit::Contain,
      size: small_size,
      expect: Size::new(200., 300.),
      expected_scale: Transform::scale(2., 2.),
    }
    .test();
  }

  fn as_builtin_field() -> Widget {
    widget! {
      MockBox {
        size: Size::new(200., 200.),
        box_fit: BoxFit::Fill,
      }
    }
    .into()
  }
  widget_layout_test!(as_builtin_field, wnd_size = WND_SIZE, size == WND_SIZE,);
}
