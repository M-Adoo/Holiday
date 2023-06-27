use futures::{
  task::{LocalSpawnExt, SpawnError},
  Future,
};
use ribir_geom::Point;
use ribir_text::TypographyStore;
use rxrust::{scheduler::FuturesLocalScheduler, subject::Subject};
use std::{cell::RefCell, convert::Infallible, rc::Rc, time::Instant};

use super::AppContext;
use crate::{
  animation::AnimateTrack,
  events::focus_mgr::{FocusHandle, FocusManager, FocusType},
  ticker::{FrameMsg, FrameTicker},
  widget::{TreeArena, WidgetId},
};

#[derive(Clone)]
pub struct WindowCtx {
  pub(crate) frame_ticker: FrameTicker,
  pub(crate) focus_mgr: Rc<RefCell<FocusManager>>,
  pub(crate) app_ctx: AppContext,
  pub(crate) actived_animates: Rc<RefCell<u32>>,
  pub(crate) frame_scheduler: FuturesLocalScheduler,
  pub(crate) ime_pos: Rc<RefCell<Point>>,
}

impl WindowCtx {
  pub fn new(app_ctx: AppContext, frame_scheduler: FuturesLocalScheduler) -> Self {
    Self {
      app_ctx,
      focus_mgr: Rc::new(RefCell::new(FocusManager::default())),
      frame_ticker: FrameTicker::default(),
      actived_animates: Rc::new(RefCell::new(0)),
      ime_pos: Rc::new(RefCell::new(Point::zero())),
      frame_scheduler,
    }
  }

  #[inline]
  pub fn app_ctx(&self) -> &AppContext { &self.app_ctx }

  /// Return an local `rxRust` Scheduler
  #[inline]
  pub fn scheduler(&self) -> FuturesLocalScheduler { self.app_ctx.scheduler() }

  /// Return an `rxRust` Scheduler, which will guarantee all task add to the
  /// scheduler will finished before current frame finished.
  #[inline]
  pub fn frame_scheduler(&self) -> FuturesLocalScheduler { self.frame_scheduler.clone() }

  /// Spawns a task that polls the given future with output `()` to completion.
  /// And guarantee wait this task will finished in current frame.
  pub fn frame_spawn(&self, f: impl Future<Output = ()> + 'static) -> Result<(), SpawnError> {
    self.frame_scheduler.spawn_local(f)
  }

  pub fn typography_store(&self) -> &TypographyStore { &self.app_ctx.typography_store }

  pub fn frame_tick_stream(&self) -> Subject<'static, FrameMsg, Infallible> {
    self.frame_ticker.frame_tick_stream()
  }

  pub(crate) fn begin_frame(&mut self) {
    self.frame_ticker.emit(FrameMsg::NewFrame(Instant::now()));
  }

  pub(crate) fn layout_ready(&mut self) {
    self
      .frame_ticker
      .emit(FrameMsg::LayoutReady(Instant::now()));
  }

  pub(crate) fn end_frame(&mut self) {
    self.app_ctx.end_frame();
    self.frame_ticker.emit(FrameMsg::Finish(Instant::now()));
  }

  pub(crate) fn next_focus(&self, arena: &TreeArena) {
    self.focus_mgr.borrow_mut().request_next_focus(arena);
  }

  pub(crate) fn prev_focus(&self, arena: &TreeArena) {
    self.focus_mgr.borrow_mut().request_prev_focus(arena);
  }

  pub(crate) fn focus_handle(&self, wid: WidgetId) -> FocusHandle {
    FocusManager::focus_handle(&self.focus_mgr, wid)
  }

  pub(crate) fn add_focus_node(
    &self,
    wid: WidgetId,
    auto_focus: bool,
    focus_type: FocusType,
    arena: &TreeArena,
  ) {
    self
      .focus_mgr
      .borrow_mut()
      .add_focus_node(wid, auto_focus, focus_type, arena);
  }

  pub(crate) fn remove_focus_node(&self, wid: WidgetId, focus_tyep: FocusType) {
    self
      .focus_mgr
      .borrow_mut()
      .remove_focus_node(wid, focus_tyep);
  }

  pub(crate) fn has_actived_animate(&self) -> bool { *self.actived_animates.borrow() > 0 }

  pub fn animate_track(&self) -> AnimateTrack {
    AnimateTrack {
      actived: false,
      actived_cnt: self.actived_animates.clone(),
    }
  }

  pub(crate) fn set_ime_pos(&self, global_pos: Point) { *self.ime_pos.borrow_mut() = global_pos; }
}
