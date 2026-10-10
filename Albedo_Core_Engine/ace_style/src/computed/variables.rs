//! # Resolução de Variáveis CSS e DAG de Dependências (`var()`)
//!
//! Detecção de ciclos em O(V+E), suporte a fallbacks profundos e proteção contra
//! ataques de explosão exponencial de memória (Billion Laughs CSS Attack).

use ace_core::intern::Atom;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::SmolStr;

/// Limite máximo de bytes gerados por expansão de variáveis para evitar exaustão de memória.
pub const MAX_EXPANDED_TOKEN_LEN: usize = 65_536; // 64 KB

/// Profundidade máxima de substituição recursiva de `var()`.
pub const MAX_SUBSTITUTION_DEPTH: usize = 128;

/// Resolvedor de variáveis CSS com detecção de ciclo e proteção de limites.
pub struct VariableResolver<'a> {
    env: &'a FxHashMap<Atom, SmolStr>,
    visiting: FxHashSet<Atom>,
    resolved_cache: FxHashMap<Atom, Option<SmolStr>>,
}

impl<'a> VariableResolver<'a> {
    pub fn new(env: &'a FxHashMap<Atom, SmolStr>) -> Self {
        Self {
            env,
            visiting: FxHashSet::default(),
            resolved_cache: FxHashMap::default(),
        }
    }

    /// Resolve todas as variáveis no ambiente e retorna o mapa com os valores expandidos válidos.
    pub fn resolve_all(&mut self) -> FxHashMap<Atom, SmolStr> {
        let mut final_vars = FxHashMap::default();
        let keys: Vec<Atom> = self.env.keys().cloned().collect();

        for key in keys {
            if let Some(val) = self.resolve_variable(&key, 0) {
                final_vars.insert(key, val);
            }
        }
        final_vars
    }

    /// Resolve uma variável individual pelo nome.
    pub fn resolve_variable(&mut self, name: &Atom, depth: usize) -> Option<SmolStr> {
        if depth >= MAX_SUBSTITUTION_DEPTH {
            return None;
        }

        // Verifica cache de resolução
        if let Some(cached) = self.resolved_cache.get(name) {
            return cached.clone();
        }

        // Detecção de dependência circular no DAG em O(V+E)
        if self.visiting.contains(name) {
            // Ciclo detectado! A variável torna-se inválida no tempo de valor computado
            return None;
        }

        let raw_val = self.env.get(name)?;

        self.visiting.insert(name.clone());
        let resolved = self.substitute_vars_in_str(raw_val.as_str(), depth + 1);
        self.visiting.remove(name);

        self.resolved_cache.insert(name.clone(), resolved.clone());
        resolved
    }

    /// Substitui todas as ocorrências de `var(--name, fallback)` dentro de uma string CSS.
    pub fn substitute_vars_in_str(&mut self, text: &str, depth: usize) -> Option<SmolStr> {
        if depth >= MAX_SUBSTITUTION_DEPTH {
            return None;
        }

        if !text.contains("var(") {
            return Some(SmolStr::new(text));
        }

        let mut output = String::new();
        let mut chars = text.char_indices().peekable();

        while let Some((idx, ch)) = chars.next() {
            if text[idx..].starts_with("var(") {
                // Encontra a chamada var( ... ) com matching de parênteses
                let start_inner = idx + 4;
                let mut paren_depth = 1;
                let mut end_call = None;

                let mut inner_chars = text[start_inner..].char_indices();
                while let Some((inner_idx, inner_ch)) = inner_chars.next() {
                    match inner_ch {
                        '(' => paren_depth += 1,
                        ')' => {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                end_call = Some(start_inner + inner_idx);
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                let Some(call_end) = end_call else {
                    return None; // var( não fechada
                };

                let inner = &text[start_inner..call_end].trim();
                let (var_name_str, fallback_str) = split_var_args(inner);

                let var_atom = Atom::new(var_name_str);
                let resolved_val = self.resolve_variable(&var_atom, depth + 1);

                let val_to_insert = match resolved_val {
                    Some(v) => v.to_string(),
                    None => {
                        // Tenta resolver o fallback se existir
                        if let Some(fb) = fallback_str {
                            let fb_resolved = self.substitute_vars_in_str(fb, depth + 1)?;
                            fb_resolved.to_string()
                        } else {
                            return None; // Sem valor e sem fallback = inválido
                        }
                    }
                };

                // Defesa contra o Billion Laughs Attack
                if output.len() + val_to_insert.len() > MAX_EXPANDED_TOKEN_LEN {
                    return None;
                }

                output.push_str(&val_to_insert);

                // Avança o iterador principal além do ')'
                while let Some(&(next_idx, _)) = chars.peek() {
                    if next_idx <= call_end {
                        chars.next();
                    } else {
                        break;
                    }
                }
            } else {
                if output.len() >= MAX_EXPANDED_TOKEN_LEN {
                    return None;
                }
                output.push(ch);
            }
        }

        Some(SmolStr::new(output))
    }
}

/// Divide os argumentos de `var(--name, fallback)` respeitando parênteses aninhados no fallback.
fn split_var_args(inner: &str) -> (&str, Option<&str>) {
    let mut paren_depth = 0;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            ',' if paren_depth == 0 => {
                let name = inner[..idx].trim();
                let fallback = inner[idx + 1..].trim();
                return (name, Some(fallback));
            }
            _ => {}
        }
    }
    (inner.trim(), None)
}
