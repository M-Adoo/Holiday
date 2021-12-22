#![feature(
  test,
  decl_macro,
  negative_impls,
  cell_filter_map,
  linked_list_cursors,
  trivial_bounds,
  auto_traits
)]

#[macro_use]
extern crate bitflags;
extern crate lazy_static;
extern crate widget_derive;

mod application;
pub mod declare;
mod render;
pub mod widget;

pub mod prelude {
  #[doc(no_inline)]
  pub use crate::application::Application;
  #[doc(no_inline)]
  pub use crate::declare::{Declare, DeclareBuilder};
  #[doc(no_inline)]
  pub use crate::render::*;
  #[doc(no_inline)]
  pub use crate::widget;
  #[doc(no_inline)]
  pub use crate::widget::{build_ctx::BuildCtx, widget_tree::WidgetId, *};
  #[doc(no_inline)]
  pub use canvas::prelude::*;
  #[doc(hidden)]
  pub use rxrust::prelude::*;
  #[doc(no_inline)]
  pub use widget::layout::{MultiChild, SingleChild};
  #[doc(no_inline)]
  pub use widget_derive::{
    declare, stateful, CombinationWidget, Declare, MultiChildWidget, RenderWidget,
    SingleChildWidget,
  };
}

pub mod test;
