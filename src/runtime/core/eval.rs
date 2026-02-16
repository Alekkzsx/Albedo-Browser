use super::runtime::{JsRuntime, JsResult};
use rquickjs::Value;

pub fn execute_script(rt: &JsRuntime, code: &str) -> JsResult<String> {
    println!("[JS] Executing script ({} bytes)...", code.len());
    let ctx = rt.context.lock().unwrap();
    ctx.with(|ctx| {
        match ctx.eval::<Value, _>(code) {
            Ok(result) => {
                println!("[JS] Script execution success.");
                // Try to convert to string, fallback to debug
                if let Some(s) = result.as_string() {
                    Ok(s.to_string()?)
                } else if result.is_null() {
                    Ok("null".to_string())
                } else if result.is_undefined() {
                    Ok("undefined".to_string())
                } else {
                    // Try JSON stringify
                    match ctx.json_stringify(result.clone()) {
                        Ok(Some(s)) => Ok(s.to_string()?),
                        _ => Ok(format!("{:?}", result))
                    }
                }
            },
            Err(e) => {
                println!("[JS] Script execution FAILED.");
                let exception_val = ctx.catch();
                if let Some(obj) = exception_val.as_object() {
                    let mut msg = String::new();
                    if let Ok(message) = obj.get::<_, String>("message") {
                        msg.push_str(&message);
                    } else {
                        msg.push_str(&format!("{:?}", exception_val));
                    }
                    
                    if let Ok(stack) = obj.get::<_, String>("stack") {
                        msg.push_str("\nStack:\n");
                        msg.push_str(&stack);
                    }
                    eprintln!("\n[JS EXCEPTION DETAILED]\n{}\n", msg);
                } else {
                        eprintln!("\n[JS EXCEPTION DETAILED] {:?}\n", exception_val);
                }
                Err(e)
            }
        }
    })
}
