use crate::layout::SizedBox;
use ribir_core::prelude::*;
use std::time::Duration;
#[derive(Declare2)]
pub struct Caret {
  pub focused: bool,
  pub height: f32,
  #[declare(default = svgs::TEXT_CARET)]
  pub icon: NamedSvg,
}

impl Compose for Caret {
  fn compose(this: State<Self>) -> impl WidgetBuilder {
    let blink_interval = Duration::from_millis(500);
    fn_widget! {
      let icon = $this.icon;
      let mut caret = @ $icon {
        opacity: 0.,
        box_fit: BoxFit::Fill,
      };
      let mut _guard = None;
      watch!($this.focused)
        .distinct_until_changed()
        .subscribe(move |focused| {
          if focused {
            $caret.write().opacity = 1.;
            let unsub = interval(blink_interval, AppCtx::scheduler())
              .subscribe(move |idx| $caret.write().opacity = (idx % 2) as f32)
              .unsubscribe_when_dropped();
            _guard = Some(unsub);
          } else {
            $caret.write().opacity = 0.;
            _guard = None;
          }
        });

      @SizedBox {
        left_anchor: pipe!(-$this.height / 2.),
        size: pipe!(Size::new($this.height, $this.height)),
        @ { caret }
      }
    }
  }
}
