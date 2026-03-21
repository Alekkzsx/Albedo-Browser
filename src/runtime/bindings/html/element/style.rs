use super::Element;
use crate::runtime::bindings::html::style_declaration::CssStyleDeclaration;
use crate::runtime::bindings::html::token_list::DomTokenList;
use rquickjs::{Class, Ctx, Result, Value};

pub fn style<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let decl = CssStyleDeclaration {
        dom: el.dom.clone(),
        index: el.index,
        mutations: el.mutations.clone(),
        stylesheet_dirty: el.stylesheet_dirty.clone(),
    };
    let instance = Class::instance(ctx, decl)?;
    Ok(instance.into_value())
}

pub fn class_list<'js>(el: &Element, ctx: Ctx<'js>) -> Result<Value<'js>> {
    let list = DomTokenList {
        dom: el.dom.clone(),
        index: el.index,
        mutations: el.mutations.clone(),
        stylesheet_dirty: el.stylesheet_dirty.clone(),
    };
    let instance = Class::instance(ctx, list)?;
    Ok(instance.into_value())
}
