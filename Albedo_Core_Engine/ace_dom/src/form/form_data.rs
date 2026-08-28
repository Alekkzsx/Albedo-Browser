//! # Pipeline de Dados de Formulário (FormData — WHATWG XMLHttpRequest/Fetch)
//!
//! Gerencia entradas de formulário e serializa para `application/x-www-form-urlencoded`
//! e `multipart/form-data` de alta performance.

use smol_str::SmolStr;

/// Valor associado a um campo de formulário no `FormData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormDataValue {
    /// Texto simples
    Text(SmolStr),
    /// Arquivo binário com metadados
    File {
        name: SmolStr,
        mime_type: SmolStr,
        data: Vec<u8>,
    },
}

impl FormDataValue {
    /// Retorna o valor como string de texto se for uma entrada textual.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s.as_str()),
            Self::File { .. } => None,
        }
    }
}

/// Entrada individual em uma estrutura `FormData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormDataEntry {
    pub name: SmolStr,
    pub value: FormDataValue,
}

/// A estrutura `FormData` padrão para submissões de rede.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormData {
    entries: Vec<FormDataEntry>,
}

impl FormData {
    /// Cria uma nova instância vazia de `FormData`.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adiciona um campo de texto ao formulário.
    pub fn append(&mut self, name: &str, value: &str) {
        self.entries.push(FormDataEntry {
            name: SmolStr::new(name),
            value: FormDataValue::Text(SmolStr::new(value)),
        });
    }

    /// Adiciona um arquivo binário ao formulário.
    pub fn append_file(&mut self, name: &str, file_name: &str, mime_type: &str, data: Vec<u8>) {
        self.entries.push(FormDataEntry {
            name: SmolStr::new(name),
            value: FormDataValue::File {
                name: SmolStr::new(file_name),
                mime_type: SmolStr::new(mime_type),
                data,
            },
        });
    }

    /// Retorna o primeiro valor associado a uma chave, se existir.
    pub fn get(&self, name: &str) -> Option<&FormDataValue> {
        self.entries
            .iter()
            .find(|e| e.name.as_str() == name)
            .map(|e| &e.value)
    }

    /// Retorna todos os valores associados a uma chave específica.
    pub fn get_all(&self, name: &str) -> Vec<&FormDataValue> {
        self.entries
            .iter()
            .filter(|e| e.name.as_str() == name)
            .map(|e| &e.value)
            .collect()
    }

    /// Verifica se há pelo menos um campo com a chave informada.
    pub fn has(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name.as_str() == name)
    }

    /// Remove todos os campos associados a uma chave.
    pub fn delete(&mut self, name: &str) {
        self.entries.retain(|e| e.name.as_str() != name);
    }

    /// Define o valor de uma chave, substituindo entradas anteriores existentes.
    pub fn set(&mut self, name: &str, value: &str) {
        self.delete(name);
        self.append(name, value);
    }

    /// Retorna o número total de entradas presentes.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Retorna `true` se não houver entradas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retorna um iterador sobre as entradas.
    pub fn iter(&self) -> impl Iterator<Item = &FormDataEntry> {
        self.entries.iter()
    }

    /// Serializa as entradas textuais no formato `application/x-www-form-urlencoded`.
    pub fn to_url_encoded(&self) -> String {
        let mut out = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            if let FormDataValue::Text(ref val) = entry.value {
                if i > 0 {
                    out.push('&');
                }
                out.push_str(&url_encode(entry.name.as_str()));
                out.push('=');
                out.push_str(&url_encode(val.as_str()));
            }
        }
        out
    }

    /// Serializa todas as entradas no formato `multipart/form-data` usando o delimitador (*boundary*) informado.
    pub fn to_multipart(&self, boundary: &str) -> Vec<u8> {
        let mut body = Vec::new();

        for entry in &self.entries {
            body.extend_from_slice(b"--");
            body.extend_from_slice(boundary.as_bytes());
            body.extend_from_slice(b"\r\n");

            match &entry.value {
                FormDataValue::Text(val) => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"\r\n\r\n{}",
                            entry.name, val
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(b"\r\n");
                }
                FormDataValue::File {
                    name,
                    mime_type,
                    data,
                } => {
                    body.extend_from_slice(
                        format!(
                            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                            entry.name, name, mime_type
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(data);
                    body.extend_from_slice(b"\r\n");
                }
            }
        }

        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"--\r\n");

        body
    }

    /// Constrói um `FormData` a partir de um elemento `<form>`, incluindo controles aninhados
    /// e controles externos associados via atributo `form="form_id"` (WHATWG HTML §4.10).
    pub fn from_form_element(doc: &crate::tree::Document, form_id: ace_core::id::NodeId) -> Self {
        let mut form_data = Self::new();

        let form_id_attr = doc.get_node(form_id).and_then(|n| n.as_element()).and_then(|el| el.get_attribute("id"));

        let mut candidate_ids = Vec::new();

        // 1. Controles filhos diretos/indiretos do form
        for (child_id, _) in doc.descendants(form_id) {
            candidate_ids.push(child_id);
        }

        // 2. Controles externos com atributo form="form_id"
        if let Some(fid) = form_id_attr {
            for (node_id, node) in doc.descendants(doc.root()) {
                if let Some(el) = node.as_element() {
                    if el.get_attribute("form") == Some(fid) && !candidate_ids.contains(&node_id) {
                        candidate_ids.push(node_id);
                    }
                }
            }
        }

        for node_id in candidate_ids {
            if let Some(node) = doc.get_node(node_id) {
                if let Some(el) = node.as_element() {
                    let tag = el.tag_name.as_str();
                    if matches!(tag, "input" | "textarea" | "select") {
                        if el.has_attribute("disabled") {
                            continue;
                        }

                        let name = match el.get_attribute("name") {
                            Some(n) if !n.is_empty() => n,
                            _ => continue,
                        };

                        let val = if tag == "textarea" {
                            let mut text = String::new();
                            for (_, child_node) in doc.children(node_id) {
                                if let Some(t) = child_node.text_content() {
                                    text.push_str(t);
                                }
                            }
                            if text.is_empty() {
                                el.get_attribute("value").unwrap_or_default().to_string()
                            } else {
                                text
                            }
                        } else if tag == "select" {
                            let mut selected_val = None;
                            for (opt_id, opt_node) in doc.children(node_id) {
                                if let Some(opt_el) = opt_node.as_element() {
                                    if opt_el.tag_name.eq_ignore_ascii_case("option") {
                                        let is_selected = opt_el.has_attribute("selected");
                                        let opt_v = opt_el.get_attribute("value").map(|s| s.to_string()).unwrap_or_else(|| {
                                            doc.children(opt_id).filter_map(|(_, n)| n.text_content()).collect::<String>()
                                        });
                                        if is_selected || selected_val.is_none() {
                                            selected_val = Some(opt_v);
                                        }
                                    }
                                }
                            }
                            selected_val.unwrap_or_default()
                        } else {
                            el.get_attribute("value").unwrap_or_default().to_string()
                        };

                        form_data.append(name, &val);
                    }
                }
            }
        }

        form_data
    }
}

/// Codifica caracteres para o padrão percent-encoding do application/x-www-form-urlencoded.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            other => {
                out.push_str(&format!("%{:02X}", other));
            }
        }
    }
    out
}
