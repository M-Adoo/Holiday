use crate::widget::*;

use blake3;
use std::{
  cmp::{Eq, Ord, PartialOrd},
  fmt::Debug,
};

/// Abstract all builtin key into a same type.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum Key {
  KUsize(usize),
  KU1(u8),
  KU2(u16),
  KU4(u32),
  KU8(u64),
  KU16(u128),

  KIsize(isize),
  KI1(i8),
  KI2(i16),
  KI4(i32),
  KI8(i64),
  KI16(i128),

  KBool(bool),
  KChar(char),

  KString(String),
  K32([u8; 32]),
}

#[derive(Debug)]
pub struct KeyDetect<W> {
  key: Key,
  child: W,
}

#[derive(Debug)]
pub struct KeyRender<R>(R);

impl<W> KeyDetect<W> {
  pub fn new<K>(key: K, child: W) -> Self
  where
    K: Into<Key>,
  {
    KeyDetect {
      key: key.into(),
      child,
    }
  }
}

impl<W> CombinationWidget for KeyDetect<W>
where
  W: CombinationWidget,
{
  #[inline]
  fn key(&self) -> Option<&Key> { Some(&self.key) }

  #[inline]
  fn build<'a>(&self) -> Widget<'a> { self.child.build() }
}

impl<W> RenderWidget for KeyDetect<W>
where
  W: RenderWidget,
{
  type RO = KeyRender<W::RO>;
  fn key(&self) -> Option<&Key> { Some(&self.key) }
  fn create_render_object(&self) -> Self::RO {
    KeyRender(self.child.create_render_object())
  }
}

impl<W> SingleChildWidget for KeyDetect<W>
where
  W: SingleChildWidget + RenderWidget,
  W::RO: Send + Sync + 'static,
{
  #[inline]
  fn take_child<'a>(&mut self) -> Widget<'a>
  where
    Self: 'a,
  {
    self.child.take_child()
  }
}

impl<W> MultiChildWidget for KeyDetect<W>
where
  W: MultiChildWidget + RenderWidget,
  W::RO: Send + Sync + 'static,
{
  #[inline]
  fn take_children<'a>(&mut self) -> Vec<Widget<'a>>
  where
    Self: 'a,
  {
    self.child.take_children()
  }
}

impl<W> RenderObject<KeyDetect<W>> for KeyRender<W::RO>
where
  W: RenderWidget,
{
  fn update(&mut self, owner_widget: &KeyDetect<W>) {
    self.0.update(&owner_widget.child)
  }
}

macro from_key_impl($($ty: ty : $name: ident)*) {
  $(
    impl From<$ty> for Key {
      fn from(s: $ty) -> Self {
        Key::$name(s)
      }
    }
  )*
}

from_key_impl!(
  usize:KUsize u8:KU1 u16:KU2 u32:KU4 u64:KU8 u128:KU16
  isize:KIsize i8:KI1 i16:KI2 i32:KI4 i64:KI8 i128:KI16
  bool:KBool char:KChar
  [u8;32]:K32
);

const MAX_KEY_STR: usize = 16;

impl From<String> for Key {
  fn from(s: String) -> Self {
    if s.len() < MAX_KEY_STR {
      Key::KString(s)
    } else {
      Key::K32(blake3::hash(s.as_bytes()).into())
    }
  }
}

impl From<&str> for Key {
  fn from(s: &str) -> Self {
    if s.len() < MAX_KEY_STR {
      Key::KString(s.to_owned())
    } else {
      Key::K32(blake3::hash(s.as_bytes()).into())
    }
  }
}

pub macro complex_key($($k: expr),*) {
  {
    let mut hasher = blake3::Hasher::new();
    $(
      $k.consume(&mut hasher);
    )*
    let bytes: [u8;32] = hasher.finalize().into();
    bytes
  }
}

trait ConsumeByHasher {
  fn consume(self, hasher: &mut blake3::Hasher);
}

impl ConsumeByHasher for String {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) {
    hasher.update(self.as_bytes());
  }
}

impl<'a> ConsumeByHasher for &'a str {
  #[inline]
  fn consume(self, hasher: &mut blake3::Hasher) {
    hasher.update(self.as_bytes());
  }
}

macro impl_as_u8_consume_by_hasher($($t: ty)*) {
  $(
    impl ConsumeByHasher for $t {
      #[inline]
      fn consume(self, hasher: &mut blake3::Hasher) {
        hasher.update(&[self as u8]);
      }
    }
  )*
}
impl_as_u8_consume_by_hasher!(bool char);

macro impl_bytes_consume_by_hasher($($ty: ty)*) {
  $(
    impl ConsumeByHasher for $ty {
      #[inline]
      fn consume(self, hasher: &mut blake3::Hasher) {
        hasher.update(&self.to_ne_bytes());
      }
    }
  )*
}

impl_bytes_consume_by_hasher!(
  usize u8 u16 u32 u64 u128
  isize i8 i16 i32 i64 i128
  f32 f64
);

#[test]
fn key_detect() {
  let k1 = KeyDetect::new(0, Text(""));
  let k2 = KeyDetect::new(String::new(), Text(""));
  let k3 = KeyDetect::new("", Text(""));
  let ck1 = KeyDetect::new(complex_key!("asd", true, 1), Text(""));
  let ck2 = KeyDetect::new(complex_key!("asd", true, 1), Text(""));
  assert!(&k1.key != &k2.key);
  assert!(&k2.key == &k3.key);
  assert!(&k3.key != &k1.key);
  assert!(ck1.key == ck2.key);
}
