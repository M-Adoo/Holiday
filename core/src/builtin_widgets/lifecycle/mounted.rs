use std::cell::RefCell;

use crate::{
  data_widget::compose_child_as_data_widget, impl_lifecycle, impl_query_self_only, prelude::*,
};

/// Listener perform when its child widget add to the widget tree.
#[derive(Declare)]
pub struct MountedListener {
  #[declare(builtin, convert=listener_callback(for<'r> FnMut(LifeCycleCtx<'r>)))]
  pub mounted: RefCell<Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>>,
}
impl_lifecycle!(MountedListener, mounted);
