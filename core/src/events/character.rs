use crate::{impl_query_self_only, prelude::*};

/// An attribute that sends a single Unicode codepoint. The character can be
/// pushed to the end of a string.
#[derive(Declare)]
pub struct CharListener {
  #[declare(builtin, custom_convert)]
  on_char: Box<dyn for<'r> FnMut(&'r mut CharEvent)>,
}

#[derive(Debug)]
pub struct CharEvent {
  pub char: char,
  pub common: EventCommon,
}

impl ComposeSingleChild for CharListener {
  #[inline]
  fn compose_single_child(this: Stateful<Self>, child: Option<Widget>, _: &mut BuildCtx) -> Widget {
    compose_child_as_data_widget(child, this, |w| w)
  }
}

impl Query for CharListener {
  impl_query_self_only!();
}

impl std::borrow::Borrow<EventCommon> for CharEvent {
  #[inline]
  fn borrow(&self) -> &EventCommon { &self.common }
}

impl std::borrow::BorrowMut<EventCommon> for CharEvent {
  #[inline]
  fn borrow_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl std::ops::Deref for CharEvent {
  type Target = EventCommon;

  #[inline]
  fn deref(&self) -> &Self::Target { &self.common }
}

impl std::ops::DerefMut for CharEvent {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.common }
}

impl CharListener {
  #[inline]
  pub fn dispatch_event(&mut self, event: &mut CharEvent) { (self.on_char)(event) }
}

impl CharListenerBuilder {
  #[inline]
  pub fn on_char_convert(
    f: impl for<'r> FnMut(&'r mut CharEvent) + 'static,
  ) -> Box<dyn for<'r> FnMut(&'r mut CharEvent)> {
    Box::new(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;

  #[test]
  fn smoke() {
    let receive = Rc::new(RefCell::new("".to_string()));
    let c_receive = receive.clone();

    let widget = widget! {
      SizedBox {
        size: SizedBox::shrink_size(),
        auto_focus: true,
        on_char: move |key| c_receive.borrow_mut().push(key.char)
      }
    };
    let mut wnd = Window::without_render(widget.into_widget(), Size::new(100., 100.));

    let test_text_case = "Hello 世界！";
    wnd.render_ready();
    test_text_case
      .chars()
      .for_each(|c| wnd.processes_native_event(WindowEvent::ReceivedCharacter(c)));

    assert_eq!(&*receive.borrow(), test_text_case);
  }
}
