pub use crate::prelude::*;
/// Trait to tell Ribir a widget can have one child.
pub trait SingleChild {}

/// Trait to tell Ribir a widget can have multi child.
pub trait MultiChild {}

/// Trait mark widget can have one child and also have compose logic for widget
/// and its child.
pub trait ComposeChild {
  type Child;
  fn compose_child(this: StateWidget<Self>, child: Self::Child) -> Widget
  where
    Self: Sized;
}

pub trait WithChild<M: ?Sized, C> {
  type Target;
  fn with_child(self, child: C) -> Self::Target;
}

pub struct WidgetWithChild<W, C> {
  pub(crate) widget: W,
  pub(crate) child: C,
}

// implementation of IntoWidget

impl<W> IntoWidget<(&dyn Render, Widget)> for WidgetWithChild<W, Widget>
where
  W: SingleChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Render(Box::new(widget));
    let children = Children::Single(Box::new(child));
    Widget { node: Some(node), children }
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Render)> for WidgetWithChild<W, C>
where
  W: SingleChild + Render + 'static,
  C: Render + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C> IntoWidget<(&dyn Render, dyn Compose)> for WidgetWithChild<W, C>
where
  W: Render + SingleChild + 'static,
  C: Compose + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W, C, M: ?Sized> IntoWidget<(&M, ConstExprWidget<C>)>
  for WidgetWithChild<W, ConstExprWidget<C>>
where
  WidgetWithChild<W, C>: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.expr }.into_widget()
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, Option<&M2>)> for WidgetWithChild<W, Option<C>>
where
  W: IntoWidget<M1>,
  WidgetWithChild<W, C>: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(child) = child {
      WidgetWithChild { widget, child }.into_widget()
    } else {
      widget.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized> IntoWidget<(Option<&M1>, &M2)> for WidgetWithChild<Option<W>, C>
where
  WidgetWithChild<W, C>: IntoWidget<M1>,
  C: IntoWidget<M2>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    if let Some(widget) = widget {
      WidgetWithChild { widget, child }.into_widget()
    } else {
      child.into_widget()
    }
  }
}

impl<W, C, M1: ?Sized, M2: ?Sized, M3: ?Sized> IntoWidget<(&M3, Option<&M1>, Option<&M2>)>
  for WidgetWithChild<Option<W>, Option<C>>
where
  W: IntoWidget<M1>,
  C: IntoWidget<M2>,
  WidgetWithChild<W, C>: IntoWidget<M3>,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    match (widget, child) {
      (None, None) => Void.into_widget(),
      (None, Some(child)) => child.into_widget(),
      (Some(widget), None) => widget.into_widget(),
      (Some(widget), Some(child)) => WidgetWithChild { widget, child }.into_widget(),
    }
  }
}

impl<W, E, R, M: ?Sized> IntoWidget<(ExprWidget<E>, &M)> for WidgetWithChild<ExprWidget<E>, W>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: SingleChild + Render + 'static,
  W: IntoWidget<M>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let mut widget = widget.into_widget();
    widget.children = Children::Single(Box::new(child.into_widget()));

    widget
  }
}

impl<W, E, R, M1: ?Sized, M2: ?Sized> IntoWidget<(&M1, ExprWidget<&M2>)>
  for WidgetWithChild<W, ExprWidget<E>>
where
  WidgetWithChild<W, Widget>: IntoWidget<M1>,
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: SingleDyn<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild {
      widget,
      child: child.inner_into_widget(),
    }
    .into_widget()
  }
}

impl<W1, W2, C, M2: ?Sized> IntoWidget<(&dyn Render, &M2)>
  for WidgetWithChild<W1, WidgetWithChild<W2, C>>
where
  W1: Render + SingleChild + 'static,
  WidgetWithChild<W2, C>: IntoWidget<M2>,
{
  #[inline]
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_widget() }.into_widget()
  }
}

impl<W> IntoWidget<dyn Render> for WidgetWithChild<W, Vec<Widget>>
where
  W: MultiChild + Render + 'static,
{
  fn into_widget(self) -> Widget {
    let Self { widget, child } = self;
    let node = WidgetNode::Render(Box::new(widget));
    let children = Children::Multi(child);
    Widget { node: Some(node), children }
  }
}

// todo: impl have children for it and strip the exprWidget.
impl<R: SingleChild, E> SingleChild for ExprWidget<E> where E: FnMut(&mut BuildCtx) -> R {}
impl<R: MultiChild, E> MultiChild for ExprWidget<E> where E: FnMut(&mut BuildCtx) -> R {}

pub trait IntoChild<M: ?Sized, C> {
  fn into_child(self) -> C;
}

// `IntoChild` implementations
impl<W: IntoWidget<M>, M: ?Sized> IntoChild<dyn IntoWidget<M>, Widget> for W {
  #[inline]
  fn into_child(self) -> Widget { self.into_widget() }
}

impl<T: IntoWidget<M>, M: ?Sized> IntoChild<dyn IntoWidget<M>, Vec<Widget>> for T {
  #[inline]
  fn into_child(self) -> Vec<Widget> { vec![self.into_child()] }
}

impl<E, M: ?Sized, C> IntoChild<dyn IntoChild<M, C>, C> for ConstExprWidget<E>
where
  E: IntoChild<M, C>,
{
  #[inline]
  fn into_child(self) -> C { self.expr.into_child() }
}

impl<T, M: ?Sized, C> IntoChild<dyn IntoChild<M, C>, Option<C>> for T
where
  T: IntoChild<M, C>,
{
  #[inline]
  fn into_child(self) -> Option<C> { Some(self.into_child()) }
}

impl<E, R, M: ?Sized> IntoChild<dyn IntoChild<M, Widget>, Widget> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: SingleDyn<M>,
{
  #[inline]
  fn into_child(self) -> Widget { self.inner_into_widget() }
}

impl<E, R, M: ?Sized> IntoChild<ExprWidget<&M>, Vec<Widget>> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: IntoWidget<M>,
{
  #[inline]
  fn into_child(self) -> Vec<Widget> { vec![self.inner_into_widget()] }
}

impl<E, R, M: ?Sized> IntoChild<dyn Iterator<Item = &M>, Vec<Widget>> for ExprWidget<E>
where
  E: FnMut(&mut BuildCtx) -> R + 'static,
  R: IntoIterator + 'static,
  R::Item: IntoWidget<M>,
{
  #[inline]
  fn into_child(self) -> Vec<Widget> { vec![self.inner_into_widget()] }
}

impl<T, M: ?Sized> IntoChild<dyn Iterator<Item = &M>, Vec<Widget>> for T
where
  T: IntoIterator,
  T::Item: IntoWidget<M>,
{
  #[inline]
  fn into_child(self) -> Vec<Widget> { self.into_iter().map(IntoWidget::into_widget).collect() }
}

impl<W, C, M: ?Sized> IntoChild<M, WidgetWithChild<W, Widget>> for WidgetWithChild<W, C>
where
  C: IntoChild<M, Widget>,
{
  #[inline]
  fn into_child(self) -> WidgetWithChild<W, Widget> {
    let Self { widget, child } = self;
    WidgetWithChild { widget, child: child.into_child() }
  }
}

macro_rules! tuple_into_children {
  ($ty: ident, $mark: ident, $($other_ty: ident, $other_mark: ident,)+) => {
    tuple_into_children!({$ty, $mark, } $($other_ty, $other_mark,)+);
  };
  (
    {$($ty: ident, $mark: ident,)+}
    $next_ty: ident, $next_mark: ident,
    $($other_ty: ident, $other_mark: ident,)*
  ) => {
      tuple_into_children!({$($ty, $mark,)+});
      tuple_into_children!({$($ty, $mark,)+ $next_ty, $next_mark, } $($other_ty, $other_mark,)*);
  };
  ({ $($ty: ident, $mark: ident,)+})  => {
    impl<$($ty, $mark: ?Sized),+> IntoChild<($(&$mark,)+), Vec<Widget>> for ($($ty,)+)
    where
      $($ty: IntoChild<$mark, Vec<Widget>>),+
    {
      #[allow(non_snake_case)]
      fn into_child(self) -> Vec<Widget> {
        let ($($ty,)+) = self;
        let mut children = vec![];
        $(children.extend($ty.into_child());)+
        children
      }
    }
  }
}

tuple_into_children!(
  T1, M1, T2, M2, T3, M3, T4, M4, T5, M5, T6, M6, T7, M7, T8, M8, T9, M9, T10, M10, T11, M11, T12,
  M12, T13, M13, T14, M14, T15, M15, T16, M16, T17, M17, T18, M18, T19, M19, T20, M20, T21, M21,
  T22, M22, T23, M23, T24, M24, T25, M25, T26, M26, T27, M27, T28, M28, T29, M29, T30, M30, T31,
  M31, T32, M32,
);

macro_rules! tuple_child_into {
  (
    $target: ident, $from: ident, $mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)+
  ) => {
    tuple_child_into!({$target, $from, $mark, } $($other_target, $other_from, $other_mark,)+);
  };
  (
    {$($target: ident, $from: ident, $mark: ident,)+}
    $next_target: ident, $next_from: ident, $next_mark: ident,
    $($other_target: ident, $other_from: ident, $other_mark: ident,)*
  ) => {
      tuple_child_into!({ $($target, $from, $mark,)+ });
      tuple_child_into!(
        {$($target, $from, $mark,)+ $next_target, $next_from, $next_mark, }
        $($other_target, $other_from, $other_mark,)*
      );
  };
  ({ $($target: ident, $from: ident, $mark: ident,)+ })  => {

    impl<$($target, $from, $mark: ?Sized),+> IntoChild<($(&$mark,)+), ($($target,)+)> for ($($from,)+)
    where
      $($from: IntoChild<$mark, $target>),+
    {
      #[allow(non_snake_case)]
      fn into_child(self) -> ($($target,)+) {
        let ($($from,)+) = self;
        ($($from.into_child(),)+)
      }
    }
  }
}

tuple_child_into!(
  T1, F1, M1, T2, F2, M2, T3, F3, M3, T4, F4, M4, T5, F5, M5, T6, F6, M6, T7, F7, M7, T8, F8, M8,
  T9, F9, M9, T10, F10, M10, T11, F11, M11, T12, F12, M12, T13, F13, M13, T14, F14, M14, T15, F15,
  M15, T16, F16, M16, T17, F17, M17, T18, F18, M18, T19, F19, M19, T20, F20, M20, T21, F21, M21,
  T22, F22, M22, T23, F23, M23, T24, F24, M24, T25, F25, M25, T26, F26, M26, T27, F27, M27, T28,
  F28, M28, T29, F29, M29, T30, F30, M30, T31, F31, M31, T32, F32, M32,
);

// implementations of `WithChild`

impl<M: ?Sized, W, C> WithChild<dyn ComposeChild<Child = &M>, C> for W
where
  W: ComposeChild,
  C: IntoChild<M, W::Child>,
{
  type Target = Widget;

  fn with_child(self, child: C) -> Self::Target {
    ComposeChild::compose_child(StateWidget::Stateless(self), child.into_child())
  }
}

impl<W, C> WithChild<dyn SingleChild, C> for W
where
  W: SingleChild,
{
  type Target = WidgetWithChild<Self, C>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target { WidgetWithChild { widget: self, child } }
}

impl<W, C, M: ?Sized> WithChild<(&dyn MultiChild, &M), C> for W
where
  W: MultiChild,
  C: IntoChild<M, Vec<Widget>>,
{
  type Target = WidgetWithChild<Self, Vec<Widget>>;
  #[inline]
  fn with_child(self, child: C) -> Self::Target {
    WidgetWithChild {
      widget: self,
      child: child.into_child(),
    }
  }
}

impl<M: ?Sized, E: WithChild<M, C>, C> WithChild<M, C> for ConstExprWidget<E> {
  type Target = E::Target;

  #[inline]
  fn with_child(self, child: C) -> Self::Target { self.expr.with_child(child) }
}

// Option type only support have single child to replace self.
impl<W: SingleChild> SingleChild for Option<W> {}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn compose_tuple_child() {
    #[derive(Declare)]
    struct Page;
    #[derive(Declare, SingleChild)]
    struct Header;
    #[derive(Declare, SingleChild)]
    struct Content;
    #[derive(Declare, SingleChild)]
    struct Footer;

    impl ComposeChild for Page {
      type Child = (
        WidgetWithChild<Header, Widget>,
        WidgetWithChild<Content, Widget>,
        WidgetWithChild<Footer, Widget>,
      );

      fn compose_child(_: StateWidget<Self>, child: Self::Child) -> Widget {
        let (header, content, footer) = child;
        widget! {
          Column {
            ExprWidget {
              expr: [header.child, content.child, footer.child]
            }
          }
        }
      }
    }

    widget! {
      Page {
        Header { Void {} }
        Content { Void {} }
        Footer { Void {} }
      }
    };
  }

  #[test]
  fn compose_option_child() {
    #[derive(Declare)]
    struct Parent;
    #[derive(Declare, SingleChild)]
    struct Child;

    impl ComposeChild for Parent {
      type Child = Option<WidgetWithChild<Child, Widget>>;

      fn compose_child(_: StateWidget<Self>, _: Self::Child) -> Widget {
        unreachable!("Only for syntax support check");
      }
    }

    widget! {
      Parent {
        Child { Void {} }
      }
    };
  }
}
