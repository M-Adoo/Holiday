use crate::prelude::*;
use crate::util::proxy_macro::*;

/// boundary limit of the render object's layout
#[derive(Debug, Clone, Copy)]
pub struct BoxLimit {
  pub min_height: f32,
  pub max_height: f32,
  pub min_width: f32,
  pub max_width: f32,
}

/// render object's layout box, the information about layout, including box
/// size, layout constraints and box_bound.application
#[derive(Debug)]
pub struct BoxLayout {
  constraints: LayoutConstraints,

  /// box bound is the bound of the layout can be place. it should be set before
  /// render object's process of layout. when the object it is in the layout
  /// such as row, flex ... it's size is decided by his parent.
  box_bound: Option<BoxLimit>,
}

impl BoxLayout {
  pub fn new(lc: LayoutConstraints) -> BoxLayout {
    BoxLayout {
      constraints: lc,

      box_bound: None,
    }
  }

  pub fn get_box_limit(&self) -> BoxLimit {
    if self.box_bound.is_some() {
      self.box_bound.unwrap()
    } else {
      BoxLimit {
        min_height: 0.0,
        max_height: f32::INFINITY,
        min_width: 0.0,
        max_width: f32::INFINITY,
      }
    }
  }

  pub fn set_box_limit(&mut self, bound: Option<BoxLimit>) { self.box_bound = bound; }

  pub fn get_constraints(&self) -> LayoutConstraints { self.constraints }
}

pub macro default_box_impl( {$($path:ident).*}) {
    #[inline]
    proxy_immut_impl!{get_constraints,  {$($path).* }, LayoutConstraints, get_constraints, ()}

    #[inline]
    proxy_mut_impl!{set_box_limit,  {$($path).*}, (), set_box_limit, (bound: Option<BoxLimit>)}
}
