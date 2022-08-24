use crate::{
  impl_lifecycle, impl_query_self_only,
  prelude::{data_widget::compose_child_as_data_widget, *},
};

#[derive(Declare)]
pub struct DisposedListener {
  #[declare(builtin, convert=box_trait(for<'r> FnMut(LifeCycleCtx<'r>)))]
  pub on_disposed: Box<dyn for<'r> FnMut(LifeCycleCtx<'r>)>,
}

impl_lifecycle!(DisposedListener, on_disposed);
