use rquickjs::{Ctx, Class, Result, Value, Object};
use std::sync::{Arc, Mutex};
use super::range::Range;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Selection {
    pub is_collapsed: bool,
    #[qjs(skip_trace)]
    pub ranges: Arc<Mutex<Vec<Range>>>,
}

#[rquickjs::methods]
impl Selection {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            is_collapsed: true,
            ranges: Arc::new(Mutex::new(Vec::new())),
        }
    }

    #[qjs(rename = "getRangeAt")]
    pub fn get_range_at<'js>(&self, ctx: Ctx<'js>, index: usize) -> Result<Value<'js>> {
        let lock = self.ranges.lock().unwrap();
        if index < lock.len() {
            let instance = Class::instance(ctx, lock[index].clone())?;
            Ok(instance.into_value())
        } else {
            Err(rquickjs::Error::Exception)
        }
    }

    #[qjs(rename = "addRange")]
    pub fn add_range(&mut self, range: Value<'_>) {
        if let Some(obj) = range.as_object() {
            if let Some(r_class) = Class::<Range>::from_object(&obj) {
                let range_data = r_class.borrow().clone();
                let mut lock = self.ranges.lock().unwrap();
                lock.push(range_data);
                self.is_collapsed = false; // By definition adding arbitrary range may uncollapse
            }
        }
    }

    #[qjs(rename = "removeAllRanges")]
    pub fn remove_all_ranges(&mut self) {
        self.ranges.lock().unwrap().clear();
        self.is_collapsed = true;
    }

    #[qjs(get, rename = "anchorNode")]
    pub fn anchor_node<'js>(&self, ctx: Ctx<'js>) -> Value<'js> {
        // In a real scenario, this would return an Element/Node wrapper. 
        // For MVP, returning null if no wrapper is tracked correctly back to JS space yet.
        Value::new_null(ctx)
    }

    #[qjs(get, rename = "anchorOffset")]
    pub fn anchor_offset(&self) -> usize {
        let lock = self.ranges.lock().unwrap();
        if let Some(r) = lock.first() {
            r.start_offset
        } else {
            0
        }
    }

    #[qjs(get, rename = "focusNode")]
    pub fn focus_node<'js>(&self, ctx: Ctx<'js>) -> Value<'js> {
        Value::new_null(ctx)
    }

    #[qjs(get, rename = "focusOffset")]
    pub fn focus_offset(&self) -> usize {
        let lock = self.ranges.lock().unwrap();
        if let Some(r) = lock.first() {
            r.end_offset
        } else {
            0
        }
    }
    
    #[qjs(get, rename = "rangeCount")]
    pub fn range_count(&self) -> usize {
        self.ranges.lock().unwrap().len()
    }
}
