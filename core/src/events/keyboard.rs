use rxrust::prelude::*;
use std::convert::Infallible;

use crate::{
  impl_all_event, impl_common_event_deref, impl_compose_child_with_focus_for_listener,
  impl_listener, impl_multi_event_listener, prelude::*, window::WindowId,
};

pub use winit::keyboard::{
  Key as VirtualKey, KeyCode, KeyLocation, ModifiersState, NamedKey, PhysicalKey,
};

#[derive(Debug)]
pub struct KeyboardEvent {
  physical_key: PhysicalKey,
  key: VirtualKey,
  is_repeat: bool,
  location: KeyLocation,
  common: CommonEvent,
}

impl KeyboardEvent {
  #[inline]
  pub fn key_code(&self) -> &PhysicalKey { &self.physical_key }

  #[inline]
  pub fn key(&self) -> &VirtualKey { &self.key }

  #[inline]
  pub fn is_repeat(&self) -> bool { self.is_repeat }

  #[inline]
  pub fn location(&self) -> KeyLocation { self.location }
}

pub type KeyboardSubject = MutRefItemSubject<'static, AllKeyboard, Infallible>;

impl_multi_event_listener! {
  "The listener use to fire and listen keyboard events.",
  Keyboard,
  "The `KeyDown` event is fired when a key is pressed.",
  KeyDown,
  "The `KeyDownCapture` event is same as `KeyDown` but emit in capture phase.",
  KeyDownCapture,
  "The `KeyUp` event is fired when a key is released.",
  KeyUp,
  "The `KeyUpCapture` event is same as `KeyUp` but emit in capture phase.",
  KeyUpCapture
}

impl_common_event_deref!(KeyboardEvent);

impl_compose_child_with_focus_for_listener!(KeyboardListener);

impl KeyboardEvent {
  #[inline]
  pub fn new(
    id: WidgetId,
    wnd_id: WindowId,
    physical_key: PhysicalKey,
    key: VirtualKey,
    is_repeat: bool,
    location: KeyLocation,
  ) -> Self {
    Self {
      physical_key,
      key,
      is_repeat,
      location,
      common: CommonEvent::new(id, wnd_id),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{test_helper::*, window::DelayEvent};
  use std::{cell::RefCell, rc::Rc};

  #[test]
  fn smoke() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    #[derive(Default)]
    struct Keys(Rc<RefCell<Vec<String>>>);

    impl Compose for Keys {
      fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
        fn_widget! {
          @MockBox {
            size: Size::zero(),
            on_key_down_capture: move |key| {
              $this.0.borrow_mut().push(format!("key down capture {:?}", key.key));
            },
            on_key_up_capture: move |key| {
              $this.0.borrow_mut().push(format!("key up capture {:?}", key.key));
            },
            @MockBox {
              size: Size::zero(),
              auto_focus: true,
              on_key_down: move |key| {
                $this.0
                  .borrow_mut()
                  .push(format!("key down {:?}", key.key));
              },
              on_key_up: move |key| {
                $this.0.borrow_mut().push(format!("key up {:?}", key.key));
              }
            }
          }
        }
      }
    }

    let w = Keys::default();
    let keys = w.0.clone();

    let mut wnd = TestWindow::new(fn_widget!(w));
    wnd.draw_frame();

    let focusing = wnd.focusing().unwrap();

    wnd.add_delay_event(DelayEvent::KeyDown(KeyboardEvent::new(
      focusing,
      wnd.id(),
      PhysicalKey::Code(KeyCode::Digit0),
      VirtualKey::Character("0".into()),
      false,
      KeyLocation::Standard,
    )));

    wnd.add_delay_event(DelayEvent::KeyUp(KeyboardEvent::new(
      focusing,
      wnd.id(),
      PhysicalKey::Code(KeyCode::Digit0),
      VirtualKey::Character("0".into()),
      false,
      KeyLocation::Standard,
    )));

    wnd.add_delay_event(DelayEvent::KeyDown(KeyboardEvent::new(
      focusing,
      wnd.id(),
      PhysicalKey::Code(KeyCode::Digit1),
      VirtualKey::Character("1".into()),
      false,
      KeyLocation::Standard,
    )));

    wnd.add_delay_event(DelayEvent::KeyUp(KeyboardEvent::new(
      focusing,
      wnd.id(),
      PhysicalKey::Code(KeyCode::Digit1),
      VirtualKey::Character("1".into()),
      false,
      KeyLocation::Standard,
    )));

    wnd.run_frame_tasks();

    assert_eq!(
      &*keys.borrow(),
      &[
        "key down capture Character(\"0\")",
        "key down Character(\"0\")",
        "key up capture Character(\"0\")",
        "key up Character(\"0\")",
        "key down capture Character(\"1\")",
        "key down Character(\"1\")",
        "key up capture Character(\"1\")",
        "key up Character(\"1\")"
      ]
    );
  }
}
