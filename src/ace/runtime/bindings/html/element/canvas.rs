use super::canvas_context::CanvasRenderingContext2D;
use super::Element;
use crate::ace::engine::dom::AceNodeType;
use crate::ace::engine::graphics::canvas2d::Canvas2D;
use rquickjs::{Class, Ctx, Result, Value};

/// TODO: add docs
pub fn get_context<'js>(el: &Element, ctx: Ctx<'js>, type_: String) -> Result<Value<'js>> {
    if type_ != "2d" {
        return Ok(Value::new_null(ctx));
    }

    // Verificar se é um elemento canvas
    let is_canvas = if let Ok(dom) = el.dom.lock() {
        if let Some(node) = dom.get_node(el.index) {
            if let AceNodeType::Element(element) = &node.node_type {
                element.tag == "canvas"
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if !is_canvas {
        return Ok(Value::new_null(ctx));
    }

    // Obter ou criar contexto
    let mut contexts = el.canvas_contexts.lock().unwrap_or_else(|e| e.into_inner());
    if !contexts.contains_key(&el.index) {
        // Obter largura/altura dos atributos ou padrão (300x150)
        let w = super::props::width(el) as u32;
        let h = super::props::height(el) as u32;
        let w = if w == 0 { 300 } else { w };
        let h = if h == 0 { 150 } else { h };

        contexts.insert(el.index, Canvas2D::new(w, h));
    }

    let canvas_context = CanvasRenderingContext2D {
        dom: el.dom.clone(),
        index: el.index,
        canvas_contexts: el.canvas_contexts.clone(),
    };

    let instance = Class::instance(ctx, canvas_context)?;
    Ok(instance.into_value())
}
