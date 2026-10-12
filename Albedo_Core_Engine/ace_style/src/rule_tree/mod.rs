//! # Árvore de Regras e Indexação em Baldes (`ace_style::rule_tree`)
//!
//! Particionamento de regras e casamento de seletores acelerado por Counting Bloom Filter.

pub mod bucket;
pub mod matcher;

pub use bucket::{RuleBucketMap, RuleRef};
pub use matcher::MatchedRule;
