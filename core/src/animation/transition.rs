use crate::prelude::*;

use crate::prelude::BuildCtx;

use super::easing::Easing;
use std::rc::Rc;
use std::time::Duration;

/// Transition describe how the state change form init to final smoothly.
#[derive(Declare, Clone, Debug, PartialEq)]
pub struct Transition<E> {
  #[declare(default, convert=strip_option)]
  pub delay: Option<Duration>,
  pub duration: Duration,
  pub easing: E,
  #[declare(default, convert=strip_option)]
  pub repeat: Option<f32>,
}

/// Calc the rate of change over time.
pub trait Roc {
  /// Calc the rate of change of the duration from animation start.
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress;
}

impl<E: Easing> Roc for Transition<E> {
  fn rate_of_change(&self, mut run_dur: Duration) -> AnimateProgress {
    if self.duration.as_secs_f32().abs() < f32::EPSILON {
      return AnimateProgress::Finish;
    }

    if let Some(delay) = self.delay {
      if run_dur < delay {
        return AnimateProgress::Dismissed;
      } else {
        run_dur -= delay;
      }
    }

    let repeat = self.repeat.unwrap_or(1.);

    let rounds = run_dur.as_secs_f32() / self.duration.as_secs_f32();
    if rounds > repeat {
      return AnimateProgress::Finish;
    }

    let time_rate = (run_dur.as_secs_f32() - rounds.floor()) / self.duration.as_secs_f32();
    let p = self.easing.easing(time_rate);
    AnimateProgress::Between(p)
  }
}

impl<T: Roc> Roc for Stateful<T> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress {
    self.state_ref().rate_of_change(dur)
  }
}

impl Roc for Box<dyn Roc> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { self.deref().rate_of_change(dur) }
}

impl<T: Roc> Roc for Rc<T> {
  #[inline]
  fn rate_of_change(&self, dur: Duration) -> AnimateProgress { self.deref().rate_of_change(dur) }
}
