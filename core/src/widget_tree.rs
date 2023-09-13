use std::{
  cell::RefCell,
  cmp::Reverse,
  collections::HashSet,
  mem::MaybeUninit,
  rc::{Rc, Weak},
};

pub mod widget_id;
pub(crate) use widget_id::TreeArena;
pub use widget_id::WidgetId;
mod layout_info;
use crate::prelude::*;
pub use layout_info::*;

use self::widget_id::{empty_node, RenderNode};

pub(crate) type DirtySet = Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>;

pub(crate) struct WidgetTree {
  pub(crate) root: WidgetId,
  wnd: Weak<Window>,
  pub(crate) arena: TreeArena,
  pub(crate) store: LayoutStore,
  pub(crate) dirty_set: DirtySet,
}

impl WidgetTree {
  pub fn init(&mut self, wnd: Weak<Window>) { self.wnd = wnd; }

  pub fn set_root(&mut self, root: WidgetId) {
    self.root = root;
    self.mark_dirty(self.root);
    self.root.on_mounted_subtree(self);
  }

  pub(crate) fn root(&self) -> WidgetId { self.root }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self) {
    let wnd = self.window();
    let mut painter = wnd.painter.borrow_mut();
    let mut ctx = PaintingCtx::new(self.root(), wnd.id(), &mut painter);
    self.root().paint_subtree(&mut ctx);
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    loop {
      let Some(mut needs_layout) = self.layout_list() else {break;};
      while let Some(wid) = needs_layout.pop() {
        if wid.is_dropped(&self.arena) {
          continue;
        }

        let clamp = self
          .store
          .layout_info(wid)
          .map(|info| info.clamp)
          .unwrap_or_else(|| BoxClamp { min: Size::zero(), max: win_size });

        let mut layouter = Layouter::new(wid, self.window().id(), true, self);
        layouter.perform_widget_layout(clamp);
      }
    }
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.dirty_set.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool { !self.dirty_set.borrow().is_empty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self.arena).count() }

  pub(crate) fn window(&self) -> Rc<Window> {
    self
      .wnd
      .upgrade()
      .expect("Must initialize the widget tree before use it.")
  }

  #[allow(unused)]
  pub fn display_tree(&self, sub_tree: WidgetId) -> String {
    fn display_node(mut prefix: String, id: WidgetId, tree: &TreeArena, display: &mut String) {
      display.push_str(&format!("{prefix}{:?}\n", id.0));

      prefix.pop();
      match prefix.pop() {
        Some('├') => prefix.push_str("│ "),
        Some(_) => prefix.push_str("  "),
        _ => {}
      }

      id.children(tree).for_each(|c| {
        let mut prefix = prefix.clone();
        let suffix = if Some(c) == id.last_child(tree) {
          "└─"
        } else {
          "├─"
        };
        prefix.push_str(suffix);
        display_node(prefix, c, tree, display)
      });
    }
    let mut display = String::new();
    display_node("".to_string(), sub_tree, &self.arena, &mut display);
    display
  }

  pub(crate) fn layout_list(&mut self) -> Option<Vec<WidgetId>> {
    if self.dirty_set.borrow().is_empty() {
      return None;
    }

    let mut needs_layout = vec![];

    let dirty_widgets = {
      let mut state_changed = self.dirty_set.borrow_mut();
      let dirty_widgets = state_changed.clone();
      state_changed.clear();
      dirty_widgets
    };

    for id in dirty_widgets.iter() {
      if id.is_dropped(&self.arena) {
        continue;
      }

      let mut relayout_root = *id;
      if let Some(info) = self.store.get_mut(id) {
        info.size.take();
      }

      // All ancestors of this render widget should relayout until the one which only
      // sized by parent.
      for p in id.0.ancestors(&self.arena).skip(1).map(WidgetId) {
        if self.store.layout_box_size(p).is_none() {
          break;
        }

        relayout_root = p;
        if let Some(info) = self.store.get_mut(&p) {
          info.size.take();
        }

        let r = p.assert_get(&self.arena);
        if r.only_sized_by_parent() {
          break;
        }
      }
      needs_layout.push(relayout_root);
    }

    (!needs_layout.is_empty()).then(|| {
      needs_layout.sort_by_cached_key(|w| Reverse(w.ancestors(&self.arena).count()));
      needs_layout
    })
  }

  pub fn detach(&mut self, id: WidgetId) {
    if self.root() == id {
      let root = self.root();
      let new_root = root
        .next_sibling(&self.arena)
        .or_else(|| root.prev_sibling(&self.arena))
        .expect("Try to remove the root and there is no other widget can be the new root.");
      self.root = new_root;
    }

    id.0.detach(&mut self.arena);
  }

  pub(crate) fn remove_subtree(&mut self, id: WidgetId) {
    assert_ne!(
      id,
      self.root(),
      "You should detach the root widget before remove it."
    );

    id.descendants(&self.arena).for_each(|id| {
      self.store.remove(id);
    });
    id.0.remove_subtree(&mut self.arena);
  }

  pub(crate) fn get_many_mut<const N: usize>(
    &mut self,
    ids: &[WidgetId; N],
  ) -> [&mut RenderNode; N] {
    unsafe {
      let mut outs: MaybeUninit<[&mut RenderNode; N]> = MaybeUninit::uninit();
      let outs_ptr = outs.as_mut_ptr();
      for (idx, wid) in ids.iter().enumerate() {
        let arena = &mut *(&mut self.arena as *mut TreeArena);
        let cur = wid.get_node_mut(arena).expect("Invalid widget id.");

        *(*outs_ptr).get_unchecked_mut(idx) = cur;
      }
      outs.assume_init()
    }
  }
}

impl Default for WidgetTree {
  fn default() -> Self {
    let mut arena = TreeArena::new();
    Self {
      root: empty_node(&mut arena),
      wnd: Weak::new(),
      arena,
      store: LayoutStore::default(),
      dirty_set: Rc::new(RefCell::new(HashSet::default())),
    }
  }
}

#[cfg(test)]
mod tests {
  extern crate test;
  use crate::{
    reset_test_env,
    test_helper::{MockBox, MockMulti, TestWindow},
    widget::widget_id::empty_node,
  };

  use super::*;
  use test::Bencher;

  #[derive(Clone, Debug)]
  pub struct Recursive {
    pub width: usize,
    pub depth: usize,
  }

  impl Compose for Recursive {
    fn compose(this: State<Self>) -> Widget {
      fn_widget! {
        @MockMulti {
          @{
            pipe!(($this.width, $this.depth))
              .map(|(width, depth)| {
                (0..width).map(move |_| -> Widget {
                  if depth > 1 {
                    Recursive { width, depth: depth - 1 }.into()
                  } else {
                    MockBox { size: Size::new(10., 10.)}.into()
                  }
                })
              })
          }
         }
      }
      .into()
    }
  }

  #[derive(Clone, Debug)]
  pub struct Embed {
    pub width: usize,
    pub depth: usize,
  }

  impl Compose for Embed {
    fn compose(this: State<Self>) -> Widget {
      fn_widget! {
        let recursive_child: Widget = if $this.depth > 1 {
          let width = $this.width;
          let depth = $this.depth - 1;
          Embed { width, depth }.into()
        } else {
          MockBox { size: Size::new(10., 10.) }.into()
        };
        @MockMulti {
          @{ pipe!{
            (0..$this.width - 1)
              .map(|_| MockBox { size: Size::new(10., 10.)})
          }}
          @{ recursive_child }
        }
      }
      .into()
    }
  }

  fn bench_recursive_inflate(width: usize, depth: usize, b: &mut Bencher) {
    let mut wnd = TestWindow::new(Void {});
    b.iter(move || {
      wnd.set_content_widget(Recursive { width, depth }.into());
      wnd.draw_frame();
    });
  }

  fn bench_recursive_repair(width: usize, depth: usize, b: &mut Bencher) {
    let w = Stateful::new(Recursive { width, depth });
    let trigger = w.clone_writer();
    let mut wnd = TestWindow::new(w);
    b.iter(|| {
      {
        let _ = trigger.write();
      }
      wnd.draw_frame();
    });
  }

  #[test]
  fn fix_relayout_incorrect_clamp() {
    reset_test_env!();

    let expect_size = Size::new(20., 20.);
    let no_boundary_size = Stateful::new(INFINITY_SIZE);
    let c_size = no_boundary_size.clone_writer();
    let w = fn_widget! {
      @MockBox {
        size: expect_size,
        @MockBox { size: pipe!(*$no_boundary_size) }
      }
    };
    let mut wnd = TestWindow::new_with_size(w, Size::new(200., 200.));
    wnd.draw_frame();
    let size = wnd.layout_info_by_path(&[0, 0]).unwrap().size.unwrap();
    assert_eq!(size, expect_size);

    // when relayout the inner `MockBox`, its clamp should same with its previous
    // layout, and clamp its size.
    {
      *c_size.write() = INFINITY_SIZE;
    }
    wnd.draw_frame();
    let size = wnd.layout_info_by_path(&[0, 0]).unwrap().size.unwrap();
    assert_eq!(size, expect_size);
  }

  #[test]
  fn fix_dropped_child_expr_widget() {
    reset_test_env!();

    let parent = Stateful::new(true);
    let child = Stateful::new(true);
    let c_p = parent.clone_writer();
    let c_c = child.clone_writer();
    let w = fn_widget! {
      @ {
        pipe!(*$parent).map(move |p|{
          p.then(|| @MockBox {
            size: Size::zero(),
            @ { pipe!($child.then(|| Void)) }
          })
        })
      }
    };

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    {
      *c_c.write() = false;
      *c_p.write() = false;
    }

    // fix crash here.
    wnd.draw_frame();
  }

  #[test]
  fn fix_child_expr_widget_same_root_as_parent() {
    reset_test_env!();

    let trigger = Stateful::new(true);
    let c_trigger = trigger.clone_writer();
    let w = fn_widget! { @ { pipe!($trigger.then(|| Void)) }};

    let mut wnd = TestWindow::new(w);
    wnd.draw_frame();

    {
      *c_trigger.write() = false;
    }

    // fix crash here
    // crash because generator live as long as its parent, at here two expr widget's
    // parent both none, all as root expr widget, parent expr widget can't remove
    // child expr widget.
    //
    // generator lifetime should bind to its generator widget instead of parent.
    wnd.draw_frame();
  }
  #[test]
  fn drop_info_clear() {
    reset_test_env!();

    let post = Embed { width: 5, depth: 3 };
    let wnd = TestWindow::new(post);
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::new(512., 512.));
    assert_eq!(tree.count(), 16);

    let root = tree.root();
    tree.mark_dirty(root);
    let new_root = empty_node(&mut tree.arena);
    root.insert_after(new_root, &mut tree.arena);
    tree.mark_dirty(new_root);
    tree.detach(root);
    tree.remove_subtree(root);

    assert_eq!(tree.layout_list(), Some(vec![new_root]));
  }

  #[bench]
  fn new_5_x_1000(b: &mut Bencher) {
    reset_test_env!();

    let wnd = TestWindow::new(Void);

    b.iter(move || {
      let post = Embed { width: 5, depth: 1000 };
      wnd.set_content_widget(post.into());
    });
  }

  #[bench]
  fn new_50_pow_2(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_inflate(50, 2, b);
  }

  #[bench]
  fn new_100_pow_2(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_inflate(100, 2, b);
  }

  #[bench]
  fn new_10_pow_4(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_inflate(10, 4, b);
  }

  #[bench]
  fn new_10_pow_5(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_inflate(10, 5, b);
  }

  #[bench]
  fn regen_5_x_1000(b: &mut Bencher) {
    reset_test_env!();

    let post = Stateful::new(Embed { width: 5, depth: 1000 });
    let trigger = post.clone_writer();
    let wnd = TestWindow::new(post);
    let mut tree = wnd.widget_tree.borrow_mut();

    b.iter(|| {
      {
        let _ = trigger.write();
      }
      tree.layout(Size::new(512., 512.));
    });
  }

  #[bench]
  fn regen_50_pow_2(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_repair(50, 2, b);
  }

  #[bench]
  fn regen_100_pow_2(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_repair(100, 2, b);
  }

  #[bench]
  fn regen_10_pow_4(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_repair(10, 4, b);
  }

  #[bench]
  fn regen_10_pow_5(b: &mut Bencher) {
    reset_test_env!();

    bench_recursive_repair(10, 5, b);
  }

  #[test]
  fn perf_silent_ref_should_not_dirty_expr_widget() {
    reset_test_env!();

    let trigger = Stateful::new(1);
    let c_trigger = trigger.clone_stateful();
    let widget = fn_widget! {
      @ MockMulti {
        @{
          pipe!(*$trigger).map(move |b| {
            let size = if b > 0 {
              Size::new(1., 1.)
            } else {
              Size::zero()
            };
            (0..3).map(move |_| MockBox { size })
          })
        }
      }
    };

    let wnd = TestWindow::new(widget);
    let mut tree = wnd.widget_tree.borrow_mut();
    tree.layout(Size::new(100., 100.));
    {
      *c_trigger.write() = 2;
    }
    assert!(!tree.is_dirty())
  }

  #[test]
  fn draw_clip() {
    reset_test_env!();

    let win_size = Size::new(150., 50.);

    let w1 = fn_widget! {
       @MockMulti {
        @ {
          (0..100).map(|_|
            widget! { MockBox {
            size: Size::new(150., 50.),
            background: Color::BLUE,
          }})
        }
    }};
    let mut wnd = TestWindow::new_with_size(w1, win_size);
    wnd.draw_frame();

    let len_100_widget = wnd.painter.borrow_mut().finish().len();

    let w2 = fn_widget! {
      @MockMulti {
        @ {
          (0..1).map(|_|
            widget! { MockBox {
             size: Size::new(150., 50.),
             background: Color::BLUE,
          }})
        }
    }};

    let mut wnd = TestWindow::new(w2);
    wnd.draw_frame();
    let len_1_widget = wnd.painter.borrow_mut().finish().len();
    assert_eq!(len_1_widget, len_100_widget);
  }
}
