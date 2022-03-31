use algo::{CowRc, FrameCache};
use std::sync::{Arc, RwLock};
use unic_bidi::{BidiClass, BidiInfo, Level, ParagraphInfo};

pub struct ReorderResult {
  pub original_classes: Vec<BidiClass>,
  pub levels: Vec<Level>,
  pub paragraphs: Vec<ParagraphInfo>,
}
#[derive(Clone, Default)]
pub struct TextReorder {
  cache: Arc<RwLock<FrameCache<CowRc<str>, Arc<ReorderResult>>>>,
}

impl TextReorder {
  pub fn get_from_cache(&self, text: &CowRc<str>) -> Option<Arc<ReorderResult>> {
    self.cache.read().unwrap().get(text).cloned()
  }

  pub fn reorder_text(&self, text: &CowRc<str>) -> Arc<ReorderResult> {
    self.get_from_cache(text).unwrap_or_else(|| {
      let BidiInfo {
        original_classes, levels, paragraphs, ..
      } = BidiInfo::new(text, None);
      let result = Arc::new(ReorderResult { original_classes, levels, paragraphs });
      let mut cache = self.cache.write().unwrap();
      cache.insert(text.clone(), result.clone());
      result
    })
  }

  pub fn end_frame(&mut self) { self.cache.write().unwrap().end_frame("Text Reorder") }
}
