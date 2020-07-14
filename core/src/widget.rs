use crate::render::*;
use std::{
  any::{Any, TypeId},
  fmt::Debug,
};
pub mod build_ctx;
pub mod key;
pub mod layout;
mod stateful;
pub mod text;
pub mod widget_tree;
pub mod window;
pub use build_ctx::BuildCtx;
pub use key::{Key, KeyDetect};
// pub use layout::row_col_layout::column;
// pub use layout::row_col_layout::row;
pub use stateful::{StateRef, StatefulWidget};
pub use text::Text;
pub mod events;
use events::pointers::{PointerEvent, PointerEventType, PointerListener};
pub use events::Event;
mod phantom;
pub use phantom::PhantomWidget;

/// The common behavior of widgets, also support to dynamic cast to special
/// widget. In most of cases, user needn't implement `Widget` trait directly,
/// and implement `CombinationWidget`, `RenderWidget` `SingleChildWidget` or
/// `MultiChildWidget` is the right way.
pub trait Widget: Debug + Any {
  /// classify this widget into one of four type widget, and return the
  /// reference.
  fn classify(&self) -> WidgetClassify;

  /// classify this widget into one of four type widget as mutation reference.
  fn classify_mut(&mut self) -> WidgetClassifyMut;

  /// return the some-value of `InheritWidget` reference if the widget is
  /// inherit from another widget, otherwise None.
  #[inline]
  fn as_inherit(&self) -> Option<&dyn InheritWidget> { None }

  /// like `as_inherit`, but return mutable reference.
  #[inline]
  fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { None }

  /// Convert a stateless widget to stateful, and will split to a stateful
  /// widget, and a `StateRef` which can be use to modify the states of the
  /// widget.
  #[inline]
  fn into_stateful(self, ctx: &mut BuildCtx) -> (BoxWidget, StateRef<Self>)
  where
    Self: Sized,
  {
    StatefulWidget::stateful(self, ctx.tree.as_mut())
  }

  /// Assign a key to the widget to help framework to track if two widget is a
  /// same widget in two frame.
  #[inline]
  fn with_key<K: Into<Key>>(self, key: K) -> BoxWidget
  where
    Self: Sized,
  {
    KeyDetect::with_key(key, self.box_it())
  }

  /// Used to specify the event handler for the pointer down event, which is
  /// fired when the pointing device is initially pressed.
  #[inline]
  fn on_pointer_down<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: FnMut(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Down, handler)
  }

  /// Used to specify the event handler for the pointer up event, which is
  /// fired when the all pressed pointing device is released.
  #[inline]
  fn on_pointer_up<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Up, handler)
  }

  /// Specify the event handler to process pointer move event.
  #[inline]
  fn on_pointer_move<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Move, handler)
  }

  /// Specify the event handler to process pointer cancel event.
  #[inline]
  fn on_pointer_cancel<F>(self, handler: F) -> BoxWidget
  where
    Self: Sized,
    F: Fn(&PointerEvent) + 'static,
  {
    PointerListener::listen_on(self.box_it(), PointerEventType::Cancel, handler)
  }
}

/// A widget represented by other widget compose.
pub trait CombinationWidget: Debug {
  /// Describes the part of the user interface represented by this widget.
  /// Called by framework, should never directly call it.
  fn build(&self, ctx: &mut BuildCtx) -> BoxWidget;
}

/// a widget has a child.
pub trait SingleChildWidget: RenderWidgetSafety {
  /// Called by framework to take child from this widget, and only called once.
  fn take_child(&mut self) -> BoxWidget;
}

/// a widget has multi child
pub trait MultiChildWidget: RenderWidgetSafety {
  /// Called by framework to take children from this widget, and only called
  /// once. Called by framework, should never directly call it.
  fn take_children(&mut self) -> Vec<BoxWidget>;
}

pub enum WidgetClassify<'a> {
  Combination(&'a dyn CombinationWidget),
  Render(&'a dyn RenderWidgetSafety),
  SingleChild(&'a dyn SingleChildWidget),
  MultiChild(&'a dyn MultiChildWidget),
}

pub enum WidgetClassifyMut<'a> {
  Combination(&'a mut dyn CombinationWidget),
  Render(&'a mut dyn RenderWidgetSafety),
  SingleChild(&'a mut dyn SingleChildWidget),
  MultiChild(&'a mut dyn MultiChildWidget),
}

impl<'a> WidgetClassify<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassify::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassify::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassify::MultiChild(_)) }
}

impl<'a> WidgetClassifyMut<'a> {
  #[inline]
  pub fn is_combination(&self) -> bool { matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_render(&self) -> bool { !matches!(self, WidgetClassifyMut::Combination(_)) }

  #[inline]
  pub fn is_single_child(&self) -> bool { matches!(self, WidgetClassifyMut::SingleChild(_)) }

  #[inline]
  pub fn is_multi_child(&self) -> bool { matches!(self, WidgetClassifyMut::MultiChild(_)) }
}

/// Use inherit method to implement a `Widget`, this is use to extend ability of
/// a widget but not increase the widget number. Notice it's difference to class
/// inherit, it's instance inherit. If the base widget already inherit a same
/// type widget, the new widget should merge into the same type base widget. If
/// the base widget is a `StatefulWidget`, the new widget should inherit
/// between `StatefulWidget` and its base widget, new widget inherit the base
/// widget of `StatefulWidget` and `StatefulWidget` inherit the new widget.
/// `StatefulWidget` is so special is because it's a preallocate widget in
/// widget tree, so if we not do this, the widget inherit from `StatefulWidget`
/// will be lost, so widget inherit `StatefulWidget` will be convert to be
/// inherited by it. Base on the before two point, the inherit order are not
/// guaranteed.
pub trait InheritWidget: Widget {
  fn base_widget(&self) -> &dyn Widget;
  fn base_widget_mut(&mut self) -> &mut dyn Widget;
}

pub struct BoxWidget {
  pub(crate) widget: Box<dyn Widget>,
}

pub trait BoxIt {
  fn box_it(self) -> BoxWidget;
}

impl std::fmt::Debug for BoxWidget {
  #[inline]
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.widget.fmt(f) }
}

impl<W: Widget> BoxIt for W {
  default fn box_it(self) -> BoxWidget {
    BoxWidget {
      widget: Box::new(self),
    }
  }
}

impl BoxIt for BoxWidget {
  #[inline]
  fn box_it(self) -> BoxWidget { self }
}

inherit_widget!(BoxWidget, widget);

/// A function help `InheritWidget` inherit `base` widget.
///
/// ## params
/// *base*: the base widget want inherit from.
/// *ctor_by_base*: construct widget with the base widget should really inherit
/// *merge*: use to merge the widget into the base widget `T`, if the type `T`
/// is already be inherited in the  base widgets, from.
pub fn inherit<T: InheritWidget, C, M>(
  mut base: BoxWidget,
  mut ctor_by_base: C,
  mut merge: M,
) -> BoxWidget
where
  M: FnMut(&mut T),
  C: FnMut(BoxWidget) -> T,
{
  if let Some(already) = Widget::dynamic_cast_mut::<T>(&mut base) {
    merge(already);
    base
  } else if let Some(stateful) = Widget::dynamic_cast_mut::<StatefulWidget>(&mut base) {
    stateful.replace_base_with(|base| ctor_by_base(base).box_it());
    base
  } else {
    ctor_by_base(base).box_it()
  }
}

impl dyn Widget {
  /// Returns some mutable reference to the boxed value if it or its **base
  /// widget** is of type T, or None if it isn't.
  pub fn dynamic_cast_mut<T: 'static>(&mut self) -> Option<&mut T> {
    if Any::type_id(self) == TypeId::of::<T>() {
      let ptr = self as *mut dyn Widget as *mut T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&mut *ptr) }
    } else {
      self
        .as_inherit_mut()
        .and_then(|inherit| inherit.base_widget_mut().dynamic_cast_mut())
    }
  }

  /// Returns some reference to the boxed value if it or its **base widget** is
  /// of type T, or None if it isn't.
  pub fn dynamic_cast_ref<T: 'static>(&self) -> Option<&T> {
    if self.type_id() == TypeId::of::<T>() {
      let ptr = self as *const dyn Widget as *const T;
      // SAFETY: just checked whether we are pointing to the correct type, and we can
      // rely on that check for memory safety because we have implemented Any for
      // all types; no other impls can exist as they would conflict with our impl.
      unsafe { Some(&*ptr) }
    } else {
      self
        .as_inherit()
        .and_then(|inherit| inherit.base_widget().dynamic_cast_ref())
    }
  }
}

use std::borrow::{Borrow, BorrowMut};

pub macro inherit_widget($ty: ty, $base_widget: ident) {
  impl InheritWidget for $ty {
    #[inline]
    fn base_widget(&self) -> &dyn Widget { self.$base_widget.borrow() }
    #[inline]
    fn base_widget_mut(&mut self) -> &mut dyn Widget { self.$base_widget.borrow_mut() }
  }

  impl_widget_for_inherit_widget!($ty);
}

/// Auto implement `Widget` for `CombinationWidget`,  We should also implement
/// `Widget` for RenderWidgetSafety, SingleChildWidget and MultiChildWidget, but
/// can not do it before rust specialization finished. So just CombinationWidget
/// implemented it, this is user use most, and others provide a macro to do it.
impl<T: CombinationWidget + 'static> Widget for T {
  #[inline]
  fn classify(&self) -> WidgetClassify { WidgetClassify::Combination(self) }

  #[inline]
  fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Combination(self) }
}

impl<T: CombinationWidget> !RenderWidget for T {}
impl<T: RenderWidget> !CombinationWidget for T {}
impl<T: MultiChildWidget> !SingleChildWidget for T {}
impl<T: SingleChildWidget> !MultiChildWidget for T {}

pub macro render_widget_base_impl($ty: ty) {
  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::Render(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::Render(self) }
  }
}

pub macro single_child_widget_base_impl($ty: ty) {
  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::SingleChild(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::SingleChild(self) }
  }
}

pub macro impl_widget_for_multi_child_widget($ty: ty) {
  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { WidgetClassify::MultiChild(self) }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { WidgetClassifyMut::MultiChild(self) }
  }
}

pub macro impl_widget_for_inherit_widget($ty: ty) {
  impl Widget for $ty {
    #[inline]
    fn classify(&self) -> WidgetClassify { self.base_widget().classify() }

    #[inline]
    fn classify_mut(&mut self) -> WidgetClassifyMut { self.base_widget_mut().classify_mut() }

    #[inline]
    fn as_inherit(&self) -> Option<&dyn InheritWidget> { Some(self) }

    #[inline]
    fn as_inherit_mut(&mut self) -> Option<&mut dyn InheritWidget> { Some(self) }
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dynamic_cast() {
    let mut widget = Text("hello".to_string())
      .with_key(0)
      .on_pointer_down(|_| {});

    assert!(Widget::dynamic_cast_ref::<KeyDetect>(&widget).is_some());
    assert!(Widget::dynamic_cast_mut::<KeyDetect>(&mut widget).is_some());
    assert!(Widget::dynamic_cast_ref::<PointerListener>(&widget).is_some());
    assert!(Widget::dynamic_cast_mut::<PointerListener>(&mut widget).is_some());
    assert!(Widget::dynamic_cast_ref::<Text>(&widget).is_some());
    assert!(Widget::dynamic_cast_mut::<Text>(&mut widget).is_some());
  }
}
