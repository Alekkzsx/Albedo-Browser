use crate::js::JsRuntime;
use rquickjs::{Ctx, Object, Function};
use rquickjs::prelude::Rest;
use std::sync::{Arc, Mutex};
use std::io::Write;

/// Console API implementation for JavaScript
/// 
/// Provides console.log, console.error, console.warn, etc.
pub struct Console {
    output: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Console {
    /// Create a new Console that writes to stdout
    pub fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(Box::new(std::io::stdout()))),
        }
    }
    
    /// Create a Console with custom output
    pub fn with_output(output: Box<dyn Write + Send>) -> Self {
        Self {
            output: Arc::new(Mutex::new(output)),
        }
    }
    
    /// Register console API in the JavaScript runtime
    pub fn register(runtime: &JsRuntime) -> Result<(), rquickjs::Error> {
        runtime.with_context(|ctx| {
            ctx.with(|ctx: Ctx| {
                Self::register_in_ctx(&ctx)
            })
        })
    }

    pub fn register_in_ctx(ctx: &Ctx<'_>) -> Result<(), rquickjs::Error> {
        let console = Object::new(ctx.clone())?;
        
        // Register functions
        console.set("log", Function::new(ctx.clone(), console_log)?)?;
        console.set("error", Function::new(ctx.clone(), console_error)?)?;
        console.set("warn", Function::new(ctx.clone(), console_warn)?)?;
        console.set("info", Function::new(ctx.clone(), console_info)?)?;
        console.set("debug", Function::new(ctx.clone(), console_debug)?)?;
        
        // Register in global scope
        let globals = ctx.globals();
        globals.set("console", console)?;
        
        Ok(())
    }
}

fn format_values<'a>(ctx: &Ctx<'a>, args: &[rquickjs::Value<'a>]) -> String {
    args.iter()
        .map(|arg| value_to_string(ctx, arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn console_log<'a>(ctx: Ctx<'a>, args: Rest<rquickjs::Value<'a>>) -> Result<(), rquickjs::Error> {
    let formatted = format_values(&ctx, &args.0);
    println!("[LOG] {}", formatted);
    Ok(())
}

fn console_error<'a>(ctx: Ctx<'a>, args: Rest<rquickjs::Value<'a>>) -> Result<(), rquickjs::Error> {
    let formatted = format_values(&ctx, &args.0);
    eprintln!("[ERROR] {}", formatted);
    Ok(())
}

fn console_warn<'a>(ctx: Ctx<'a>, args: Rest<rquickjs::Value<'a>>) -> Result<(), rquickjs::Error> {
    let formatted = format_values(&ctx, &args.0);
    eprintln!("[WARN] {}", formatted);
    Ok(())
}

fn console_info<'a>(ctx: Ctx<'a>, args: Rest<rquickjs::Value<'a>>) -> Result<(), rquickjs::Error> {
    let formatted = format_values(&ctx, &args.0);
    println!("[INFO] {}", formatted);
    Ok(())
}

fn console_debug<'a>(ctx: Ctx<'a>, args: Rest<rquickjs::Value<'a>>) -> Result<(), rquickjs::Error> {
    let formatted = format_values(&ctx, &args.0);
    println!("[DEBUG] {}", formatted);
    Ok(())
}

/// Convert a JavaScript value to a string representation
fn value_to_string<'a>(ctx: &Ctx<'a>, value: &rquickjs::Value<'a>) -> String {
    use rquickjs::Type;
    
    match value.type_of() {
        Type::Null => "null".to_string(),
        Type::Undefined => "undefined".to_string(),
        Type::Bool => {
            value.as_bool().map(|b| b.to_string()).unwrap_or_else(|| "false".to_string())
        },
        Type::Int => {
            value.as_int().map(|i| i.to_string()).unwrap_or_else(|| "0".to_string())
        },
        Type::Float => {
            value.as_number().map(|f| f.to_string()).unwrap_or_else(|| "0.0".to_string())
        },
        Type::String => {
            value.as_string()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_else(|| "".to_string())
        },
        Type::Array => {
            // Try to format as array
            if let Some(arr) = value.as_array() {
                let len = arr.len();
                let items: Vec<String> = (0..len.min(10))
                    .filter_map(|i| arr.get::<rquickjs::Value>(i).ok())
                    .map(|v| value_to_string(ctx, &v))
                    .collect();
                
                if len > 10 {
                    format!("[{}, ... ({} more)]", items.join(", "), len - 10)
                } else {
                    format!("[{}]", items.join(", "))
                }
            } else {
                "[Array]".to_string()
            }
        },
        Type::Object => {
            // Try to stringify as JSON
            // json_stringify requires value to be strictly bound to context lifetime
            // We use a workaround by trying to convert to string directly if it's an object
            // or just use generic Object representation if json fails
            if let Ok(Some(json_val)) = ctx.json_stringify(value.clone()) {
                if let Some(js_string) = json_val.as_string() {
                    if let Ok(s) = js_string.to_string() {
                        return s;
                    }
                }
            }
            "[Object]".to_string()
        },
        Type::Function => "[Function]".to_string(),
        Type::Constructor => "[Constructor]".to_string(),
        _ => "[Unknown]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_console_log() {
        let rt = JsRuntime::new().unwrap();
        Console::register(&rt).unwrap();
        
        // Should not crash
        rt.execute_script("console.log('Hello, World!')").unwrap();
    }
    
    #[test]
    fn test_console_multiple_args() {
        let rt = JsRuntime::new().unwrap();
        Console::register(&rt).unwrap();
        
        rt.execute_script("console.log('Number:', 42, 'Boolean:', true, 'Null:', null)").unwrap();
    }
    
    #[test]
    fn test_console_error() {
        let rt = JsRuntime::new().unwrap();
        Console::register(&rt).unwrap();
        
        rt.execute_script("console.error('This is an error')").unwrap();
    }
    
    #[test]
    fn test_console_objects() {
        let rt = JsRuntime::new().unwrap();
        Console::register(&rt).unwrap();
        
        rt.execute_script("console.log({name: 'Albedo', version: '0.1.0'})").unwrap();
    }
}
