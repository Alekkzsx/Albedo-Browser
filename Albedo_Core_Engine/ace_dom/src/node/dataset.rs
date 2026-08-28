//! # Mapeamento de Atributos Personalizados de Dados (DOMStringMap / dataset — WHATWG §3.2.6.2)
//!
//! Fornece sincronização bidirecional entre atributos `data-*` do HTML e propriedades camelCase em `element.dataset`.

use crate::node::ElementData;
use smol_str::SmolStr;

/// Objeto `DOMStringMap` representando a propriedade `dataset` de um elemento HTML.
pub struct DOMStringMap<'a> {
    element: &'a mut ElementData,
}

impl<'a> DOMStringMap<'a> {
    /// Cria uma nova visão `DOMStringMap` sobre os atributos do elemento.
    pub fn new(element: &'a mut ElementData) -> Self {
        Self { element }
    }

    /// Converte um nome camelCase (ex: `userId`) para o formato de atributo HTML `data-*` (ex: `data-user-id`).
    pub fn camel_to_data_attr(camel: &str) -> String {
        let mut out = String::with_capacity(camel.len() + 5);
        out.push_str("data-");
        for c in camel.chars() {
            if c.is_ascii_uppercase() {
                out.push('-');
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Converte um nome de atributo HTML `data-*` (ex: `data-user-id`) para o formato camelCase (ex: `userId`).
    pub fn data_attr_to_camel(attr_name: &str) -> Option<String> {
        let trimmed = attr_name.strip_prefix("data-")?;
        let mut out = String::with_capacity(trimmed.len());
        let mut capitalize_next = false;

        for c in trimmed.chars() {
            if c == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                out.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(c);
            }
        }
        Some(out)
    }

    /// Obtém o valor associado a uma chave camelCase.
    pub fn get(&self, key: &str) -> Option<&str> {
        let attr_name = Self::camel_to_data_attr(key);
        self.element.get_attribute(&attr_name)
    }

    /// Define ou atualiza o valor de uma chave camelCase no atributo `data-*`.
    pub fn set(&mut self, key: &str, value: &str) {
        let attr_name = Self::camel_to_data_attr(key);
        self.element.set_attribute(attr_name, value);
    }

    /// Remove o atributo `data-*` correspondente à chave camelCase.
    pub fn remove(&mut self, key: &str) -> bool {
        let attr_name = Self::camel_to_data_attr(key);
        self.element.remove_attribute(&attr_name)
    }

    /// Retorna `true` se a chave informada estiver presente no `dataset`.
    pub fn contains(&self, key: &str) -> bool {
        let attr_name = Self::camel_to_data_attr(key);
        self.element.has_attribute(&attr_name)
    }

    /// Retorna todos os pares (chave camelCase, valor) presentes no `dataset`.
    pub fn entries(&self) -> Vec<(String, SmolStr)> {
        let mut list = Vec::new();
        for attr in self.element.attributes.as_slice() {
            if let Some(camel_key) = Self::data_attr_to_camel(attr.name.as_str()) {
                list.push((camel_key, attr.value.clone()));
            }
        }
        list
    }
}
