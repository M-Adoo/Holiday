use crate::prelude::*;

#[derive(Declare, Default, Clone)]
pub struct Icon {
  pub src: &'static str,
  pub size: Size,
}

impl StatefulCombination for Icon {
  #[widget]
  fn build(this: &Stateful<Self>, ctx: &mut BuildCtx) -> BoxedWidget {
    let Size { width, height, .. } = this.size;
    let svg = Svg::new(load_src(this.src).unwrap());
    widget! {
      declare SizedBox {
        size: Size::new(width, height),
        ExprChild { svg }
      }
    }
  }
}
