//! # CSSStyleDeclaration (WHATWG CSSOM §6.4.1)
//!
//! Estrutura compacta para armazenamento de propriedades CSS com `InlineVec<CSSProperty, 8>`
//! (zero alocação no heap para estilos inline de até 8 propriedades) e parsing de alto desempenho.

use ace_core::collections::InlineVec;
use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Helper normativo para comparar nomes de propriedades CSS:
/// Custom properties (`--*`) são estritamente case-sensitive (W3C CSS Variables Level 1 §2).
/// Standard properties são ASCII case-insensitive.
#[inline]
pub(crate) fn prop_name_matches(stored_name: &str, query_name: &str) -> bool {
    if stored_name.starts_with("--") || query_name.starts_with("--") {
        stored_name == query_name
    } else {
        stored_name.eq_ignore_ascii_case(query_name)
    }
}

/// Uma declaração de propriedade CSS (ex: `color: red !important;`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSSProperty {
    pub name: Atom,
    pub value: SmolStr,
    pub important: bool,
}

impl CSSProperty {
    pub fn new(name: impl Into<Atom>, value: impl Into<SmolStr>, important: bool) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            important,
        }
    }
}

/// Representação de uma lista de declarações CSS (Blink `CSSPropertyValueSet` pattern).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CSSStyleDeclaration {
    pub properties: InlineVec<CSSProperty, 8>,
}

/// Remove comentários CSS `/* ... */` fora de strings literais (`'...'` ou `"..."`),
/// substituindo-os por um espaço em branco conforme W3C CSS Syntax Level 3 §4.1.3.
fn strip_css_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;

    while i < len {
        let ch = chars[i];
        if ch == '\\' && i + 1 < len && (in_single || in_double) {
            out.push(ch);
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if !in_single && !in_double && ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i < len {
                if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            out.push(' ');
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        }

        out.push(ch);
        i += 1;
    }
    out
}

impl CSSStyleDeclaration {
    /// Cria uma nova declaração de estilo vazia.
    pub fn new() -> Self {
        Self {
            properties: InlineVec::new(),
        }
    }

    /// Faz o parsing de uma string de estilo em linha (ex: `color: #fff; font-size: 16px;`),
    /// respeitando comentários `/* ... */`, aspas simples/duplas e parênteses ao delimitar declarações.
    pub fn parse(css_text: &str) -> Self {
        let clean_text = strip_css_comments(css_text);
        let mut decl = Self::new();
        let chars: Vec<char> = clean_text.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut chunk_start = 0;
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut paren_depth: usize = 0;

        while i < len {
            let ch = chars[i];
            if ch == '\\' && i + 1 < len {
                i += 2;
                continue;
            }
            if ch == '\'' && !in_double_quote {
                in_single_quote = !in_single_quote;
            } else if ch == '"' && !in_single_quote {
                in_double_quote = !in_double_quote;
            } else if !in_single_quote && !in_double_quote {
                if ch == '(' {
                    paren_depth += 1;
                } else if ch == ')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if ch == ';' && paren_depth == 0 {
                    let chunk: String = chars[chunk_start..i].iter().collect();
                    Self::parse_declaration_chunk(&mut decl, &chunk);
                    chunk_start = i + 1;
                }
            }
            i += 1;
        }

        if chunk_start < len {
            let chunk: String = chars[chunk_start..len].iter().collect();
            Self::parse_declaration_chunk(&mut decl, &chunk);
        }

        decl
    }

    fn parse_declaration_chunk(decl: &mut Self, chunk: &str) {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            return;
        }

        let chars: Vec<char> = trimmed.chars().collect();
        let len = chars.len();
        let mut colon_pos = None;
        let mut in_single = false;
        let mut in_double = false;
        let mut paren_depth: usize = 0;
        let mut i = 0;

        while i < len {
            let ch = chars[i];
            if ch == '\\' && i + 1 < len {
                i += 2;
                continue;
            }
            if ch == '\'' && !in_double {
                in_single = !in_single;
            } else if ch == '"' && !in_single {
                in_double = !in_double;
            } else if !in_single && !in_double {
                if ch == '(' {
                    paren_depth += 1;
                } else if ch == ')' {
                    paren_depth = paren_depth.saturating_sub(1);
                } else if ch == ':' && paren_depth == 0 {
                    colon_pos = Some(i);
                    break;
                }
            }
            i += 1;
        }

        if let Some(pos) = colon_pos {
            let name_part: String = chars[..pos].iter().collect();
            let name_trimmed = name_part.trim();
            let val_part: String = chars[pos + 1..].iter().collect();
            let val_trimmed = val_part.trim();
            if name_trimmed.is_empty() || val_trimmed.is_empty() {
                return;
            }

            // Localiza o último '!' no nível zero (fora de aspas e parênteses, não escapado)
            let val_chars: Vec<char> = val_trimmed.chars().collect();
            let val_len = val_chars.len();
            let mut last_top_level_exclamation = None;
            let mut in_s = false;
            let mut in_d = false;
            let mut p_depth: usize = 0;
            let mut vi = 0;

            while vi < val_len {
                let ch = val_chars[vi];
                if ch == '\\' && vi + 1 < val_len {
                    vi += 2;
                    continue;
                }
                if ch == '\'' && !in_d {
                    in_s = !in_s;
                } else if ch == '"' && !in_s {
                    in_d = !in_d;
                } else if !in_s && !in_d {
                    if ch == '(' {
                        p_depth += 1;
                    } else if ch == ')' {
                        p_depth = p_depth.saturating_sub(1);
                    } else if ch == '!' && p_depth == 0 {
                        last_top_level_exclamation = Some(vi);
                    }
                }
                vi += 1;
            }

            let mut important = false;
            let final_val: String;

            if let Some(idx) = last_top_level_exclamation {
                let suffix: String = val_chars[idx + 1..].iter().collect();
                if suffix.trim().eq_ignore_ascii_case("important") {
                    important = true;
                    final_val = val_chars[..idx].iter().collect::<String>().trim().to_string();
                } else {
                    final_val = val_trimmed.to_string();
                }
            } else {
                final_val = val_trimmed.to_string();
            }

            let name = if name_trimmed.starts_with("--") {
                SmolStr::new(name_trimmed)
            } else {
                SmolStr::new(name_trimmed.to_ascii_lowercase())
            };

            if !name.is_empty() && !final_val.is_empty() {
                decl.set_property(name.as_str(), final_val.as_str(), important);
            }
        }
    }

    /// Retorna o valor de uma propriedade CSS se definida.
    pub fn get_property_value(&self, prop_name: &str) -> Option<&str> {
        self.properties
            .as_slice()
            .iter()
            .find(|p| prop_name_matches(p.name.as_str(), prop_name))
            .map(|p| p.value.as_str())
    }

    /// Retorna a prioridade (`"important"` ou `""`) de uma propriedade CSS.
    pub fn get_property_priority(&self, prop_name: &str) -> &'static str {
        self.properties
            .as_slice()
            .iter()
            .find(|p| prop_name_matches(p.name.as_str(), prop_name))
            .map(|p| if p.important { "important" } else { "" })
            .unwrap_or("")
    }

    /// Define ou atualiza uma propriedade CSS.
    pub fn set_property(
        &mut self,
        prop_name: impl Into<Atom>,
        value: impl Into<SmolStr>,
        important: bool,
    ) {
        let name_atom = prop_name.into();
        let value_smol = value.into();
        let name_str = name_atom.as_str();

        if let Some(existing) = self
            .properties
            .as_mut_slice()
            .iter_mut()
            .find(|p| prop_name_matches(p.name.as_str(), name_str))
        {
            existing.name = name_atom;
            existing.value = value_smol;
            existing.important = important;
        } else {
            self.properties.push(CSSProperty {
                name: name_atom,
                value: value_smol,
                important,
            });
        }
    }

    /// Remove uma propriedade CSS se ela existir, retornando o valor antigo.
    pub fn remove_property(&mut self, prop_name: &str) -> Option<SmolStr> {
        let pos = self
            .properties
            .as_slice()
            .iter()
            .position(|p| prop_name_matches(p.name.as_str(), prop_name))?;
        Some(self.properties.remove(pos).value)
    }

    /// Retorna o número de propriedades declaradas.
    #[inline]
    pub fn len(&self) -> usize {
        self.properties.len()
    }

    /// Retorna `true` se não houver propriedades declaradas.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    /// Serializa as propriedades em formato CSS normativo (`cssText`).
    pub fn css_text(&self) -> String {
        let mut out = String::new();
        for (i, prop) in self.properties.as_slice().iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(prop.name.as_str());
            out.push_str(": ");
            out.push_str(prop.value.as_str());
            if prop.important {
                out.push_str(" !important");
            }
            out.push(';');
        }
        out
    }
}
