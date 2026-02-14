use crate::js::bindings::element::Element;
use crate::js::bindings::style_declaration::CssStyleDeclaration;
use crate::js::bindings::token_list::DomTokenList;
use rquickjs::{Class, Ctx, Result, Value};

pub fn style<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let decl = CssStyleDeclaration { 
        node: el.node.clone(),
        mutations: el.mutations.clone(),
        stylesheet_dirty: el.stylesheet_dirty.clone(),
    };
    let instance = Class::instance(ctx, decl)?;
    Ok(instance.into_value())
}

pub fn class_list<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let list = DomTokenList { 
        node: el.node.clone(),
        mutations: el.mutations.clone(),
        stylesheet_dirty: el.stylesheet_dirty.clone(),
    };
    let instance = Class::instance(ctx, list)?;
    Ok(instance.into_value())
}
