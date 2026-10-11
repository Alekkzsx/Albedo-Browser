//! # Motor de Estilo e RenderTree (`ace_style::engine`)
//!
//! Coordenação de folhas de estilo, cascata, render tree, cache MPC e invalidação reativa cirúrgica.

pub mod cache;
pub mod invalidation;
pub mod mpc;
pub mod style_engine;

pub use cache::{StyleSharingCache, StyleSharingKey};
pub use invalidation::{
    DOMMutation, InvalidationResult, InvalidationSet, RuleInvalidationSets, StyleDirtyFlags,
};
pub use mpc::{compute_rules_signature, MatchedPropertiesCache, MpcKey};
pub use style_engine::{RenderNodeKind, RenderTree, RenderTreeNode, StyleEngine};
