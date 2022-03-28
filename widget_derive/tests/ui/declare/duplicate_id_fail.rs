use ribir::prelude::*;

#[widget]
fn id_must_be_unique_err(_this: (), ctx: &mut BuildCtx) {
  widget! {
    BoxDecoration {
      id: same_id,
      background: Some(Color::RED.into()),
      ..<_>::default(),
      SizedBox {
        id: same_id,
        size: Size::zero(),
      }
    }
  };
}

fn main() {}
