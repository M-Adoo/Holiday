use ribir::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase, WindowEvent};

fn wheel_widget(w: Widget) -> Window {
  let mut wnd = Window::default_mock(w, None);

  wnd.draw_frame();
  let device_id = unsafe { DeviceId::dummy() };
  wnd.processes_native_event(WindowEvent::MouseWheel {
    device_id,
    delta: MouseScrollDelta::LineDelta(1.0, 1.0),
    phase: TouchPhase::Started,
    modifiers: ModifiersState::default(),
  });
  wnd
}

#[test]
fn listener_trigger_have_handler() {
  let handler_call_times = Rc::new(Cell::new(0));
  let h1 = handler_call_times.clone();
  let animate_state = false.into_stateful();

  let w = widget! {
    states { animate_state:  animate_state.clone() }
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
      wheel: move |_| h1.set(h1.get() + 1),
    }
    Animate {
      id: leak_animate,
      from: State {
        sized_box.size: Size::zero(),
      },
      transition: Transition {
        easing: easing::LINEAR,
        duration: Duration::from_millis(100)
      }
    }
    on sized_box {
      wheel: move |_| leak_animate.run()
    }
    change_on leak_animate.is_running() ~> *animate_state
  };

  wheel_widget(w);

  assert_eq!(true, *animate_state.raw_ref());
  assert_eq!(handler_call_times.get(), 1);
}

#[test]
fn listener_trigger() {
  let animate_state = false.into_stateful();

  let w = widget! {
    states { animate_state:  animate_state.clone() }
    SizedBox {
      id: sized_box,
      size: Size::new(100., 100.),
    }
    Animate {
      id: leak_animate,
      from: State {
        sized_box.size: Size::zero(),
      },
      transition: Transition {
        easing: easing::LINEAR,
        duration: Duration::from_millis(100)
      }
    }
    on sized_box {
      wheel: move |_| leak_animate.run()
    }
    change_on leak_animate.is_running() ~> *animate_state
  };

  wheel_widget(w);

  assert_eq!(true, *animate_state.raw_ref());
}
