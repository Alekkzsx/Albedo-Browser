//! # Motor de Public Suffix List (PSL) e eTLD+1 (Chromium Registry Controlled Domains Pattern)
//!
//! A determinação de eTLD+1 (*effective Top-Level Domain + 1 label*) define as fronteiras de segurança
//! do navegador para isolamento de cookies (RFC 6265bis), `SchemefulSite` e particionamento de processos (Site Isolation).
//!
//! Implementado via árvore compacta de sufixos com suporte a regras exatas, wildcards (`*`) e exceções (`!`).

use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleType {
    Normal,
    Wildcard,
    Exception,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainCategory {
    Icann,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PslMatch {
    pub rule_type: RuleType,
    pub category: DomainCategory,
    pub suffix_labels: usize,
}

/// Árvore compacta para matching da Public Suffix List em $O(k)$ sobre os labels do host.
pub struct CompactPslTrie {
    nodes: Vec<TrieNode>,
}

struct TrieNode {
    children: FxHashMap<String, usize>,
    rule: Option<(RuleType, DomainCategory)>,
}

impl Default for CompactPslTrie {
    fn default() -> Self {
        Self::new()
    }
}

impl CompactPslTrie {
    pub fn new() -> Self {
        Self {
            nodes: vec![TrieNode {
                children: FxHashMap::default(),
                rule: None,
            }],
        }
    }

    /// Cria uma instância pré-populada com os TLDs e ccTLDs mais comuns.
    pub fn new_with_standard_rules() -> Self {
        let mut trie = Self::new();
        let standard_rules = [
            ("com", DomainCategory::Icann),
            ("org", DomainCategory::Icann),
            ("net", DomainCategory::Icann),
            ("edu", DomainCategory::Icann),
            ("gov", DomainCategory::Icann),
            ("io", DomainCategory::Icann),
            ("dev", DomainCategory::Icann),
            ("app", DomainCategory::Icann),
            ("co.uk", DomainCategory::Icann),
            ("gov.uk", DomainCategory::Icann),
            ("com.br", DomainCategory::Icann),
            ("org.br", DomainCategory::Icann),
            ("gov.br", DomainCategory::Icann),
            ("com.au", DomainCategory::Icann),
        ];

        for (rule, cat) in standard_rules {
            trie.insert(rule, cat);
        }
        trie
    }

    /// Insere uma regra na PSL.
    pub fn insert(&mut self, rule: &str, category: DomainCategory) {
        let (rule_type, clean_rule) = if let Some(stripped) = rule.strip_prefix('!') {
            (RuleType::Exception, stripped)
        } else if let Some(stripped) = rule.strip_prefix("*.") {
            (RuleType::Wildcard, stripped)
        } else {
            (RuleType::Normal, rule)
        };

        let labels: Vec<&str> = clean_rule.split('.').collect();
        let mut curr_idx = 0;

        for &label in labels.iter().rev() {
            let next_idx = self.nodes.len();
            let entry = self.nodes[curr_idx]
                .children
                .entry(label.to_ascii_lowercase())
                .or_insert(next_idx);

            if *entry == next_idx {
                self.nodes.push(TrieNode {
                    children: FxHashMap::default(),
                    rule: None,
                });
                curr_idx = next_idx;
            } else {
                curr_idx = *entry;
            }
        }

        self.nodes[curr_idx].rule = Some((rule_type, category));
    }

    /// Determina a quantidade de labels que compõem o sufixo público.
    pub fn find_public_suffix_labels(&self, host_labels: &[&str]) -> Option<PslMatch> {
        let mut curr_idx = 0;
        let mut longest_match: Option<PslMatch> = None;

        for (i, &label) in host_labels.iter().rev().enumerate() {
            let label_lower = label.to_ascii_lowercase();
            if let Some(&next_idx) = self.nodes[curr_idx].children.get(&label_lower) {
                curr_idx = next_idx;
                if let Some((rule_type, category)) = self.nodes[curr_idx].rule {
                    match rule_type {
                        RuleType::Exception => {
                            return Some(PslMatch {
                                rule_type,
                                category,
                                suffix_labels: i,
                            });
                        }
                        RuleType::Wildcard => {
                            longest_match = Some(PslMatch {
                                rule_type,
                                category,
                                suffix_labels: i + 2,
                            });
                        }
                        RuleType::Normal => {
                            longest_match = Some(PslMatch {
                                rule_type,
                                category,
                                suffix_labels: i + 1,
                            });
                        }
                    }
                }
            } else {
                break;
            }
        }

        longest_match
    }

    /// Extrai o domínio registrável eTLD+1 da string de hostname sem alocações adicionais.
    pub fn get_etld_plus_one<'a>(&self, hostname: &'a str) -> Option<&'a str> {
        let hostname = hostname.trim_end_matches('.');
        if hostname.is_empty() || hostname.starts_with('.') {
            return None;
        }

        let labels: Vec<&str> = hostname.split('.').collect();
        if labels.len() <= 1 {
            return None;
        }

        let suffix_len = if let Some(m) = self.find_public_suffix_labels(&labels) {
            m.suffix_labels
        } else {
            1
        };

        if labels.len() <= suffix_len {
            return None;
        }

        let etld_plus_one_label_count = suffix_len + 1;
        let start_label_idx = labels.len() - etld_plus_one_label_count;

        let mut char_offset = 0;
        for (i, label) in labels.iter().enumerate() {
            if i == start_label_idx {
                return Some(&hostname[char_offset..]);
            }
            char_offset += label.len() + 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psl_etld_plus_one() {
        let trie = CompactPslTrie::new_with_standard_rules();

        assert_eq!(trie.get_etld_plus_one("sub.example.com"), Some("example.com"));
        assert_eq!(
            trie.get_etld_plus_one("portal.service.gov.br"),
            Some("service.gov.br")
        );
        assert_eq!(trie.get_etld_plus_one("com"), None);
        assert_eq!(trie.get_etld_plus_one("gov.br"), None);
    }
}
