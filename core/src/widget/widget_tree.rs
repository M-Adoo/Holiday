use crate::prelude::*;
use ahash::RandomState;
use indextree::*;
use smallvec::SmallVec;
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
  rc::Rc,
};

pub(crate) mod animation_store;
mod generator_store;
mod layout_info;
use animation_store::AnimateStore;
pub use layout_info::*;

use super::Children;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]
pub struct WidgetId(NodeId);
pub(crate) struct WidgetTree {
  arena: Arena<Box<dyn Render>>,
  root: Option<WidgetId>,
  ctx: Rc<RefCell<AppContext>>,
  pub(crate) state_changed: Rc<RefCell<HashSet<WidgetId, ahash::RandomState>>>,
  /// Store the render object's place relative to parent coordinate and the
  /// clamp passed from parent.
  layout_store: HashMap<WidgetId, BoxLayout, ahash::RandomState>,
  pub(crate) generator_store: generator_store::GeneratorStore,
  pub(crate) animations_store: Rc<RefCell<AnimateStore>>,
}

impl WidgetTree {
  pub(crate) fn root(&self) -> WidgetId { self.root.expect("Empty tree.") }

  pub(crate) fn new_node(&mut self, node: Box<dyn Render>) -> WidgetId {
    WidgetId(self.arena.new_node(node))
  }

  pub(crate) fn empty_node(&mut self) -> WidgetId { self.new_node(Box::new(Void)) }

  pub(crate) fn new(root_widget: Widget, ctx: Rc<RefCell<AppContext>>) -> WidgetTree {
    let ticker = ctx.borrow().frame_ticker.frame_tick_stream();
    let animations_store = Rc::new(RefCell::new(AnimateStore::new(ticker)));
    let mut tree = WidgetTree {
      arena: Arena::default(),
      root: None,
      state_changed: <_>::default(),
      ctx,
      layout_store: <_>::default(),
      generator_store: <_>::default(),
      animations_store,
    };

    tree.set_root(root_widget);
    tree
  }

  /// Draw current tree by painter.
  pub(crate) fn draw(&self, painter: &mut Painter) {
    let mut w = Some(self.root());

    let mut paint_ctx = PaintingCtx::new(self.root(), self, painter);
    while let Some(id) = w {
      paint_ctx.id = id;
      let rect = paint_ctx
        .box_rect()
        .expect("when paint node, it's mut be already layout.");
      paint_ctx
        .painter
        .save()
        .translate(rect.min_x(), rect.min_y());
      let rw = id.assert_get(self);
      rw.paint(&mut paint_ctx);

      w = id
        // deep first.
        .first_child(self)
        // goto sibling or back to parent sibling
        .or_else(|| {
          let mut node = w;
          while let Some(p) = node {
            // self node sub-tree paint finished, goto sibling
            paint_ctx.painter.restore();
            node = p.next_sibling(self);
            if node.is_some() {
              break;
            } else {
              // if there is no more sibling, back to parent to find sibling.
              node = p.parent(self);
            }
          }
          node
        });
    }
  }

  /// Repair the gaps between widget tree represent and current data state after
  /// some user or device inputs has been processed.
  pub(crate) fn tree_repair(&mut self) {
    while let Some(mut needs_regen) = self.generator_store.take_needs_regen_generator() {
      needs_regen
        .sort_by_cached_key(|g| g.info.parent().map_or(0, |wid| wid.ancestors(self).count()));
      needs_regen.iter_mut().for_each(|g| {
        if !g.info.parent().map_or(false, |p| p.is_dropped(self)) {
          g.refresh(self);
        }
      });

      needs_regen
        .into_iter()
        .for_each(|g| self.generator_store.add_generator(g));
    }
  }

  /// Do the work of computing the layout for all node which need, Return if any
  /// node has really computing the layout.
  pub(crate) fn layout(&mut self, win_size: Size) {
    let mut performed = vec![];

    loop {
      if let Some(needs_layout) = self.layout_list() {
        needs_layout.iter().for_each(|wid| {
          let clamp = BoxClamp { min: Size::zero(), max: win_size };
          wid.perform_layout(clamp, self, &mut performed);
        });
      } else {
        break;
      }
    }

    performed.drain(..).for_each(|id| {
      let (tree1, tree2) = unsafe { self.split_tree() };
      id.assert_get_mut(tree1).query_all_type_mut(
        |l: &mut PerformedLayoutListener| {
          (l.on_performed_layout)(LifeCycleCtx { id, tree: tree2 });
          true
        },
        QueryOrder::OutsideFirst,
      );
    });
  }

  pub(crate) fn set_root(&mut self, widget: Widget) {
    assert!(self.root.is_none());

    let root = widget.into_subtree(None, self).expect("must have a root");
    self.set_root_id(root);
    root.on_mounted_subtree(self, true);
    self.mark_dirty(root);
  }

  pub(crate) fn set_root_id(&mut self, id: WidgetId) {
    assert!(self.root.is_none());
    self.root = Some(id);
  }

  pub(crate) fn swap_node_data(&mut self, a: WidgetId, b: WidgetId) {
    let a_node = std::mem::replace(a.assert_get_mut(self), Box::new(Void));
    let b_node = std::mem::replace(b.assert_get_mut(self), a_node);
    let _ = std::mem::replace(a.assert_get_mut(self), b_node);
  }

  pub(crate) fn mark_dirty(&self, id: WidgetId) { self.state_changed.borrow_mut().insert(id); }

  pub(crate) fn is_dirty(&self) -> bool {
    self.any_state_modified() || self.generator_store.is_dirty()
  }

  pub(crate) fn any_state_modified(&self) -> bool { !self.state_changed.borrow().is_empty() }

  pub(crate) fn any_struct_dirty(&self) -> bool { self.generator_store.is_dirty() }

  pub(crate) fn count(&self) -> usize { self.root().descendants(&self).count() }

  pub(crate) fn context(&self) -> &Rc<RefCell<AppContext>> { &self.ctx }

  /// #panic
  /// dst should not be empty.
  pub(crate) fn replace_children(
    &mut self,
    dst: &[WidgetId],
    widgets: Vec<Widget>,
  ) -> SmallVec<[WidgetId; 1]> {
    fn collect_same_key_pairs(
      old_widgets: impl Iterator<Item = WidgetId>,
      new_widgets: impl Iterator<Item = WidgetId>,
      tree: &WidgetTree,
      same_key_pairs: &mut Vec<(WidgetId, WidgetId)>,
    ) {
      let new_keys = new_widgets
        .filter_map(|n| n.key(tree).map(|k| (k, n)))
        .collect::<HashMap<_, _, RandomState>>();

      for o in old_widgets {
        if let Some(n) = o.key(tree).and_then(|k| new_keys.get(&k)) {
          same_key_pairs.push((o, *n));
          collect_same_key_pairs(o.children(tree), n.children(tree), tree, same_key_pairs);
        }
      }
    }

    let mut sign = *dst.last().expect("replace target at least have one widget");
    let parent = sign.parent(self);
    let mut new_widgets = widgets
      .into_iter()
      .flat_map(|w| w.into_subtree(parent, self))
      .collect::<SmallVec<[WidgetId; 1]>>();

    if new_widgets.is_empty() {
      // gen root at least have a void widget as road sign.
      new_widgets.push(self.empty_node());
    }

    for w in new_widgets.iter().cloned() {
      sign.insert_after(w, self);
      sign = w;
    }

    let mut same_key_pairs = vec![];
    collect_same_key_pairs(
      dst.iter().cloned(),
      new_widgets.iter().cloned(),
      self,
      &mut same_key_pairs,
    );

    if same_key_pairs.is_empty() {
      dst.iter().for_each(|o| o.remove_subtree(self));
      new_widgets
        .iter()
        .for_each(|n| n.on_mounted_subtree(self, true));
    } else {
      let mut swapped = HashMap::<_, _, RandomState>::default();
      for &(o, n) in same_key_pairs.iter() {
        n.swap(o, self);
        self.swap_node_data(n, o);
        swapped.insert(o, n);
        swapped.insert(n, o);
      }

      let (tree1, tree2) = unsafe { self.split_tree() };
      dst.iter().for_each(|o| {
        let old = swapped.get(o).unwrap_or(o);
        old.descendants(tree1).for_each(|o| {
          if !swapped.contains_key(&o) {
            o.on_disposed(tree2);
          }
        });
        old.0.remove_subtree(&mut tree1.arena);
      });

      new_widgets
        .iter_mut()
        .flat_map(|n| {
          if let Some(s) = swapped.get(n) {
            *n = *s;
          }
          n.descendants(tree1)
        })
        .for_each(|n| n.on_mounted(tree2, !swapped.contains_key(&n)));
    }

    if self.root.is_none() {
      assert_eq!(new_widgets.len(), 1, "must have one widget as root");
      self.set_root_id(new_widgets[0]);
    }
    new_widgets
  }

  pub(crate) unsafe fn split_tree(&mut self) -> (&mut WidgetTree, &mut WidgetTree) {
    let ptr = self as *mut WidgetTree;
    (&mut *ptr, &mut *ptr)
  }
}

impl WidgetId {
  /// Returns a reference to the node data.
  pub(crate) fn get(self, tree: &WidgetTree) -> Option<&dyn Render> {
    tree.arena.get(self.0).map(|node| node.get().as_ref())
  }

  /// Returns a mutable reference to the node data.
  pub(crate) fn get_mut(self, tree: &mut WidgetTree) -> Option<&mut Box<dyn Render>> {
    tree.arena.get_mut(self.0).map(|node| node.get_mut())
  }

  /// Compute layout of the render widget `id`, and store its result in the
  /// store.
  pub(crate) fn perform_layout(
    self,
    out_clamp: BoxClamp,
    tree: &mut WidgetTree,
    performed: &mut Vec<WidgetId>,
  ) -> Size {
    tree
      .layout_info(self)
      .and_then(|BoxLayout { clamp, rect }| {
        rect.and_then(|r| (&out_clamp == clamp).then(|| r.size))
      })
      .unwrap_or_else(|| {
        // Safety: `LayoutCtx` will never mutable access widget tree, so split a node is
        // safe.
        let (tree1, tree2) = unsafe { tree.split_tree() };
        let mut ctx = LayoutCtx { id: self, tree: tree1, performed };
        let layout = self.assert_get(tree2);
        let size = layout.perform_layout(out_clamp, &mut ctx);
        let size = out_clamp.clamp(size);
        let info = tree1.layout_info_or_default(self);
        info.clamp = out_clamp;
        info.rect.get_or_insert_with(Rect::zero).size = size;

        self.assert_get_mut(tree1).query_all_type_mut(
          |_: &mut PerformedLayoutListener| {
            performed.push(self);
            false
          },
          QueryOrder::OutsideFirst,
        );
        size
      })
  }

  /// detect if the widget of this id point to is dropped.
  pub(crate) fn is_dropped(self, tree: &WidgetTree) -> bool { self.0.is_removed(&tree.arena) }

  #[allow(clippy::needless_collect)]
  pub(crate) fn common_ancestor_of(self, other: WidgetId, tree: &WidgetTree) -> Option<WidgetId> {
    if self.is_dropped(tree) || other.is_dropped(tree) {
      return None;
    }

    let p0 = other.ancestors(tree).collect::<Vec<_>>();
    let p1 = self.ancestors(tree).collect::<Vec<_>>();

    p0.iter()
      .rev()
      .zip(p1.iter().rev())
      .filter(|(a, b)| a == b)
      .last()
      .map(|(p, _)| p.clone())
  }

  pub(crate) fn parent(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.parent())
  }

  pub(crate) fn first_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.first_child())
  }

  pub(crate) fn last_child(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.last_child())
  }

  pub(crate) fn next_sibling(self, tree: &WidgetTree) -> Option<WidgetId> {
    self.node_feature(tree, |node| node.next_sibling())
  }

  pub(crate) fn ancestors(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.ancestors(&tree.arena).map(WidgetId)
  }

  /// Detect if this widget is the ancestors of `w`
  pub(crate) fn ancestors_of(self, w: WidgetId, tree: &WidgetTree) -> bool {
    w.ancestors(tree).any(|a| a == self)
  }

  pub(crate) fn children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn reverse_children(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.reverse_children(&tree.arena).map(WidgetId)
  }

  pub(crate) fn descendants(self, tree: &WidgetTree) -> impl Iterator<Item = WidgetId> + '_ {
    self.0.descendants(&tree.arena).map(WidgetId)
  }

  pub(crate) fn swap(self, other: WidgetId, tree: &mut WidgetTree) {
    let first_child = self.first_child(tree);
    let mut cursor = first_child;
    while let Some(c) = cursor {
      cursor = c.next_sibling(tree);
      other.append(c, tree);
    }
    let mut other_child = other.first_child(tree);
    while other_child.is_some() && other_child != first_child {
      let o_c = other_child.unwrap();
      other_child = o_c.next_sibling(tree);
      self.append(o_c, tree);
    }

    let guard = tree.empty_node();
    self.insert_after(guard, tree);
    other.insert_after(self, tree);
    guard.insert_after(other, tree);
    guard.0.remove(&mut tree.arena);
  }

  pub(crate) fn remove_subtree(self, tree: &mut WidgetTree) {
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self
      .0
      .descendants(&tree1.arena)
      .map(WidgetId)
      .for_each(|id| id.on_disposed(tree2));
    self.0.remove_subtree(&mut tree1.arena);
    if tree1.root() == self {
      tree1.root.take();
    }
  }

  pub(crate) fn insert_after(self, next: WidgetId, tree: &mut WidgetTree) {
    self.0.insert_after(next.0, &mut tree.arena);
  }

  pub(crate) fn prepend(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.prepend(child.0, &mut tree.arena);
  }

  pub(crate) fn append(self, child: WidgetId, tree: &mut WidgetTree) {
    self.0.append(child.0, &mut tree.arena);
  }

  pub(crate) fn on_mounted_subtree(self, tree: &mut WidgetTree, brand_new: bool) {
    let (tree1, tree2) = unsafe { tree.split_tree() };

    self
      .descendants(tree1)
      .for_each(|w| w.on_mounted(tree2, brand_new));
  }

  pub(crate) fn on_mounted(self, tree: &mut WidgetTree, brand_new: bool) {
    self.assert_get(tree).query_all_type(
      |notifier: &StateChangeNotifier| {
        let state_changed = tree.state_changed.clone();
        notifier
          .change_stream()
          .filter(|b| b.contains(ChangeScope::FRAMEWORK))
          .subscribe(move |_| {
            state_changed.borrow_mut().insert(self);
          });
        true
      },
      QueryOrder::OutsideFirst,
    );

    if brand_new {
      // Safety: lifecycle context have no way to change tree struct.
      let (tree1, tree2) = unsafe { tree.split_tree() };
      self.assert_get_mut(tree1).query_all_type_mut(
        |m: &mut MountedListener| {
          (m.on_mounted)(LifeCycleCtx { id: self, tree: tree2 });
          true
        },
        QueryOrder::OutsideFirst,
      );
    }
  }

  pub(crate) fn on_disposed(self, tree: &mut WidgetTree) {
    tree.layout_store.remove(&self);
    // Safety: tmp code, remove after deprecated `query_all_type_mut`.
    let (tree1, tree2) = unsafe { tree.split_tree() };
    self.assert_get_mut(tree1).query_all_type_mut(
      |d: &mut DisposedListener| {
        (d.on_disposed)(LifeCycleCtx { id: self, tree: tree2 });
        true
      },
      QueryOrder::OutsideFirst,
    )
  }

  /// Return the single child of `widget`, panic if have more than once child.
  pub(crate) fn single_child(&self, tree: &WidgetTree) -> Option<WidgetId> {
    assert_eq!(self.first_child(tree), self.last_child(tree));
    self.first_child(tree)
  }

  fn node_feature<F: Fn(&Node<Box<dyn Render>>) -> Option<NodeId>>(
    self,
    tree: &WidgetTree,
    method: F,
  ) -> Option<WidgetId> {
    tree.arena.get(self.0).map(method).flatten().map(WidgetId)
  }

  pub(crate) fn assert_get(self, tree: &WidgetTree) -> &dyn Render {
    self.get(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn assert_get_mut(self, tree: &mut WidgetTree) -> &mut Box<dyn Render> {
    self.get_mut(tree).expect("Widget not exists in the `tree`")
  }

  pub(crate) fn key(self, tree: &WidgetTree) -> Option<Key> {
    let mut key = None;
    self
      .assert_get(tree)
      .query_on_first_type(QueryOrder::OutsideFirst, |k: &Key| key = Some(k.clone()));
    key
  }
}

impl Widget {
  pub(crate) fn into_subtree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
  ) -> Option<WidgetId> {
    let (id, children) = self.place_node_in_tree(parent, tree);
    if let Some(id) = id {
      let mut pairs = vec![];
      children.for_each(|w| pairs.push((id, w)));

      while let Some((parent, widget)) = pairs.pop() {
        let (child, children) = widget.place_node_in_tree(Some(parent), tree);
        if let Some(child) = child {
          parent.prepend(child, tree);
        }
        children.for_each(|w| pairs.push((child.unwrap_or(parent), w)));
      }
      Some(id)
    } else {
      match children {
        Children::None => None,
        _ => unreachable!(),
      }
    }
  }

  fn place_node_in_tree(
    self,
    parent: Option<WidgetId>,
    tree: &mut WidgetTree,
  ) -> (Option<WidgetId>, Children) {
    let Self { node, children } = self;

    if let Some(node) = node {
      match node {
        WidgetNode::Compose(c) => {
          assert!(children.is_none(), "compose widget shouldn't have child.");
          let mut build_ctx = BuildCtx::new(parent, tree);
          let c = c(&mut build_ctx);
          c.place_node_in_tree(parent, tree)
        }
        WidgetNode::Render(r) => (Some(tree.new_node(r)), children),
        WidgetNode::Dynamic(e) => {
          let road_sign = tree.empty_node();
          tree
            .generator_store
            .new_generator(e, parent, road_sign, !children.is_none());
          (Some(road_sign), children)
        }
      }
    } else {
      match children {
        Children::None => (None, Children::None),
        Children::Single(s) => s.place_node_in_tree(parent, tree),
        Children::Multi(_) => unreachable!("None parent with multi child is forbidden."),
      }
    }
  }
}
#[cfg(test)]
mod tests {
  extern crate test;
  use std::{cell::RefCell, rc::Rc};

  use test::Bencher;

  use super::*;
  use crate::{
    prelude::{widget_tree::WidgetTree, IntoWidget},
    test::{embed_post::EmbedPost, key_embed_post::EmbedPostWithKey, recursive_row::RecursiveRow},
  };

  fn test_sample_create(width: usize, depth: usize) -> WidgetTree {
    WidgetTree::new(RecursiveRow { width, depth }.into_widget(), <_>::default())
  }

  #[test]
  fn drop_info_clear() {
    let post = EmbedPost::new(3);
    let ctx = Rc::new(RefCell::new(AppContext::default()));
    let mut tree = WidgetTree::new(post.into_widget(), ctx);
    tree.tree_repair();
    assert_eq!(tree.count(), 17);

    tree.mark_dirty(tree.root());
    tree.root().remove_subtree(&mut tree);

    assert_eq!(tree.is_dirty(), false);
  }

  #[bench]
  fn inflate_5_x_1000(b: &mut Bencher) {
    b.iter(|| {
      let post = EmbedPost::new(1000);
      WidgetTree::new(post.into_widget(), <_>::default());
    });
  }

  #[bench]
  fn inflate_50_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(50, 2)) }

  #[bench]
  fn inflate_100_pow_2(b: &mut Bencher) { b.iter(|| test_sample_create(100, 2)) }

  #[bench]
  fn inflate_10_pow_4(b: &mut Bencher) { b.iter(|| test_sample_create(10, 4)) }

  #[bench]
  fn inflate_10_pow_5(b: &mut Bencher) { b.iter(|| test_sample_create(10, 5)) }

  #[bench]
  fn repair_5_x_1000(b: &mut Bencher) {
    let post = EmbedPostWithKey::new(1000);
    let mut tree = WidgetTree::new(post.into_widget(), <_>::default());
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair()
    });
  }

  #[bench]
  fn repair_50_pow_2(b: &mut Bencher) {
    let mut tree = test_sample_create(50, 2);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_100_pow_2(b: &mut Bencher) {
    let mut tree = test_sample_create(100, 2);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_4(b: &mut Bencher) {
    let mut tree = test_sample_create(10, 4);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }

  #[bench]
  fn repair_10_pow_5(b: &mut Bencher) {
    let mut tree = test_sample_create(10, 5);
    b.iter(|| {
      tree.mark_dirty(tree.root());
      tree.tree_repair();
    })
  }
}
