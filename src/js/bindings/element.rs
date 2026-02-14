use rquickjs::{Ctx, Class, Result, Value};
use kuchiki::NodeRef;
use kuchiki::traits::*;
use crate::js::bindings::token_list::DomTokenList;

#[derive(Clone, rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Element {
    #[qjs(skip_trace)]
    pub node: NodeRef,
}

use crate::js::bindings::style_declaration::CssStyleDeclaration;
use crate::js::bindings::event::EventTargetImpl;
use rquickjs::Function;

#[rquickjs::methods]
impl Element {
    #[qjs(rename = "addEventListener")]
    pub fn add_event_listener<'js>(&self, type_: String, listener: Function<'js>) {
        let ptr = &*self.node as *const _ as usize;
        unsafe {
            let listener_static: Function<'static> = std::mem::transmute(listener);
            EventTargetImpl::add_listener(ptr, type_, listener_static);
        }
    }

    #[qjs(rename = "removeEventListener")]
    pub fn remove_event_listener<'js>(&self, type_: String, _listener: Function<'js>) {
        // TODO: Implement specific listener removal (needs equality check or ID)
        // For MVP, removing all for type or ignoring?
        // Right now implementation removes all of type in EventTargetImpl::remove_listener??
        // Wait, EventTargetImpl::remove_listener removes the ENTRY for the type.
        // That effectively removes ALL listeners of that type.
        // It's technically incorrect (should remove only specific one) but okay for MVP Step 1.
        let ptr = &*self.node as *const _ as usize;
        EventTargetImpl::remove_listener(ptr, type_);
    }

    #[qjs(rename = "dispatchEvent")]
    pub fn dispatch_event<'js>(&self, ctx: Ctx<'js>, event: Value<'js>) -> bool {
        let ptr = &*self.node as *const _ as usize;
        
        // Helper to find parent
        let get_parent = |p: usize| -> Option<usize> {
            // Need to convert ptr back to NodeRef safely? 
            // This is unsafe. But we are in the same thread and DOM is alive.
            // Using unsafe transmutation of pointer back to reference is dangerous if we don't own it.
            // However, we know 'p' is a pointer to a NodeData inside a NodeRef/Rc.
            // Actually NodeRef is a pointer to Node.
            unsafe {
                let node_ref = &*(p as *const kuchiki::Node);
                node_ref.parent().map(|p| &*p as *const _ as usize)
            }
        };

        if let Some(event_obj_js) = event.as_object() {
             if let Ok(type_val) = event_obj_js.get::<_, String>("type") {
                 let bubbles = event_obj_js.get::<_, bool>("bubbles").unwrap_or(false);
                 
                 // Create Rust Event struct representation for logic
                 // We don't need full struct, just data
                 let event_data = crate::js::bindings::event::Event {
                     type_: type_val.clone(),
                     bubbles,
                     cancelable: false, // read from obj?
                     target: None,
                     current_target: None,
                 };
                 
                 // 1. Set 'target' property on the event object
                 let _ = event_obj_js.set("target", self.clone());

                 // 2. Get propagation path and listeners
                 let listeners_chain = EventTargetImpl::dispatch_event_with_bubbling(ptr, &event_data, get_parent);
                 
                // 3. Execute
                 let mut last_ptr = 0;
                 // First element is always target, set last_ptr to first one to avoid immediate break
                 if let Some((first, _)) = listeners_chain.first() {
                     last_ptr = *first;
                 }
                 
                 for (curr_ptr, listener_static) in listeners_chain {
                     // Check propagation status
                     let event_obj = event_obj_js.clone();
                     
                     let propagation_stopped = event_obj.get::<_, bool>("_propagationStopped").unwrap_or(false);
                     let immediate_stopped = event_obj.get::<_, bool>("_immediatePropagationStopped").unwrap_or(false);
                     
                     // If moving to a bubbling parent
                     if curr_ptr != last_ptr {
                         if propagation_stopped {
                             break; // Stop bubbling
                         }
                         last_ptr = curr_ptr;
                     }
                     
                     // If immediate stop requested
                     if immediate_stopped {
                         continue; // Skip rest of listeners for this element (and propagation is also stopped implied)
                     }
                     
                     let listener: Function<'js> = unsafe { std::mem::transmute(listener_static) };
                     let _: Result<Value> = listener.call((event.clone(),));
                 }
             }
        }
        true 
    }

    // Getter for style
    #[qjs(get, rename = "style")]
    pub fn style<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let decl = CssStyleDeclaration { node: self.node.clone() };
        let instance = Class::instance(ctx, decl)?;
        Ok(instance.into_value())
    }

    // Getter for classList
    #[qjs(get, rename = "classList")]
    pub fn class_list<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let list = DomTokenList { node: self.node.clone() };
        let instance = Class::instance(ctx, list)?;
        Ok(instance.into_value())
    }

    // Getter for tagName
    #[qjs(get, rename = "tagName")]
    pub fn tag_name(&self) -> String {
        self.node.as_element()
            .map(|data| data.name.local.to_string().to_uppercase())
            .unwrap_or_else(|| "".to_string())
    }

    // Getter for textContent
    #[qjs(get, rename = "textContent")]
    pub fn text_content(&self) -> String {
        self.node.text_contents()
    }
    
    // Setter for textContent
    #[qjs(set, rename = "textContent")]
    pub fn set_text_content(&self, text: String) {
        // Clear all children and append new text node
        // Requires mutating the tree. kuchiki NodeRef uses RefCell internally.
        // We can detach all children
        self.node.children().for_each(|child| child.detach());
        self.node.append(NodeRef::new_text(text));
    }

    #[qjs(rename = "getAttribute")]
    pub fn get_attribute(&self, name: String) -> Option<String> {
        self.node.as_element().and_then(|data| {
            data.attributes.borrow().get(name.as_str()).map(|s| s.to_string())
        })
    }
    
    #[qjs(rename = "setAttribute")]
    pub fn set_attribute(&self, name: String, value: String) {
        if let Some(data) = self.node.as_element() {
            data.attributes.borrow_mut().insert(name, value);
        }
    }
    
    #[qjs(get, rename = "innerHTML")]
    pub fn inner_html(&self) -> String {
        self.node.children().map(|c| c.to_string()).collect::<String>()
    }
    
    #[qjs(set, rename = "innerHTML")]
    pub fn set_inner_html(&self, html: String) {
        // Clear children
        self.node.children().for_each(|child| child.detach());
        
        // Parse HTML string
        // kuchiki::parse_html().one(html) creates a full document
        let document = kuchiki::parse_html().one(html);
        
        // Extract content from body and append to this node
        // Warning: This is a simplification. Context-aware parsing is harder.
        if let Ok(body) = document.select_first("body") {
            // We need to collect children first to avoid iterator invalidation during move
            let children: Vec<_> = body.as_node().children().collect();
            for child in children {
                self.node.append(child);
            }
        }
    }

    #[qjs(rename = "appendChild")]
    pub fn append_child<'js>(&self, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        let child_borrow = child.borrow();
        self.node.append(child_borrow.node.clone());
        Ok(child.clone())
    }

    #[qjs(rename = "removeChild")]
    pub fn remove_child<'js>(&self, _ctx: Ctx<'js>, child: Class<'js, Element>) -> Result<Class<'js, Element>> {
        let child_borrow = child.borrow();
        
        // Check if child is actually a child of this node
        if let Some(parent) = child_borrow.node.parent() {
            if parent == self.node {
                child_borrow.node.detach();
                return Ok(child.clone());
            }
        }
        
        Err(rquickjs::Error::new_from_js("NotFoundError", "The node to be removed is not a child of this node"))
    }

    #[qjs(rename = "querySelector")]
    pub fn query_selector<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        if let Ok(mut match_iter) = self.node.select(&selector) {
             if let Some(node_match) = match_iter.next() {
                 let element = Element { node: node_match.as_node().clone() };
                 return Class::instance(ctx, element).map(|i| i.into_value());
             }
        }
        Ok(Value::new_null(ctx))
    }

    #[qjs(rename = "querySelectorAll")]
    pub fn query_selector_all<'js>(&self, ctx: Ctx<'js>, selector: String) -> Result<Value<'js>> {
        // Return Array of Elements
        let array = rquickjs::Array::new(ctx.clone())?;
        if let Ok(match_iter) = self.node.select(&selector) {
            for (i, node_match) in match_iter.enumerate() {
                let element = Element { node: node_match.as_node().clone() };
                let instance = Class::instance(ctx.clone(), element)?;
                array.set(i, instance)?;
            }
        }
        Ok(array.into_value())
    }
}
