//! # API DOMParser Multimídia (WHATWG DOMParsing and Serialization)
//!
//! Permite o parsing unificado de documentos a partir de strings com despacho por MIME-Type.

use crate::error::DomError;
use crate::node::element::Namespace;
use crate::parse_html;
use crate::tree::Document;

/// Tipos MIME suportados pelo `DOMParser`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedType {
    /// HTML clássico (`text/html`)
    TextHtml,
    /// Vetor SVG XML (`image/svg+xml`)
    ImageSvgXml,
    /// XML Genérico (`application/xml`)
    ApplicationXml,
    /// Texto XML (`text/xml`)
    TextXml,
}

impl SupportedType {
    /// Faz o parsing de uma string MIME para o tipo enum suportado.
    pub fn from_mime(mime: &str) -> Result<Self, DomError> {
        let lower = mime.trim().to_ascii_lowercase();
        match lower.as_str() {
            "text/html" => Ok(Self::TextHtml),
            "image/svg+xml" => Ok(Self::ImageSvgXml),
            "application/xml" => Ok(Self::ApplicationXml),
            "text/xml" | "application/xhtml+xml" => Ok(Self::TextXml),
            _ => Err(DomError::HierarchyRequestError(format!(
                "Tipo MIME '{}' não é suportado pelo DOMParser",
                mime
            ))),
        }
    }
}

/// O parser DOM multimídia oficial.
#[derive(Debug, Default)]
pub struct DOMParser;

impl DOMParser {
    /// Cria uma nova instância de `DOMParser`.
    pub fn new() -> Self {
        Self
    }

    /// Faz o parsing de uma string de acordo com o tipo MIME informado.
    pub fn parse_from_string(&self, text: &str, mime_type: SupportedType) -> Result<Document, DomError> {
        match mime_type {
            SupportedType::TextHtml => Ok(parse_html(text)),
            SupportedType::ImageSvgXml => {
                let mut doc = Document::new(None);
                let root_id = doc.root();
                let svg_id = doc.create_element("svg", Namespace::Svg);
                let _ = doc.append_child(root_id, svg_id);
                doc.document_element = Some(svg_id);

                // Parsing contextual dos nós SVG
                let frag_id = crate::parse_fragment(&mut doc, Some(svg_id), text);
                let children: Vec<_> = doc.children(frag_id).map(|(id, _)| id).collect();
                for child in children {
                    let _ = doc.append_child(svg_id, child);
                }
                let _ = doc.remove_child(root_id, frag_id);
                Ok(doc)
            }
            SupportedType::ApplicationXml | SupportedType::TextXml => {
                // XML bem-formado: despacha para o parser HTML5 estruturado
                Ok(parse_html(text))
            }
        }
    }

    /// Faz o parsing a partir de uma string MIME em texto plano.
    pub fn parse_from_str(&self, text: &str, mime_str: &str) -> Result<Document, DomError> {
        let mime = SupportedType::from_mime(mime_str)?;
        self.parse_from_string(text, mime)
    }
}
