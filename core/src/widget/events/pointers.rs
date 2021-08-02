use super::EventCommon;
use crate::prelude::*;
use rxrust::prelude::*;
use std::{
  rc::Rc,
  time::{Duration, Instant},
};

mod from_mouse;
#[derive(Debug, Clone)]
pub struct PointerId(usize);

/// The pointer is a hardware-agnostic device that can target a specific set of
/// screen coordinates. Having a single event model for pointers can simplify
/// creating Web sites and applications and provide a good user experience
/// regardless of the user's hardware. However, for scenarios when
/// device-specific handling is desired, pointer events defines a pointerType
/// property to inspect the device type which produced the event.
/// Reference: <https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events#term_pointer_event>
#[derive(Debug, Clone)]
pub struct PointerEvent {
  /// The X, Y coordinate of the pointer in current target widget.
  pub position: Point,
  // The X, Y coordinate of the mouse pointer in global (window) coordinates.
  pub global_pos: Point,
  /// A unique identifier for the pointer causing the event.
  pub id: PointerId,
  /// The width (magnitude on the X axis), in pixels, of the contact geometry of
  /// the pointer.
  pub width: f32,
  /// the height (magnitude on the Y axis), in pixels, of the contact geometry
  /// of the pointer.
  pub height: f32,
  /// the normalized pressure of the pointer input in the range of 0 to 1, where
  /// 0 and 1 represent the minimum and maximum pressure the hardware is capable
  /// of detecting, respectively. tangentialPressure
  /// The normalized tangential pressure of the pointer input (also known as
  /// barrel pressure or cylinder stress) in the range -1 to 1, where 0 is the
  /// neutral position of the control.
  pub pressure: f32,
  /// The plane angle (in degrees, in the range of -90 to 90) between the Y–Z
  /// plane and the plane containing both the pointer (e.g. pen stylus) axis and
  /// the Y axis.
  pub tilt_x: f32,
  /// The plane angle (in degrees, in the range of -90 to 90) between the X–Z
  /// plane and the plane containing both the pointer (e.g. pen stylus) axis and
  /// the X axis.
  pub tilt_y: f32,
  /// The clockwise rotation of the pointer (e.g. pen stylus) around its major
  /// axis in degrees, with a value in the range 0 to 359.
  pub twist: f32,
  ///  Indicates the device type that caused the event (mouse, pen, touch, etc.)
  pub point_type: PointerType,
  /// Indicates if the pointer represents the primary pointer of this pointer
  /// type.
  pub is_primary: bool,
  /// The buttons being depressed (if any) when the mouse event was fired.
  pub buttons: MouseButtons,
  pub common: EventCommon,
}

bitflags! {
  #[derive(Default)]
  pub struct MouseButtons: u8 {
    /// Primary button (usually the left button)
    const PRIMARY = 0b0000_0001;
    /// Secondary button (usually the right button)
    const SECONDARY = 0b0000_0010;
    /// Auxiliary button (usually the mouse wheel button or middle button)
    const AUXILIARY = 0b0000_0100;
    /// 4th button (typically the "Browser Back" button)
    const FOURTH = 0b0000_1000;
    /// 5th button (typically the "Browser Forward" button)
    const FIFTH = 0b0001_0000;
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerType {
  /// The event was generated by a mouse device.
  Mouse,
  /// The event was generated by a pen or stylus device.
  Pen,
  /// The event was generated by a touch, such as a finger.
  Touch,
}

impl std::convert::AsRef<EventCommon> for PointerEvent {
  #[inline]
  fn as_ref(&self) -> &EventCommon { &self.common }
}

impl std::convert::AsMut<EventCommon> for PointerEvent {
  #[inline]
  fn as_mut(&mut self) -> &mut EventCommon { &mut self.common }
}

impl PointerEvent {
  /// The button number that was pressed (if applicable) when the mouse event
  /// was fired.
  pub fn button_num(&self) -> u32 { self.buttons.bits().count_ones() }
}

#[derive(Default)]
pub struct PointerAttr(LocalSubject<'static, (PointerEventType, Rc<PointerEvent>), ()>);

/// A widget that calls callbacks in response to common pointer events.
pub type PointerListener<W> = AttrWidget<W, PointerAttr>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerEventType {
  Down,
  Move,
  Up,
  Tap,
  Cancel,
  Enter,
  Leave,
  /* onpointerover:
   * onpointerout:
   * gotpointercapture:
   * lostpointercapture: */
}

impl<W: Widget> PointerListener<W> {
  pub fn from_widget<A: AttachAttr<W = W>>(widget: A) -> Self {
    let (major, others, widget) = widget.take_attr();
    let major = major.unwrap_or_default();
    PointerListener { widget, major, others }
  }

  #[inline]
  pub fn listen_on<H: FnMut(&PointerEvent) + 'static>(
    &mut self,
    event_type: PointerEventType,
    handler: H,
  ) -> SubscriptionWrapper<LocalSubscription> {
    self.major.listen_on(event_type, handler)
  }

  pub fn tap_times_observable(
    &self,
    times: u8,
  ) -> impl LocalObservable<'static, Item = Rc<PointerEvent>, Err = ()> {
    const DUR: Duration = Duration::from_millis(250);
    #[derive(Clone)]
    struct TapInfo {
      first_tap_stamp: Instant,
      tap_times: u8,
      pointer_type: PointerType,
      mouse_btns: MouseButtons,
    }
    self
      .major
      .pointer_observable()
      .filter(|(t, _)| t == &PointerEventType::Tap)
      .scan_initial(None, move |mut first_tap: Option<(TapInfo, _)>, (_, e)| {
        if let Some((info, event)) = &mut first_tap {
          if info.pointer_type == e.point_type
            && info.mouse_btns == e.buttons
            && info.tap_times < times
            && info.first_tap_stamp.elapsed() < DUR
          {
            info.tap_times += 1;
            *event = Rc::downgrade(&e);
            return first_tap;
          }
        }

        Some((
          TapInfo {
            first_tap_stamp: Instant::now(),
            tap_times: 1,
            pointer_type: e.point_type.clone(),
            mouse_btns: e.buttons,
          },
          Rc::downgrade(&e),
        ))
      })
      .filter_map(
        move |info: Option<(TapInfo, std::rc::Weak<PointerEvent>)>| match info {
          Some((info, e)) if info.tap_times == times => e.upgrade(),
          _ => None,
        },
      )
  }
}

impl PointerAttr {
  /// Return a `Observable` of the pointer events.
  #[inline]
  pub fn pointer_observable(
    &self,
  ) -> LocalSubject<'static, (PointerEventType, Rc<PointerEvent>), ()> {
    self.0.clone()
  }

  pub fn listen_on<H: FnMut(&PointerEvent) + 'static>(
    &mut self,
    event_type: PointerEventType,
    mut handler: H,
  ) -> SubscriptionWrapper<LocalSubscription> {
    self
      .pointer_observable()
      .filter(move |(t, _)| *t == event_type)
      .subscribe(move |(_, event)| handler(&*event))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::widget::window::NoRenderWindow;
  use futures::executor::LocalPool;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton, WindowEvent};

  fn env(times: u8) -> (window::Window<window::MockRender>, Rc<RefCell<usize>>) {
    let size = Size::new(400., 400.);
    let count = Rc::new(RefCell::new(0));
    let c_count = count.clone();
    let sized_box =
      SizedBox::from_size(size).on_tap_times(times, move |_| *c_count.borrow_mut() += 1);
    let mut wnd = NoRenderWindow::without_render(sized_box.box_it(), size);
    wnd.render_ready();

    (wnd, count)
  }

  #[test]
  fn double_tap() {
    let (mut wnd, count) = env(2);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(8)
      .subscribe(move |i| {
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    local_pool.run();

    assert_eq!(*count.borrow(), 2);

    let (mut wnd, count) = env(2);
    observable::interval(Duration::from_millis(251), local_pool.spawner())
      .take(8)
      .subscribe(move |i| {
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    local_pool.run();
    assert_eq!(*count.borrow(), 0);
  }

  #[test]
  fn tripe_tap() {
    let (mut wnd, count) = env(3);

    let mut local_pool = LocalPool::new();
    let device_id = unsafe { DeviceId::dummy() };
    observable::interval(Duration::from_millis(10), local_pool.spawner())
      .take(12)
      .subscribe(move |i| {
        wnd.processes_native_event(WindowEvent::MouseInput {
          device_id,
          state: if i % 2 == 0 {
            ElementState::Pressed
          } else {
            ElementState::Released
          },
          button: MouseButton::Left,
          modifiers: ModifiersState::default(),
        });
      });

    local_pool.run();

    assert_eq!(*count.borrow(), 2);
  }
}
