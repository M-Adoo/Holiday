use crate::{
  builtin_widgets::{delay_drop_widget::query_drop_until_widget, key::AnyKey},
  impl_proxy_query, impl_query_self_only,
  prelude::{
    child_convert::{FillVec, IntoChild},
    *,
  },
  widget::{
    widget_id::{dispose_nodes, empty_node, split_arena},
    *,
  },
};
use std::{
  cell::RefCell,
  collections::{HashMap, HashSet},
};

/// the information of a widget generated by `DynWidget`.
pub(crate) enum DynWidgetGenInfo {
  /// DynWidget generate single result, and have static children. The depth
  /// describe the distance from first dynamic widget (self) to the static
  /// child.
  DynDepth(usize),
  /// `DynWidget` without static children, and the whole subtree of generated
  /// widget are dynamic widgets. The value record how many dynamic siblings
  /// have.
  WholeSubtree(usize),
}

/// Widget that as a container of dynamic widgets

#[derive(Declare)]
pub struct DynWidget<D> {
  #[declare(convert=custom)]
  pub(crate) dyns: Option<D>,
}

impl<D> DynWidgetDeclarer<D> {
  pub fn dyns(mut self, d: D) -> Self {
    self.dyns = Some(Some(d));
    self
  }
}

impl<D> DynWidget<D> {
  pub fn set_declare_dyns(&mut self, dyns: D) { self.dyns = Some(dyns); }

  pub fn into_inner(mut self) -> D {
    self
      .dyns
      .take()
      .unwrap_or_else(|| unreachable!("stateless `DynWidget` must be initialized."))
  }
}

/// Widget help to limit which `DynWidget` can be a parent widget and which can
/// be a child.
pub(crate) struct DynRender<D> {
  dyn_widgets: Stateful<DynWidget<D>>,
  self_render: RefCell<Box<dyn Render>>,
  gen_info: RefCell<Option<DynWidgetGenInfo>>,
  dyns_to_widgets: fn(D) -> Vec<Widget>,
  drop_until_widgets: WidgetsHost,
}

// A dynamic widget must be stateful, depends others.
impl<D: 'static> Render for DynRender<D> {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if !self.regen_if_need(ctx) {
      self.destroy_unhosted(ctx.arena, ctx.store);
    }
    self.self_render.perform_layout(clamp, ctx)
  }

  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.drop_until_widgets.is_empty() {
      ctx.painter.save();
      // set the position back to parent.
      let rc = ctx.box_rect().unwrap();
      ctx.painter.translate(-rc.min_x(), -rc.min_y());
      self.drop_until_widgets.paint(ctx);
      ctx.painter.restore();
    }

    self.self_render.paint(ctx);
  }

  fn only_sized_by_parent(&self) -> bool {
    // Dyn widget effect the children of its parent. Even if its self render is only
    // sized by parent, but itself effect siblings, sibling effect parent, means
    // itself not only sized by parent but also its sibling.
    false
  }

  fn hit_test(&self, ctx: &HitTestCtx, pos: Point) -> HitTest {
    self.self_render.hit_test(ctx, pos)
  }

  fn can_overflow(&self) -> bool {
    !self.drop_until_widgets.is_empty() || self.self_render.can_overflow()
  }

  fn get_transform(&self) -> Option<Transform> { self.self_render.get_transform() }
}

#[derive(Default)]
struct WidgetsHost {
  wids: RefCell<HashSet<WidgetId>>,
}

impl WidgetsHost {
  fn add(&self, wid: WidgetId) { self.wids.borrow_mut().insert(wid); }

  fn is_empty(&self) -> bool { self.wids.borrow().is_empty() }

  fn paint(&self, ctx: &mut PaintingCtx) {
    self.wids.borrow().iter().for_each(|wid| {
      wid.paint_subtree(ctx.arena, ctx.store, ctx.wnd_ctx, ctx.painter);
    });
  }

  fn remove_unhost(&self, arena: &mut TreeArena, store: &mut LayoutStore) {
    let mut set = HashSet::new();
    let arr = self
      .wids
      .borrow()
      .iter()
      .filter(|id| {
        query_drop_until_widget(**id, arena).map_or(true, |w| w.state_ref().delay_drop_until)
      })
      .cloned()
      .collect::<Vec<_>>();
    arr.iter().for_each(|id| {
      self.wids.borrow_mut().remove(id);
    });
    self.collect_subtree(arr.into_iter(), arena, &mut set);
    set.iter().for_each(|id| id.remove_subtree(arena, store));
  }

  fn collect_subtree(
    &self,
    it: impl Iterator<Item = WidgetId>,
    arena: &TreeArena,
    set: &mut HashSet<WidgetId>,
  ) {
    it.for_each(|id| {
      id.descendants(arena).for_each(|wid| {
        wid
          .assert_get(arena)
          .query_on_first_type(QueryOrder::OutsideFirst, |w: &WidgetsHost| {
            w.collect_subtree(w.wids.borrow().iter().cloned(), arena, set)
          })
      });
      set.insert(id);
    });
  }
}

impl Query for WidgetsHost {
  impl_query_self_only!();
}

impl<D> DynRender<D> {
  pub(crate) fn new<M: ImplMarker>(dyns: Stateful<DynWidget<D>>) -> Self
  where
    D: FillVec<M, Widget>,
  {
    fn into_vec<M, D: FillVec<M, Widget>>(this: D) -> Vec<Widget> {
      let mut vec = vec![];
      this.fill_vec(&mut vec);
      vec
    }

    Self {
      dyn_widgets: dyns,
      self_render: RefCell::new(Box::new(Void)),
      gen_info: <_>::default(),
      dyns_to_widgets: into_vec::<_, D>,
      drop_until_widgets: <_>::default(),
    }
  }

  fn regen_if_need(&self, ctx: &mut LayoutCtx) -> bool {
    let Some(new_widgets) = self.dyn_widgets.silent_ref().dyns.take() else {
      return false;
    };

    let mut gen_info = self.gen_info.borrow_mut();
    let mut gen_info = gen_info.get_or_insert_with(|| {
      if ctx.has_child() {
        DynWidgetGenInfo::DynDepth(1)
      } else {
        DynWidgetGenInfo::WholeSubtree(1)
      }
    });

    let LayoutCtx {
      id: sign,
      arena,
      store,
      wnd_ctx,
      dirty_set,
    } = ctx;

    let mut new_widgets = (self.dyns_to_widgets)(new_widgets)
      .into_iter()
      .filter_map(|w| w.into_subtree(None, arena, wnd_ctx))
      .collect::<Vec<_>>();
    if new_widgets.is_empty() {
      new_widgets.push(empty_node(arena));
    }

    self.update_key_state(*sign, gen_info, &new_widgets, arena);

    // Place the real old render in node, the dyn render in node keep.
    std::mem::swap(
      &mut *self.self_render.borrow_mut(),
      sign.assert_get_mut(arena),
    );

    // swap the new sign and old, so we can always keep the sign id not change.
    fn refresh_sign_id(
      sign: WidgetId,
      ids: &mut [WidgetId],
      arena: &mut TreeArena,
      store: &mut LayoutStore,
    ) -> WidgetId {
      sign.swap_id(ids[0], arena, store);
      let old_sign = ids[0];
      ids[0] = sign;
      old_sign
    }

    match &mut gen_info {
      DynWidgetGenInfo::DynDepth(depth) => {
        assert_eq!(new_widgets.len(), 1);

        dispose_nodes(
          iter_single_child(*sign, arena, *depth),
          arena,
          store,
          wnd_ctx,
        );
        let old_sign = refresh_sign_id(*sign, &mut new_widgets, arena, store);
        let declare_child_parent = single_down(old_sign, arena, *depth as isize - 1);
        let (new_leaf, down_level) = down_to_leaf(*sign, arena);

        let new_depth = down_level + 1;
        if let Some(declare_child_parent) = declare_child_parent {
          // Safety: control two subtree not intersect.
          let (arena1, arena2) = unsafe { split_arena(arena) };
          declare_child_parent
            .children(arena1)
            .for_each(|c| new_leaf.append(c, arena2));
        }

        old_sign.insert_after(*sign, arena);
        self.remove_old_subtree(*sign, old_sign, arena, store, dirty_set);

        let mut w = *sign;
        loop {
          w.on_mounted(arena, store, wnd_ctx, dirty_set);
          if w == new_leaf {
            break;
          }
          w = w.single_child(arena).unwrap();
        }

        *depth = new_depth;
      }

      DynWidgetGenInfo::WholeSubtree(siblings) => {
        let mut cursor = Some(*sign);
        (0..*siblings).for_each(|_| {
          let o = cursor.unwrap();
          cursor = o.next_sibling(arena);
          dispose_nodes(o.descendants(arena), arena, store, wnd_ctx);
          self.remove_old_subtree(*sign, o, arena, store, dirty_set);
        });

        let head = refresh_sign_id(*sign, &mut new_widgets, arena, store);
        new_widgets.iter().fold(head, |prev, wid| {
          prev.insert_after(*wid, arena);
          wid.on_mounted_subtree(arena, store, wnd_ctx, dirty_set);
          *wid
        });

        head.remove_subtree(arena, store);
        *siblings = new_widgets.len()
      }
    };

    // Place the dynRender back in node.
    std::mem::swap(
      &mut *self.self_render.borrow_mut(),
      sign.assert_get_mut(arena),
    );
    true
  }

  fn remove_old_subtree(
    &self,
    sign: WidgetId,
    mut wid: WidgetId,
    arena: &mut TreeArena,
    store: &mut LayoutStore,
    dirty_set: &DirtySet,
  ) {
    fn detach(
      sign: WidgetId,
      wid: WidgetId,
      drop_until: Stateful<DelayDropWidget>,
      arena: &mut TreeArena,
      dirty_set: &DirtySet,
    ) {
      wid.detach(arena);
      wid.assert_get(arena).query_all_type(
        |notifier: &StateChangeNotifier| {
          let state_changed = dirty_set.clone();
          // abandon the old subscribe
          notifier.reset();
          notifier
            .raw_modifies()
            .filter(|b| b.contains(ModifyScope::FRAMEWORK))
            .subscribe(move |_| {
              state_changed.borrow_mut().insert(wid);
            });
          true
        },
        QueryOrder::OutsideFirst,
      );

      let tmp = drop_until.clone();
      let state_changed = dirty_set.clone();
      drop_until
        .raw_modifies()
        .filter(|b| b.contains(ModifyScope::FRAMEWORK))
        .subscribe(move |_| {
          // notify dyn widget relayout to remove self.
          if tmp.state_ref().delay_drop_until {
            state_changed.borrow_mut().insert(sign);
          }
        });
    }

    if wid == sign {
      let new_sign = WidgetId::new_node(arena);
      wid.insert_before(new_sign, arena);
      wid.swap_id(new_sign, arena, store);
      wid = new_sign;
    }

    let drop_until = query_drop_until_widget(wid, arena);
    let is_drop = drop_until
      .as_ref()
      .map_or(true, |w| w.state_ref().delay_drop_until);
    if is_drop {
      wid.remove_subtree(arena, store);
    } else {
      detach(sign, wid, drop_until.unwrap(), arena, dirty_set);
      self.drop_until_widgets.add(wid);
      dirty_set.borrow_mut().insert(wid);
    }
  }

  fn destroy_unhosted(&self, arena: &mut TreeArena, store: &mut LayoutStore) {
    self.drop_until_widgets.remove_unhost(arena, store)
  }

  fn update_key_state(
    &self,
    sign_id: WidgetId,
    gen_info: &DynWidgetGenInfo,
    new_widgets: &[WidgetId],
    arena: &TreeArena,
  ) {
    let mut old_key_list = HashMap::new();
    let siblings = match gen_info {
      DynWidgetGenInfo::DynDepth(_) => 1,
      DynWidgetGenInfo::WholeSubtree(width) => *width,
    };
    let mut remove = Some(sign_id);
    (0..siblings).for_each(|_| {
      let o = remove.unwrap();
      inspect_key(&o, arena, |old_key_widget: &dyn AnyKey| {
        let key = old_key_widget.key();
        old_key_list.insert(key, o);
      });

      remove = o.next_sibling(arena);
    });

    new_widgets.iter().for_each(|n| {
      inspect_key(n, arena, |new_key_widget: &dyn AnyKey| {
        let key = &new_key_widget.key();
        if let Some(wid) = old_key_list.get(key) {
          inspect_key(wid, arena, |old_key_widget: &dyn AnyKey| {
            new_key_widget.record_before_value(old_key_widget);
          });
          old_key_list.remove(key);
        } else {
          new_key_widget.mounted();
        }
      });
    });

    old_key_list
      .values()
      .for_each(|wid| inspect_key(wid, arena, |old_key_widget| old_key_widget.disposed()));
  }
}

impl<D: 'static> Query for DynRender<D> {
  impl_proxy_query!(self.self_render, self.dyn_widgets, self.drop_until_widgets);
}

impl<D: 'static> Query for DynWidget<D> {
  impl_query_self_only!();
}

fn inspect_key(id: &WidgetId, tree: &TreeArena, mut cb: impl FnMut(&dyn AnyKey)) {
  #[allow(clippy::borrowed_box)]
  id.assert_get(tree).query_on_first_type(
    QueryOrder::OutsideFirst,
    |key_widget: &Box<dyn AnyKey>| {
      cb(&**key_widget);
    },
  );
}

fn single_down(id: WidgetId, arena: &TreeArena, mut down_level: isize) -> Option<WidgetId> {
  let mut res = Some(id);
  while down_level > 0 {
    down_level -= 1;
    res = res.unwrap().single_child(arena);
  }
  res
}

fn iter_single_child(
  id: WidgetId,
  arena: &TreeArena,
  depth: usize,
) -> impl Iterator<Item = WidgetId> + '_ {
  (0..depth).scan(id, |id, idx| {
    if idx != 0 {
      *id = id.single_child(arena).unwrap();
    }
    Some(*id)
  })
}

fn down_to_leaf(id: WidgetId, arena: &TreeArena) -> (WidgetId, usize) {
  let mut leaf = id;
  let mut depth = 0;
  while let Some(c) = leaf.single_child(arena) {
    leaf = c;
    depth += 1;
  }
  (leaf, depth)
}

// impl IntoWidget

// only `DynWidget` gen single widget can as a parent widget
impl<M, D> IntoWidget<NotSelf<M>> for Stateful<DynWidget<D>>
where
  M: ImplMarker,
  D: IntoChild<M, Widget> + 'static,
{
  #[inline]
  fn into_widget(self) -> Widget { DynRender::new(self).into_widget() }
}

/// Not implement as `IntoWidget` trait, because need to avoid conflict
/// implement for `IntoIterator`. `Stateful<DynWidget<Option<W: IntoWidget>>>`
/// both satisfied `IntoWidget` as a single child and `Stateful<DynWidget<impl
/// IntoIterator<Item= impl IntoWidget>>>` as multi child.
impl<D> Stateful<DynWidget<Option<D>>> {
  #[inline]
  pub fn into_widget<M>(self) -> Widget
  where
    M: ImplMarker,
    D: IntoChild<M, Widget> + 'static,
  {
    DynRender::new(self).into_widget()
  }
}

impl<W: SingleChild> SingleChild for DynWidget<W> {}

#[cfg(test)]
mod tests {
  use std::{cell::RefCell, rc::Rc};

  use crate::{
    builtin_widgets::key::KeyChange, impl_query_self_only, prelude::*, test::*,
    widget_tree::WidgetTree,
  };

  #[test]
  fn expr_widget_as_root() {
    let size = Stateful::new(Size::zero());
    let w = widget! {
      states { size: size.clone() }
      DynWidget {
        dyns: MockBox { size: *size },
        Void {}
      }
    };
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(
      w.into_widget(),
      WindowCtx::new(AppContext::default(), scheduler),
    );
    tree.layout(Size::zero());
    let ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    {
      *size.state_ref() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 2);

    assert_eq!(ids[1], new_ids[1]);
  }

  #[test]
  fn expr_widget_with_declare_child() {
    let size = Stateful::new(Size::zero());
    let w = widget! {
      states { size: size.clone() }
      MockBox {
        size: Size::zero(),
        DynWidget {
          dyns: MockBox { size: *size },
          Void {}
        }
      }
    };
    let app_ctx = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w.into_widget(), WindowCtx::new(app_ctx, scheduler));
    tree.layout(Size::zero());
    let ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    {
      *size.state_ref() = Size::new(1., 1.);
    }
    tree.layout(Size::zero());
    let new_ids = tree.root().descendants(&tree.arena).collect::<Vec<_>>();
    assert_eq!(new_ids.len(), 3);

    assert_eq!(ids[0], new_ids[0]);
    assert_eq!(ids[2], new_ids[2]);
  }

  #[test]
  fn expr_widget_mounted_new() {
    let v = Stateful::new(vec![1, 2, 3]);

    let new_cnt = Stateful::new(0);
    let drop_cnt = Stateful::new(0);
    let w = widget! {
      states {
        v: v.clone(),
        new_cnt: new_cnt.clone(),
        drop_cnt: drop_cnt.clone(),
      }

      MockMulti { DynWidget {
        dyns: {
          v.clone().into_iter().map(move |_| {
            widget! {
              MockBox{
                size: Size::zero(),
                on_mounted: move |_| *new_cnt += 1,
                on_disposed: move |_| *drop_cnt += 1
              }
            }
          })
        }
      }}
    };
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(
      w.into_widget(),
      WindowCtx::new(AppContext::default(), scheduler),
    );
    tree.layout(Size::zero());
    assert_eq!(*new_cnt.state_ref(), 3);
    assert_eq!(*drop_cnt.state_ref(), 0);

    v.state_ref().push(4);
    tree.layout(Size::zero());
    assert_eq!(*new_cnt.state_ref(), 7);
    assert_eq!(*drop_cnt.state_ref(), 3);

    v.state_ref().pop();
    tree.layout(Size::zero());
    assert_eq!(*new_cnt.state_ref(), 10);
    assert_eq!(*drop_cnt.state_ref(), 7);
  }

  #[test]
  fn dyn_widgets_with_key() {
    let v = Stateful::new(vec![(1, '1'), (2, '2'), (3, '3')]);
    let enter_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let update_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let leave_list: Stateful<Vec<char>> = Stateful::new(vec![]);
    let key_change: Stateful<KeyChange<char>> = Stateful::new(KeyChange::default());
    let w = widget! {
      states {
        v: v.clone(),
        enter_list: enter_list.clone(),
        update_list: update_list.clone(),
        leave_list: leave_list.clone(),
        key_change: key_change.clone(),
      }

      MockMulti {
        DynWidget {
          dyns: {
            v.clone().into_iter().map(move |(i, c)| {
              widget! {
                KeyWidget {
                  id: key,
                  key: Key::from(i),
                  value: Some(c),

                  MockBox {
                    size: Size::zero(),
                    on_mounted: move |_| {
                      if key.is_enter() {
                        (*enter_list).push(key.value.unwrap());
                      }

                      if key.is_changed() {
                        (*update_list).push(key.value.unwrap());
                        *key_change = key.get_change();
                      }
                    },
                    on_disposed: move |_| {
                      if key.is_disposed() {
                        (*leave_list).push(key.value.unwrap());
                      }
                    }
                  }
                }
              }
            })
          }
        }
      }
    };

    // 1. 3 item enter
    let app_ctx = <_>::default();
    let scheduler = FuturesLocalSchedulerPool::default().spawner();
    let mut tree = WidgetTree::new(w.into_widget(), WindowCtx::new(app_ctx, scheduler));
    tree.layout(Size::zero());
    let expect_vec = vec!['1', '2', '3'];
    assert_eq!((*enter_list.state_ref()).len(), 3);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    (*enter_list.state_ref()).clear();

    // 2. add 1 item
    v.state_ref().push((4, '4'));
    tree.layout(Size::zero());
    let expect_vec = vec!['4'];
    assert_eq!((*enter_list.state_ref()).len(), 1);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    // clear enter list vec
    (*enter_list.state_ref()).clear();

    // 3. update the second item
    v.state_ref()[1].1 = 'b';
    tree.layout(Size::zero());

    let expect_vec = vec![];
    assert_eq!((*enter_list.state_ref()).len(), 0);
    assert!(
      (*enter_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );

    let expect_vec = vec!['b'];
    assert_eq!((*update_list.state_ref()).len(), 1);
    assert!(
      (*update_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.state_ref(), KeyChange(Some('2'), Some('b')));
    (*update_list.state_ref()).clear();

    // 4. remove the second item
    v.state_ref().remove(1);
    tree.layout(Size::zero());
    let expect_vec = vec!['b'];
    assert_eq!((*leave_list.state_ref()), expect_vec);
    assert_eq!((*leave_list.state_ref()).len(), 1);
    assert!(
      (*leave_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    (*leave_list.state_ref()).clear();

    // 5. update the first item
    v.state_ref()[0].1 = 'a';
    tree.layout(Size::zero());

    assert_eq!((*enter_list.state_ref()).len(), 0);

    let expect_vec = vec!['a'];
    assert_eq!((*update_list.state_ref()).len(), 1);
    assert!(
      (*update_list.state_ref())
        .iter()
        .all(|item| expect_vec.contains(item))
    );
    assert_eq!(*key_change.state_ref(), KeyChange(Some('1'), Some('a')));
    (*update_list.state_ref()).clear();
  }

  #[test]
  fn delay_drop_widgets() {
    #[derive(Default, Clone)]
    struct Task {
      mounted: u32,
      pin: bool,
      paint_cnt: Rc<RefCell<u32>>,
      layout_cnt: Rc<RefCell<u32>>,
      trigger: u32,
      wid: Option<WidgetId>,
    }

    fn build(item: Stateful<Task>) -> Widget {
      widget! {
        states { task: item.clone() }
        TaskWidget {
          delay_drop_until: !task.pin,
          layout_cnt: task.layout_cnt.clone(),
          paint_cnt: task.paint_cnt.clone(),
          trigger: task.trigger,
          on_mounted: move |ctx| {
            task.mounted += 1;
            task.wid = Some(ctx.id);
          },
          on_disposed: move |ctx| {
            let wid = task.wid.take();
            assert_eq!(wid, Some(ctx.id));
          }
        }
      }
      .into_widget()
    }

    #[derive(Declare)]
    struct TaskWidget {
      trigger: u32,
      paint_cnt: Rc<RefCell<u32>>,
      layout_cnt: Rc<RefCell<u32>>,
    }

    impl Render for TaskWidget {
      fn perform_layout(&self, _: BoxClamp, _: &mut LayoutCtx) -> Size {
        *self.layout_cnt.borrow_mut() += 1;
        Size::new(1., 1.)
      }

      fn paint(&self, _: &mut PaintingCtx) { *self.paint_cnt.borrow_mut() += 1; }
    }

    impl Query for TaskWidget {
      impl_query_self_only!();
    }

    fn child_count(wnd: &Window) -> usize {
      let tree = &wnd.widget_tree;
      let root = tree.root();
      root.children(&tree.arena).count()
    }

    let tasks = (0..3)
      .map(|_| Stateful::new(Task::default()))
      .collect::<Vec<_>>();
    let tasks = Stateful::new(tasks);
    let w = widget! {
      states {tasks: tasks.clone()}
      MockMulti {
        DynWidget {
          dyns: tasks.clone().into_iter().map(build)
        }
      }
    };

    let mut wnd = Window::default_mock(w, None);
    let mut removed = vec![];

    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 3);

    // the first pined widget will still paint it
    tasks.state_ref()[0].state_ref().pin = true;
    removed.push(tasks.state_ref().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 2);
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 2);

    // the remove pined widget will paint and no layout when no changed
    let first_layout_cnt = *removed[0].state_ref().layout_cnt.borrow();
    tasks.state_ref().get(0).unwrap().state_ref().pin = true;
    removed.push(tasks.state_ref().remove(0));
    wnd.draw_frame();
    assert_eq!(child_count(&wnd), 1);
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 3);
    assert_eq!(*removed[1].state_ref().paint_cnt.borrow(), 3);
    assert_eq!(
      *removed[0].state_ref().layout_cnt.borrow(),
      first_layout_cnt
    );

    // the remove pined widget only mark self dirty
    let first_layout_cnt = *removed[0].state_ref().layout_cnt.borrow();
    let secord_layout_cnt = *removed[1].state_ref().layout_cnt.borrow();
    let host_layout_cnt = *tasks.state_ref()[0].state_ref().layout_cnt.borrow();
    removed[0].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(
      *removed[0].state_ref().layout_cnt.borrow(),
      first_layout_cnt + 1
    );
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 4);
    assert_eq!(
      *removed[1].state_ref().layout_cnt.borrow(),
      secord_layout_cnt
    );
    assert_eq!(
      *tasks.state_ref()[0].state_ref().layout_cnt.borrow(),
      host_layout_cnt
    );

    // when unpined, it will no paint anymore
    removed[0].state_ref().pin = false;
    wnd.draw_frame();
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 4);
    assert_eq!(*removed[1].state_ref().paint_cnt.borrow(), 5);

    // after removed, it will no paint and layout anymore
    let first_layout_cnt = *removed[0].state_ref().layout_cnt.borrow();
    removed[0].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 4);
    assert_eq!(*removed[1].state_ref().paint_cnt.borrow(), 5);
    assert_eq!(
      *removed[0].state_ref().layout_cnt.borrow(),
      first_layout_cnt
    );

    // other pined widget is work fine.
    let first_layout_cnt = *removed[0].state_ref().layout_cnt.borrow();
    let second_layout_cnt = *removed[1].state_ref().layout_cnt.borrow();
    removed[1].state_ref().trigger += 1;
    wnd.draw_frame();
    assert_eq!(*removed[0].state_ref().paint_cnt.borrow(), 4);
    assert_eq!(*removed[1].state_ref().paint_cnt.borrow(), 6);
    assert_eq!(
      *removed[0].state_ref().layout_cnt.borrow(),
      first_layout_cnt
    );
    assert_eq!(
      *removed[1].state_ref().layout_cnt.borrow(),
      second_layout_cnt + 1,
    );
  }

  #[test]
  fn remove_delay_drop_widgets() {
    let child = Stateful::new(Some(()));
    let child_destroy_until = Stateful::new(false);
    let grandson = Stateful::new(Some(()));
    let grandson_destroy_until = Stateful::new(false);
    let w = widget! {
    states {
      child: child.clone(),
      child_destroy_until: child_destroy_until.clone(),
      grandson: grandson.clone(),
      grandson_destroy_until: grandson_destroy_until.clone(),
    }
    MockMulti {
      Option::map(child.as_ref(), move|_| widget! {
        MockMulti {
          delay_drop_until: *child_destroy_until,
          Option::map(grandson.as_ref(), move|_| widget! {
            MockBox {
              delay_drop_until: *grandson_destroy_until,
              size: Size::zero(),
            }
          })
        }
      })
      }
    };
    let mut wnd = Window::default_mock(w, None);
    wnd.draw_frame();

    let grandson_id = {
      let tree = &wnd.widget_tree;
      let root = tree.root();
      root
        .first_child(&tree.arena)
        .unwrap()
        .first_child(&tree.arena)
        .unwrap()
    };

    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(&wnd.widget_tree.arena));

    child.state_ref().take();
    wnd.draw_frame();
    assert!(!grandson_id.is_dropped(&wnd.widget_tree.arena));

    *child_destroy_until.state_ref() = true;
    wnd.draw_frame();
    assert!(grandson_id.is_dropped(&wnd.widget_tree.arena));
  }
}
