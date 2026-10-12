//! # Custom Properties (Variáveis CSS `--*` e Função `var()`)
//!
//! Representação tipada de nomes de variáveis CSS conforme CSS Custom Properties Level 1.

use ace_core::intern::Atom;
use smol_str::SmolStr;

/// Nome de uma propriedade customizada CSS (garantido iniciar com `--`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomPropertyName(Atom);

impl CustomPropertyName {
    /// Cria um `CustomPropertyName` se o identificador começar com `--` e possuir pelo menos 1 caractere subsequente.
    pub fn new(name: impl AsRef<str>) -> Option<Self> {
        let s = name.as_ref().trim();
        if s.starts_with("--") && s.len() > 2 {
            Some(Self(Atom::new(s)))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn atom(&self) -> &Atom {
        &self.0
    }
}

/// Referência a uma variável CSS `var(--name, fallback)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef {
    pub name: CustomPropertyName,
    pub fallback: Option<SmolStr>,
}

impl VarRef {
    /// Faz o parsing de uma chamada `var(...)`.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        let inner = trimmed
            .strip_prefix("var(")
            .and_then(|s| s.strip_suffix(')'))?
            .trim();

        // Extrai o nome da variável e o fallback (se houver, respeitando parênteses aninhados)
        let mut paren_depth = 0;
        let mut comma_pos = None;

        for (idx, ch) in inner.char_indices() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                }
                ',' if paren_depth == 0 => {
                    comma_pos = Some(idx);
                    break;
                }
                _ => {}
            }
        }

        if let Some(pos) = comma_pos {
            let var_name = inner[..pos].trim();
            let fallback_str = inner[pos + 1..].trim();
            let name = CustomPropertyName::new(var_name)?;
            Some(Self {
                name,
                fallback: Some(SmolStr::new(fallback_str)),
            })
        } else {
            let name = CustomPropertyName::new(inner)?;
            Some(Self {
                name,
                fallback: None,
            })
        }
    }
}
