use crate::value::JsonValue;

pub fn stringify(value: &JsonValue, pretty: bool) -> String {
    let mut result = String::new();
    serialize_value(value, &mut result, pretty, 0);
    result
}

fn serialize_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '/' => out.push_str("\\/"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => {
                if c.is_control() {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                } else {
                    out.push(c);
                }
            }
        }
    }
    out.push('"');
}

fn serialize_value(value: &JsonValue, out: &mut String, pretty: bool, indent_level: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => out.push_str(&n.to_string()),
        JsonValue::String(s) => serialize_string(s, out),
        JsonValue::Array(arr) => {
            out.push('[');
            if !arr.is_empty() {
                if pretty {
                    out.push('\n');
                }
                for (i, val) in arr.iter().enumerate() {
                    if pretty {
                        indent(out, indent_level + 1);
                    }
                    serialize_value(val, out, pretty, indent_level + 1);
                    if i < arr.len() - 1 {
                        out.push(',');
                    }
                    if pretty {
                        out.push('\n');
                    }
                }
                if pretty {
                    indent(out, indent_level);
                }
            }
            out.push(']');
        }
        JsonValue::Object(obj) => {
            out.push('{');
            if !obj.is_empty() {
                if pretty {
                    out.push('\n');
                }
                let mut entries: Vec<_> = obj.iter().collect();
                // Ordenar chaves para deterministicos testes se necessário, mas HashMap é aleatório.
                // Aqui mantemos a ordem do iterador.
                for (i, (key, val)) in entries.iter().enumerate() {
                    if pretty {
                        indent(out, indent_level + 1);
                    }
                    serialize_string(key, out);
                    out.push_str(": ");
                    serialize_value(val, out, pretty, indent_level + 1);
                    if i < entries.len() - 1 {
                        out.push(',');
                    }
                    if pretty {
                        out.push('\n');
                    }
                }
                if pretty {
                    indent(out, indent_level);
                }
            }
            out.push('}');
        }
    }
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}
