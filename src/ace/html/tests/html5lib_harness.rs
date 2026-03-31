use crate::ace::html::{HtmlDocument, HtmlNode, Namespace, parse_document, parse_fragment};
use std::fmt::Write;

pub struct Html5Test {
    pub data: String,
    pub errors: Vec<String>,
    pub document: String,
    pub fragment_context: Option<String>,
}

impl Html5Test {
    pub fn from_dat(input: &str) -> Vec<Self> {
        let mut tests = Vec::new();
        let mut current_test = Html5Test {
            data: String::new(),
            errors: Vec::new(),
            document: String::new(),
            fragment_context: None,
        };
        let mut current_section = "";

        for line in input.lines() {
            if line.starts_with('#') {
                let section = line.trim();
                match section {
                    "#data" => {
                        if !current_test.data.is_empty() || current_test.fragment_context.is_some() || !current_test.document.is_empty() {
                            tests.push(current_test);
                            current_test = Html5Test {
                                data: String::new(),
                                errors: Vec::new(),
                                document: String::new(),
                                fragment_context: None,
                            };
                        }
                        current_section = "data";
                    }
                    "#errors" => current_section = "errors",
                    "#document" => current_section = "document",
                    "#document-fragment" => current_section = "fragment",
                    _ => {}
                }
                continue;
            }

            match current_section {
                "data" => {
                    if !current_test.data.is_empty() {
                        current_test.data.push('\n');
                    }
                    current_test.data.push_str(line);
                }
                "errors" => current_test.errors.push(line.to_string()),
                "document" => {
                    current_test.document.push_str(line);
                    current_test.document.push('\n');
                }
                "fragment" => {
                    current_test.fragment_context = Some(line.trim().to_string());
                }
                _ => {}
            }
        }
        if !current_test.data.is_empty() {
            tests.push(current_test);
        }
        tests
    }

    pub fn run(&self) -> Result<(), String> {
        let actual_tree = if let Some(ref ctx) = self.fragment_context {
            let nodes = parse_fragment(&self.data, Some(ctx));
            serialize_fragment(&nodes)
        } else {
            let doc = parse_document(&self.data);
            serialize_document(&doc)
        };

        if actual_tree.trim() != self.document.trim() {
            return Err(format!(
                "Tree mismatch!\nData: {:?}\nContext: {:?}\nExpected:\n{}\nActual:\n{}",
                self.data, self.fragment_context, self.document, actual_tree
            ));
        }
        Ok(())
    }
}

fn serialize_document(doc: &HtmlDocument) -> String {
    let mut out = String::new();
    if let Some(ref doctype) = doc.doctype {
        let name = doctype.name.as_deref().unwrap_or("");
        let public = doctype.public_id.as_deref().unwrap_or("");
        let system = doctype.system_id.as_deref().unwrap_or("");
        
        write!(out, "| <!DOCTYPE {}", name).unwrap();
        if !public.is_empty() || !system.is_empty() {
            write!(out, " \"{}\" \"{}\"", public, system).unwrap();
        }
        out.push_str(">\n");
    }
    for node in &doc.children {
        serialize_node(node, 0, &mut out);
    }
    out
}

fn serialize_fragment(nodes: &[HtmlNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        serialize_node(node, 0, &mut out);
    }
    out
}

fn serialize_node(node: &HtmlNode, indent: usize, out: &mut String) {
    let prefix = "| ".to_string() + &"  ".repeat(indent);
    match node {
        HtmlNode::Element(el) => {
            let ns_prefix = match el.namespace {
                Namespace::Html => "",
                Namespace::Svg => "svg ",
                Namespace::MathMl => "math ",
            };
            writeln!(out, "{}<{}{}>", prefix, ns_prefix, el.tag).unwrap();
            
            // Attributes (sorted)
            let mut attrs: Vec<_> = el.attributes.iter().collect();
            attrs.sort_by(|a, b| a.0.cmp(b.0));
            for (name, value) in attrs {
                writeln!(out, "{}  {}=\"{}\"", prefix, name, value).unwrap();
            }
            
            for child in &el.children {
                serialize_node(child, indent + 1, out);
            }
        }
        HtmlNode::Text(text) => {
            writeln!(out, "{}\"{}\"", prefix, text).unwrap();
        }
        HtmlNode::Comment(text) => {
            writeln!(out, "{}<!-- {} -->", prefix, text).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adoption_agency_01() {
        let dat = r#"
#data
<p><b><i>x</b>y</i></p>
#errors
#document
| <html>
|   <head>
|   <body>
|     <p>
|       <b>
|         <i>
|           "x"
|       <i>
|         "y"
"#;
        let tests = Html5Test::from_dat(dat);
        for test in tests {
            test.run().unwrap();
        }
    }

    #[test]
    fn test_tables_01() {
        let dat = r#"
#data
<table>hello<tr><td>cell</td></tr></table>
#errors
#document
| <html>
|   <head>
|   <body>
|     "hello"
|     <table>
|       <tbody>
|         <tr>
|           <td>
|             "cell"
"#;
        let tests = Html5Test::from_dat(dat);
        for test in tests {
            test.run().unwrap();
        }
    }

    #[test]
    fn test_fragments_01() {
        let dat = r#"
#data
<span>hello</span>
#document-fragment
div
#errors
#document
| <span>
|   "hello"
"#;
        let tests = Html5Test::from_dat(dat);
        for test in tests {
            test.run().unwrap();
        }
    }

    #[test]
    fn test_svg_namespaces_01() {
        let dat = r#"
#data
<div><svg><circle/></svg></div>
#errors
#document
| <html>
|   <head>
|   <body>
|     <div>
|       <svg svg>
|         <svg circle>
"#;
        let tests = Html5Test::from_dat(dat);
        for test in tests {
            test.run().unwrap();
        }
    }
}
