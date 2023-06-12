use crate::{prelude::*, widget_tree::WidgetTree, window::DelayEvent};
use ribir_text::PIXELS_PER_EM;
use std::rc::{Rc, Weak};
use winit::event::{DeviceId, ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};

pub(crate) struct Dispatcher {
  wnd: Weak<Window>,
  pub(crate) info: DispatchInfo,
  pub(crate) entered_widgets: Vec<WidgetId>,
  pub(crate) pointer_down_uid: Option<WidgetId>,
}

impl Dispatcher {
  pub fn new() -> Self {
    Self {
      wnd: Weak::new(),
      info: <_>::default(),
      entered_widgets: vec![],
      pointer_down_uid: None,
    }
  }

  pub fn init(&mut self, wnd: Weak<Window>) { self.wnd = wnd; }

  pub fn window(&self) -> Rc<Window> {
    self
      .wnd
      .upgrade()
      .expect("The window of the `Dispatcher` already dropped")
  }
}
#[derive(Default)]
pub(crate) struct DispatchInfo {
  /// The current state of mouse button press state.
  mouse_button: (Option<DeviceId>, MouseButtons),
  /// The current global position (relative to window) of mouse
  cursor_pos: Point,
  /// The current state of the keyboard modifiers
  modifiers: ModifiersState,
}

impl Dispatcher {
  pub fn dispatch(&mut self, event: WindowEvent, wnd_factor: f64) {
    log::info!("Dispatch winit event {:?}", event);
    match event {
      WindowEvent::ModifiersChanged(s) => self.info.modifiers = s,
      WindowEvent::CursorMoved { position, .. } => {
        let pos = position.to_logical::<f32>(wnd_factor);
        self.cursor_move_to(Point::new(pos.x, pos.y))
      }
      WindowEvent::CursorLeft { .. } => self.on_cursor_left(),
      WindowEvent::MouseInput { state, button, device_id, .. } => {
        self.dispatch_mouse_input(device_id, state, button);
      }
      WindowEvent::KeyboardInput { input, .. } => {
        self.dispatch_keyboard_input(input);
      }
      WindowEvent::ReceivedCharacter(c) => {
        self.add_chars_event(c.to_string());
      }
      WindowEvent::Ime(ime) => {
        if let Ime::Commit(s) = ime {
          self.add_chars_event(s)
        }
      }
      WindowEvent::MouseWheel { delta, .. } => self.dispatch_wheel(delta, wnd_factor),
      _ => log::info!("not processed event {:?}", event),
    }
  }

  pub fn dispatch_keyboard_input(&mut self, input: winit::event::KeyboardInput) {
    let wnd = self.window();
    if let (Some(key), Some(id)) = (input.virtual_keycode, wnd.focusing()) {
      let scancode = input.scancode;
      match input.state {
        ElementState::Pressed => wnd.add_delay_event(DelayEvent::KeyDown { id, scancode, key }),
        ElementState::Released => wnd.add_delay_event(DelayEvent::KeyUp { id, scancode, key }),
      };
    }
  }

  pub fn add_chars_event(&mut self, chars: String) {
    let wnd = self.window();
    if let Some(focus) = wnd.focusing() {
      self
        .window()
        .add_delay_event(DelayEvent::Chars { id: focus, chars });
    }
  }

  pub fn cursor_move_to(&mut self, position: Point) {
    self.info.cursor_pos = position;
    self.pointer_enter_leave_dispatch();
    if let Some(hit) = self.hit_widget() {
      self.window().add_delay_event(DelayEvent::PointerMove(hit));
    }
  }

  pub fn on_cursor_left(&mut self) {
    self.info.cursor_pos = Point::new(-1., -1.);
    self.pointer_enter_leave_dispatch();
  }

  pub fn dispatch_mouse_input(
    &mut self,
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
  ) {
    // A mouse press/release emit during another mouse's press will ignored.
    if self.info.mouse_button.0.get_or_insert(device_id) == &device_id {
      match state {
        ElementState::Pressed => {
          self.info.mouse_button.1 |= button.into();
          // only the first button press emit event.
          if self.info.mouse_button.1 == button.into() {
            self.bubble_pointer_down();
          }
        }
        ElementState::Released => {
          self.info.mouse_button.1.remove(button.into());
          // only the last button release emit event.
          if self.info.mouse_button.1.is_empty() {
            self.info.mouse_button.0 = None;
            let wnd = self.window();
            let mut dispatch = |tree: &WidgetTree| {
              let hit = self.hit_widget()?;
              wnd.add_delay_event(DelayEvent::PointerUp(hit));

              let tap_on = self
                .pointer_down_uid
                .take()?
                .lowest_common_ancestor(hit, &tree.arena)?;
              wnd.add_delay_event(DelayEvent::Tap(tap_on));
              Some(())
            };

            dispatch(&wnd.widget_tree.borrow());
          }
        }
      };
    }
  }

  pub fn dispatch_wheel(&mut self, delta: MouseScrollDelta, wnd_factor: f64) {
    if let Some(wid) = self.hit_widget() {
      let (delta_x, delta_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (x * PIXELS_PER_EM, y * PIXELS_PER_EM),
        MouseScrollDelta::PixelDelta(delta) => {
          let winit::dpi::LogicalPosition { x, y } = delta.to_logical(wnd_factor);
          (x, y)
        }
      };

      self
        .window()
        .add_delay_event(DelayEvent::Wheel { id: wid, delta_x, delta_y });
    }
  }

  fn bubble_pointer_down(&mut self) {
    let hit = self.hit_widget();
    self.pointer_down_uid = hit;
    let wnd = self.window();
    let tree = wnd.widget_tree.borrow();

    let nearest_focus = self.pointer_down_uid.and_then(|wid| {
      wid.ancestors(&tree.arena).find(|id| {
        id.get(&tree.arena)
          .map_or(false, |w| w.contain_type::<FocusNode>())
      })
    });
    if let Some(focus_id) = nearest_focus {
      self.window().focus_mgr.borrow_mut().focus(focus_id);
    } else {
      self.window().focus_mgr.borrow_mut().blur();
    }
    if let Some(hit) = hit {
      wnd.add_delay_event(DelayEvent::PointerDown(hit));
    }
  }

  fn pointer_enter_leave_dispatch(&mut self) {
    let new_hit = self.hit_widget();
    let wnd = self.window();
    let tree = wnd.widget_tree.borrow();

    let old = self
      .entered_widgets
      .iter()
      .find(|wid| !(*wid).is_dropped(&tree.arena))
      .copied();

    if let Some(old) = old {
      let ancestor = new_hit.and_then(|w| w.common_ancestors(old, &tree.arena).next());
      wnd.add_delay_event(DelayEvent::PointerLeave { bottom: old, up: ancestor });
    };

    if let Some(new) = new_hit {
      let ancestor = old.and_then(|o| o.common_ancestors(new, &tree.arena).next());
      wnd.add_delay_event(DelayEvent::PointerEnter { bottom: new, up: ancestor });
    }

    self.entered_widgets =
      new_hit.map_or(vec![], |wid| wid.ancestors(&tree.arena).collect::<Vec<_>>());
  }

  fn hit_widget(&self) -> Option<WidgetId> {
    let mut hit_target = None;
    let wnd = self.window();
    let tree = wnd.widget_tree.borrow();
    let arena = &tree.arena;
    let store = &tree.store;

    let mut w = Some(tree.root());
    let mut pos = self.info.cursor_pos;
    while let Some(id) = w {
      let r = id.assert_get(arena);
      let ctx = HitTestCtx { id, wnd: &wnd };
      let HitTest { hit, can_hit_child } = r.hit_test(&ctx, pos);

      pos = tree.store.map_from_parent(id, pos, arena);

      if hit {
        hit_target = w;
      }

      w = id
        .last_child(&tree.arena)
        .filter(|_| can_hit_child)
        .or_else(|| {
          if hit {
            return None;
          }

          pos = store.map_to_parent(id, pos, arena);

          let mut node = w;
          while let Some(p) = node {
            node = p.previous_sibling(&tree.arena);
            if node.is_some() {
              break;
            } else {
              node = p.parent(&tree.arena);

              if let Some(id) = node {
                pos = store.map_to_parent(id, pos, arena);
                if node == hit_target {
                  node = None;
                }
              }
            }
          }
          node
        });
    }
    hit_target
  }
}

impl DispatchInfo {
  #[inline]
  pub fn modifiers(&self) -> ModifiersState { self.modifiers }

  #[inline]
  pub fn global_pos(&self) -> Point { self.cursor_pos }

  #[inline]
  pub fn mouse_buttons(&self) -> MouseButtons { self.mouse_button.1 }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_helper::*;
  use std::{cell::RefCell, rc::Rc};
  use winit::event::WindowEvent;
  use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

  struct Info {
    pos: Point,
    btns: MouseButtons,
  }

  fn record_pointer(event_stack: Rc<RefCell<Vec<Info>>>, widget: Widget) -> Widget {
    let handler_ctor = move || {
      let stack = event_stack.clone();

      move |e: &mut PointerEvent| {
        stack.borrow_mut().push(Info {
          pos: e.position(),
          btns: e.mouse_buttons(),
        })
      }
    };
    widget! {
      DynWidget {
        dyns: widget,
        on_pointer_down : handler_ctor(),
        on_pointer_move: handler_ctor(),
        on_pointer_up: handler_ctor(),
        on_pointer_cancel: handler_ctor(),
      }
    }
    .into()
  }

  #[test]
  fn mouse_pointer_bubble() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let event_record = Rc::new(RefCell::new(vec![]));
    let record = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } }.into(),
    );
    let root = record_pointer(
      event_record.clone(),
      widget! { MockMulti { DynWidget  { dyns: record } } }.into(),
    );
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1., 1.).into(),
      modifiers: ModifiersState::default(),
    });

    {
      let mut records = event_record.borrow_mut();
      assert_eq!(records.len(), 2);
      assert_eq!(records[0].btns.bits().count_ones(), 0);
      records.clear();
    }

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    let mut records = event_record.borrow_mut();
    assert_eq!(records[0].btns.bits().count_ones(), 1);
    assert_eq!(records[0].pos, (1., 1.).into());
    records.clear();
  }

  #[test]
  fn mouse_buttons() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } }.into(),
    );
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Right,
      modifiers: ModifiersState::default(),
    });

    let records = event_record.borrow();
    assert_eq!(records.len(), 3);

    assert_eq!(records[0].btns, MouseButtons::PRIMARY);
    assert_eq!(
      records[1].btns,
      MouseButtons::PRIMARY | MouseButtons::SECONDARY
    );
    assert_eq!(records[2].btns, MouseButtons::default());
  }

  // Can not mock two different device id for macos.
  #[cfg(not(target_os = "macos"))]
  #[test]
  fn different_device_mouse() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let event_record = Rc::new(RefCell::new(vec![]));
    let root = record_pointer(
      event_record.clone(),
      widget! { MockBox { size: Size::new(100., 30.) } }.into(),
    );
    let mut wnd = TestWindow::new(root);
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);

    // A mouse press/release emit during another mouse's press will be ignored.
    let device_id_2 = unsafe {
      let mut id = DeviceId::dummy();
      (&mut id as *mut DeviceId).write_bytes(1, 1);
      id
    };
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: device_id_2,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });
    assert_eq!(event_record.borrow().len(), 1);

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id: device_id_2,
      position: (1, 1).into(),
      modifiers: ModifiersState::default(),
    });

    // but cursor move processed.
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow().len(), 2);
    assert_eq!(event_record.borrow()[1].btns, MouseButtons::PRIMARY);

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 3);
  }

  #[test]
  fn cancel_bubble() {
    let _guard = unsafe { AppCtx::new_lock_scope() };
    #[derive(Default)]
    struct EventRecord(Rc<RefCell<Vec<WidgetId>>>);
    impl Compose for EventRecord {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: INFINITY_SIZE,
            on_pointer_down: move |e| { this.0.borrow_mut().push(e.current_target()); },

            MockBox {
              size: Size::new(100., 30.),
              on_pointer_down: move |e| {
                this.0.borrow_mut().push(e.current_target());
                e.stop_propagation();
              }
            }
          }
        }
        .into()
      }
    }

    let root = EventRecord::default();
    let event_record = root.0.clone();

    let mut wnd = TestWindow::new_with_size(root, Size::new(100., 100.));
    wnd.draw_frame();

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id: unsafe { DeviceId::dummy() },
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers: ModifiersState::default(),
    });

    assert_eq!(event_record.borrow().len(), 1);
  }

  #[test]
  fn enter_leave() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    #[derive(Default)]
    struct EnterLeave {
      enter: Rc<RefCell<Vec<i32>>>,
      leave: Rc<RefCell<Vec<i32>>>,
    }

    impl Compose for EnterLeave {
      fn compose(this: State<Self>) -> Widget {
        widget! {
          states { this: this.into_writable() }
          MockBox {
            size: INFINITY_SIZE,
            on_pointer_enter: move |_| { this.enter.borrow_mut().push(2); },
            on_pointer_leave: move |_| { this.leave.borrow_mut().push(2); },
            MockBox {
              margin: EdgeInsets::all(4.),
              size: INFINITY_SIZE,
              on_pointer_enter: move |_| { this.enter.borrow_mut().push(1); },
              on_pointer_leave: move |_| { this.leave.borrow_mut().push(1); }
            }
          }
        }
        .into()
      }
    }

    let w = EnterLeave::default();
    let enter_event = w.enter.clone();
    let leave_event = w.leave.clone();

    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);

    // leave to parent
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (99, 99).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*leave_event.borrow(), &[1]);

    // move in same widget,
    // check if duplicate event fired.
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (99, 99).into(),
      modifiers: ModifiersState::default(),
    });
    assert_eq!(&*enter_event.borrow(), &[2, 1]);
    assert_eq!(&*leave_event.borrow(), &[1]);

    // leave all
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (999, 999).into(),
      modifiers: ModifiersState::default(),
    });

    assert_eq!(&*leave_event.borrow(), &[1, 2]);

    // leave event trigger by window left.
    leave_event.borrow_mut().clear();
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (10, 10).into(),
      modifiers: ModifiersState::default(),
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorLeft { device_id });
    assert_eq!(&*leave_event.borrow(), &[1, 2]);
  }

  #[test]
  fn capture_click() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let click_path = Stateful::new(vec![]) as Stateful<Vec<usize>>;
    let w = widget! {
      states { click_path: click_path.clone() }
      MockBox {
        size: Size::new(100., 100.),
        on_tap: move |_| (*click_path).push(4),
        on_tap_capture: move |_| (*click_path).push(1),
        MockBox {
          size: Size::new(100., 100.),
          on_tap: move |_| (*click_path).push(3),
          on_tap_capture: move |_| (*click_path).push(2),
        }
      }
    };

    // Stretch row
    let mut wnd = TestWindow::new_with_size(w, Size::new(400., 400.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });

    {
      let clicked = click_path.state_ref();
      assert_eq!(*clicked, [1, 2, 3, 4]);
    }
  }

  #[test]
  fn click() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let click_path = Stateful::new(0);
    let w = widget! {
      states { click_path: click_path.clone() }
      MockMulti {
        on_tap: move |_| *click_path += 1,
        MockBox {
          size: Size::new(100., 100.),
          on_tap: move |_| *click_path += 1,
        }
        MockBox { size: Size::new(100., 400.) }
      }
    };

    // Stretch row
    let mut wnd = TestWindow::new_with_size(w, Size::new(400., 400.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });

    {
      let mut clicked = click_path.state_ref();
      assert_eq!(*clicked, 2);
      *clicked = 0;
    }

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 50f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (50f64, 150f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });

    {
      let clicked = click_path.state_ref();
      assert_eq!(*clicked, 1);
    }
  }

  #[test]
  fn focus_change_by_event() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = widget! {
      MockMulti {
        MockBox {
          size: Size::new(50., 50.),
          tab_index: 0
        }
        MockBox {
          size: Size::new(50., 50.)
        }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(100., 100.));
    wnd.draw_frame();

    let device_id = unsafe { DeviceId::dummy() };
    let modifiers = ModifiersState::default();
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (45f64, 45f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    // point down on a focus widget
    assert!(wnd.focus_mgr.borrow().focusing().is_some());

    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Released,
      button: MouseButton::Left,
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::CursorMoved {
      device_id,
      position: (80f64, 80f64).into(),
      modifiers,
    });
    #[allow(deprecated)]
    wnd.processes_native_event(WindowEvent::MouseInput {
      device_id,
      state: ElementState::Pressed,
      button: MouseButton::Left,
      modifiers,
    });

    assert!(wnd.focus_mgr.borrow().focusing().is_none());
  }

  #[test]
  fn fix_hit_out_window() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    let w = MockBox { size: INFINITY_SIZE };
    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();
    let mut dispatcher = wnd.dispatcher.borrow_mut();
    dispatcher.info.cursor_pos = Point::new(-1., -1.);
    let hit = dispatcher.hit_widget();

    assert_eq!(hit, None);
  }

  #[test]
  fn hit_test_case() {
    let _guard = unsafe { AppCtx::new_lock_scope() };

    fn normal_mode_search() {
      struct T {
        pub wid1: Option<WidgetId>,
        pub wid2: Option<WidgetId>,
      }
      let data = Rc::new(RefCell::new(T { wid1: None, wid2: None }));
      let data1 = data.clone();
      let data2 = data.clone();

      let w = widget! {
        MockBox {
          size: Size::new(200., 200.),
          MockStack {
            child_pos: vec![
              Point::new(50., 50.),
              Point::new(100., 100.),
            ],
            MockBox {
              on_mounted: move |ctx| {
                data1.borrow_mut().wid1 = Some(ctx.id);
              },
              size: Size::new(100., 100.),
            }
            MockBox {
              on_mounted: move |ctx| {
                data2.borrow_mut().wid2 = Some(ctx.id);
              },
              size: Size::new(50., 150.),
            }
          }
        }
      };

      let mut wnd = TestWindow::new_with_size(w, Size::new(500., 500.));
      wnd.draw_frame();
      let mut dispatcher = wnd.dispatcher.borrow_mut();
      dispatcher.info.cursor_pos = Point::new(125., 125.);
      let hit_2 = dispatcher.hit_widget();
      assert_eq!(hit_2, data.borrow().wid2);

      dispatcher.info.cursor_pos = Point::new(80., 80.);
      let hit_1 = dispatcher.hit_widget();
      assert_eq!(hit_1, data.borrow().wid1);
    }

    normal_mode_search();
  }
}
