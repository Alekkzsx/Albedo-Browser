//! # Estilo Computado & Resolução em Cascata (CSS Cascading & Inheritance Level 4)
//!
//! Algoritmo de resolução de estilos combinando regras de folhas de estilo e estilos em linha,
//! resolução de variáveis customizadas (`var()`), herança automática W3C e avaliação de `@media` queries.
//!
//! ## Performance
//! O `StyleResolver` suporta resolução com cache (`StyleCache`) que evita recomputar o estilo do
//! elemento pai recursivamente a cada chamada, reduzindo a complexidade de O(n²) para O(n) em
//! árvores profundas. Use `StyleResolver::resolve_with_cache` para resolver múltiplos elementos
//! de uma só vez, ou `StyleResolver::resolve_element_style_with_context` para resolução avulsa.

use crate::cssom::declaration::CSSProperty;
use crate::cssom::media::{evaluate_media_query, MediaContext};
use crate::cssom::stylesheet::{CSSRule, CSSStyleSheet};
use crate::query::Specificity;
use crate::tree::Document;
use ace_core::id::NodeId;
use ace_core::intern::Atom;
use rustc_hash::{FxHashMap, FxHashSet};
use smol_str::SmolStr;

/// Cache de estilos computados por `NodeId`. Reutilize entre múltiplas chamadas de resolução
/// para evitar recomputação do estilo do elemento pai (O(n²) → O(n)).
pub type StyleCache = FxHashMap<NodeId, ComputedStyle>;

/// Representação do conjunto de estilos computados para um elemento DOM.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComputedStyle {
    properties: FxHashMap<SmolStr, SmolStr>,
    custom_properties: FxHashMap<Atom, SmolStr>,
}

impl ComputedStyle {
    pub fn new() -> Self {
        Self {
            properties: FxHashMap::default(),
            custom_properties: FxHashMap::default(),
        }
    }

    /// Retorna o valor computado de uma propriedade CSS.
    /// Para custom properties (`--*`), a busca é estritamente case-sensitive.
    /// Para propriedades padrão, a busca é case-insensitive.
    #[inline]
    pub fn get_property_value(&self, name: &str) -> Option<&str> {
        if name.starts_with("--") {
            let atom = Atom::new(name);
            self.custom_properties.get(&atom).map(|s| s.as_str())
        } else {
            self.properties
                .get(name.to_ascii_lowercase().as_str())
                .map(|s| s.as_str())
        }
    }

    /// Retorna o valor de uma propriedade customizada CSS (`--*`).
    #[inline]
    pub fn get_custom_property(&self, name: &str) -> Option<&str> {
        let atom = Atom::new(name);
        self.custom_properties.get(&atom).map(|s| s.as_str())
    }

    /// Define diretamente uma propriedade computada.
    pub fn set_property(&mut self, name: impl Into<SmolStr>, value: impl Into<SmolStr>) {
        let n = name.into();
        let v = value.into();
        if n.starts_with("--") {
            let atom = Atom::new(n.as_str());
            self.custom_properties.insert(atom, v.clone());
            self.properties.insert(n, v);
        } else {
            self.properties.insert(SmolStr::new(n.to_ascii_lowercase()), v);
        }
    }

    /// Define uma propriedade customizada (`--*`) de forma case-sensitive.
    pub fn set_custom_property(&mut self, name: impl Into<Atom>, value: impl Into<SmolStr>) {
        let a = name.into();
        let v = value.into();
        self.properties.insert(SmolStr::new(a.as_str()), v.clone());
        self.custom_properties.insert(a, v);
    }

    /// Retorna uma referência imutável ao mapa de propriedades padrão.
    #[inline]
    pub fn properties(&self) -> &FxHashMap<SmolStr, SmolStr> {
        &self.properties
    }

    /// Retorna uma referência imutável ao mapa de propriedades customizadas (`--*`).
    #[inline]
    pub fn custom_properties(&self) -> &FxHashMap<Atom, SmolStr> {
        &self.custom_properties
    }
}

/// Verifica se uma propriedade CSS é naturalmente herdável conforme W3C CSS Cascading Level 4 §6.
pub fn is_inheritable_property(name: &str) -> bool {
    if name.starts_with("--") {
        return true;
    }
    matches!(
        name.to_ascii_lowercase().as_str(),
        "color"
            | "font-family"
            | "font-size"
            | "font-weight"
            | "font-style"
            | "font-variant"
            | "font"
            | "line-height"
            | "visibility"
            | "letter-spacing"
            | "word-spacing"
            | "text-align"
            | "text-align-last"
            | "text-transform"
            | "text-indent"
            | "text-shadow"
            | "white-space"
            | "direction"
            | "cursor"
            | "list-style"
            | "list-style-type"
            | "list-style-position"
            | "list-style-image"
            | "quotes"
            | "border-collapse"
            | "border-spacing"
            | "caption-side"
            | "empty-cells"
            | "tab-size"
            | "hyphens"
    )
}

/// Retorna o valor inicial normativo de uma propriedade CSS conforme especificações W3C.
pub fn initial_value_for_property(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "color" => "black",
        "font-family" => "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        "font-size" => "16px",
        "font-weight" => "normal",
        "font-style" => "normal",
        "line-height" => "normal",
        "visibility" => "visible",
        "letter-spacing" => "normal",
        "word-spacing" => "normal",
        "text-align" => "start",
        "text-transform" => "none",
        "text-indent" => "0px",
        "white-space" => "normal",
        "direction" => "ltr",
        "cursor" => "auto",
        "list-style" | "list-style-type" => "disc",
        "list-style-position" => "outside",
        "list-style-image" => "none",
        "quotes" => "auto",
        "border-collapse" => "separate",
        "border-spacing" => "0px",
        "caption-side" => "top",
        "empty-cells" => "show",
        "display" => "inline",
        "position" => "static",
        "top" | "right" | "bottom" | "left" => "auto",
        "z-index" => "auto",
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => "0px",
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => "0px",
        "border" | "border-width" | "border-style" | "border-color" => "medium none currentcolor",
        "background" | "background-color" => "transparent",
        "background-image" => "none",
        "width" | "height" => "auto",
        "min-width" | "min-height" => "auto",
        "max-width" | "max-height" => "none",
        "opacity" => "1",
        "overflow" | "overflow-x" | "overflow-y" => "visible",
        "box-sizing" => "content-box",
        "flex" => "0 1 auto",
        "flex-direction" => "row",
        "align-items" => "stretch",
        "justify-content" => "flex-start",
        "transform" => "none",
        _ => "",
    }
}

#[derive(Clone)]
struct MatchedProperty<'a> {
    property: &'a CSSProperty,
    specificity: Specificity,
    order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PropStatus {
    Visiting,
    Valid(SmolStr),
    Invalid,
}

/// Resolvedor de cascata de estilos para a árvore DOM conforme W3C CSS Cascading Level 4.
pub struct StyleResolver;

impl StyleResolver {
    /// Computa o estilo final para um elemento com o `MediaContext` padrão.
    pub fn resolve_element_style(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
    ) -> ComputedStyle {
        Self::resolve_element_style_with_context(doc, element_id, sheets, &MediaContext::default())
    }

    /// Computa o estilo final para um elemento. Versão sem cache — adequada para chamadas avulsas.
    /// Para resolver estilos de múltiplos elementos de uma mesma árvore de forma eficiente,
    /// prefira `resolve_with_cache` que reutiliza o cache entre chamadas (O(n²) → O(n)).
    pub fn resolve_element_style_with_context(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
        media_ctx: &MediaContext,
    ) -> ComputedStyle {
        let mut cache = StyleCache::default();
        Self::resolve_with_cache(doc, element_id, sheets, media_ctx, &mut cache)
    }

    /// Computa o estilo final com memoização dos estilos dos ancestrais.
    ///
    /// ## Complexidade
    /// - **Sem cache compartilhado:** O(depth²) — cada elemento pai é resolvido do zero.
    /// - **Com cache compartilhado:** O(n × rules) — cada nó é resolvido no máximo uma vez.
    ///
    /// Passe o mesmo `cache: &mut StyleCache` para todos os elementos do documento a fim de
    /// obter desempenho O(n) total.
    pub fn resolve_with_cache(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
        media_ctx: &MediaContext,
        cache: &mut StyleCache,
    ) -> ComputedStyle {
        if let Some(cached) = cache.get(&element_id) {
            return cached.clone();
        }

        let result = Self::resolve_inner(doc, element_id, sheets, media_ctx, cache);
        cache.insert(element_id, result.clone());
        result
    }

    /// Implementação interna da cascata. Separada para evitar borrow duplo no cache.
    fn resolve_inner(
        doc: &Document,
        element_id: NodeId,
        sheets: &[CSSStyleSheet],
        media_ctx: &MediaContext,
        cache: &mut StyleCache,
    ) -> ComputedStyle {
        let mut computed = ComputedStyle::new();

        // 1. Resolve o estilo do elemento pai (para herança automática e variáveis)
        // Usa o cache para evitar O(n²): cada pai é resolvido no máximo uma vez.
        let parent_element_id = find_parent_element(doc, element_id);
        let parent_style = parent_element_id.map(|pid| {
            Self::resolve_with_cache(doc, pid, sheets, media_ctx, cache)
        });

        // 2. Coleta declarações casadas das folhas de estilo e de estilos inline
        let mut normal_decls: Vec<MatchedProperty> = Vec::new();
        let mut important_decls: Vec<MatchedProperty> = Vec::new();
        let mut order_counter = 0;

        for sheet in sheets {
            collect_matched_rules(
                doc,
                element_id,
                sheet,
                media_ctx,
                &mut normal_decls,
                &mut important_decls,
                &mut order_counter,
            );
        }

        // Ordena por especificidade e ordem de aparição na folha
        normal_decls.sort_by(|a, b| {
            a.specificity
                .cmp(&b.specificity)
                .then_with(|| a.order.cmp(&b.order))
        });
        important_decls.sort_by(|a, b| {
            a.specificity
                .cmp(&b.specificity)
                .then_with(|| a.order.cmp(&b.order))
        });

        // 3. Coleta declarações de propriedades customizadas (`--*`) no elemento
        let mut declared_custom_props: Vec<(Atom, SmolStr)> = Vec::new();

        // 3.1 Normal stylesheet rules
        for decl in &normal_decls {
            if decl.property.name.as_str().starts_with("--") {
                declared_custom_props.push((decl.property.name.clone(), decl.property.value.clone()));
            }
        }

        // 3.2 Normal inline styles
        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if !prop.important && prop.name.as_str().starts_with("--") {
                                declared_custom_props.push((prop.name.clone(), prop.value.clone()));
                            }
                        }
                    }
                }
            }
        }

        // 3.3 Important stylesheet rules
        for decl in &important_decls {
            if decl.property.name.as_str().starts_with("--") {
                declared_custom_props.push((decl.property.name.clone(), decl.property.value.clone()));
            }
        }

        // 3.4 Important inline styles
        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if prop.important && prop.name.as_str().starts_with("--") {
                                declared_custom_props.push((prop.name.clone(), prop.value.clone()));
                            }
                        }
                    }
                }
            }
        }

        // 4. Resolve o grafo de variáveis CSS com detecção de ciclo DFS
        let inherited_custom_props = parent_style
            .as_ref()
            .map(|p| p.custom_properties())
            .cloned()
            .unwrap_or_default();

        let (resolved_custom_props, cycle_props) =
            resolve_all_custom_properties(&inherited_custom_props, &declared_custom_props);

        // 5. Aplica propriedades padrão na ordem de precedência da cascata:
        //    (1) Normal stylesheet -> (2) Normal inline -> (3) Important stylesheet -> (4) Important inline
        let mut ordered_standard_decls: Vec<(&Atom, &SmolStr)> = Vec::new();

        for decl in &normal_decls {
            if !decl.property.name.as_str().starts_with("--") {
                ordered_standard_decls.push((&decl.property.name, &decl.property.value));
            }
        }

        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if !prop.important && !prop.name.as_str().starts_with("--") {
                                ordered_standard_decls.push((&prop.name, &prop.value));
                            }
                        }
                    }
                }
            }
        }

        for decl in &important_decls {
            if !decl.property.name.as_str().starts_with("--") {
                ordered_standard_decls.push((&decl.property.name, &decl.property.value));
            }
        }

        if let Some(node) = doc.get_node(element_id) {
            if let Some(el) = node.as_element() {
                if let Some(ref rare) = el.rare_data {
                    if let Some(ref inline_decl) = rare.inline_style_decl {
                        for prop in inline_decl.properties.as_slice() {
                            if prop.important && !prop.name.as_str().starts_with("--") {
                                ordered_standard_decls.push((&prop.name, &prop.value));
                            }
                        }
                    }
                }
            }
        }

        // Processa cada declaração padrão substituindo var() e avaliando palavras-chave
        for (name_atom, raw_val) in ordered_standard_decls {
            let prop_name = name_atom.as_str();
            let substituted =
                resolve_var_in_standard_property(raw_val.as_str(), &resolved_custom_props, &cycle_props);

            if let Some(val_str) = substituted {
                let trimmed_val = val_str.trim();
                let lower_val = trimmed_val.to_ascii_lowercase();

                match lower_val.as_str() {
                    "inherit" => {
                        apply_keyword_inherit(&mut computed, prop_name, parent_style.as_ref());
                    }
                    "initial" => {
                        computed.set_property(prop_name, initial_value_for_property(prop_name));
                    }
                    "unset" | "revert" => {
                        if is_inheritable_property(prop_name) {
                            apply_keyword_inherit(&mut computed, prop_name, parent_style.as_ref());
                        } else {
                            computed.set_property(prop_name, initial_value_for_property(prop_name));
                        }
                    }
                    _ => {
                        computed.set_property(prop_name, trimmed_val);
                    }
                }
            } else {
                // IACVT (Invalid At Computed-Value Time) per W3C CSS Variables §3.3 -> age como 'unset'
                if is_inheritable_property(prop_name) {
                    apply_keyword_inherit(&mut computed, prop_name, parent_style.as_ref());
                } else {
                    computed.set_property(prop_name, initial_value_for_property(prop_name));
                }
            }
        }

        // 6. Herança automática W3C CSS Cascading Level 4 §6:
        //    Propriedades herdáveis não declaradas no filho herdam o valor computado do pai
        if let Some(ref parent) = parent_style {
            for (p_name, p_val) in parent.properties() {
                if is_inheritable_property(p_name.as_str())
                    && computed.get_property_value(p_name.as_str()).is_none()
                {
                    computed.set_property(p_name.as_str(), p_val.as_str());
                }
            }
        }

        // 7. Registra todas as propriedades customizadas resolvidas neste elemento
        for (k, v) in resolved_custom_props {
            computed.set_custom_property(k, v);
        }

        computed
    }
}

/// Aplica a palavra-chave `inherit` a uma propriedade CSS.
fn apply_keyword_inherit(
    computed: &mut ComputedStyle,
    prop_name: &str,
    parent_style: Option<&ComputedStyle>,
) {
    if let Some(parent) = parent_style {
        if let Some(pval) = parent.get_property_value(prop_name) {
            computed.set_property(prop_name, pval);
        } else {
            computed.set_property(prop_name, initial_value_for_property(prop_name));
        }
    } else {
        computed.set_property(prop_name, initial_value_for_property(prop_name));
    }
}

/// Helper para obter o ID do elemento pai na árvore DOM.
fn find_parent_element(doc: &Document, node_id: NodeId) -> Option<NodeId> {
    let mut curr = doc.get_node(node_id)?.parent;
    while let Some(pid) = curr {
        if let Some(node) = doc.get_node(pid) {
            if node.is_element() {
                return Some(pid);
            }
            curr = node.parent;
        } else {
            break;
        }
    }
    None
}

/// Coleta regras casadas de uma folha de estilo, avaliando blocos `@media`.
fn collect_matched_rules<'a>(
    doc: &Document,
    element_id: NodeId,
    sheet: &'a CSSStyleSheet,
    media_ctx: &MediaContext,
    normal_decls: &mut Vec<MatchedProperty<'a>>,
    important_decls: &mut Vec<MatchedProperty<'a>>,
    order_counter: &mut usize,
) {
    if sheet.disabled {
        return;
    }
    for rule in &sheet.rules {
        collect_from_rule(
            doc,
            element_id,
            rule,
            media_ctx,
            normal_decls,
            important_decls,
            order_counter,
        );
    }
}

fn collect_from_rule<'a>(
    doc: &Document,
    element_id: NodeId,
    rule: &'a CSSRule,
    media_ctx: &MediaContext,
    normal_decls: &mut Vec<MatchedProperty<'a>>,
    important_decls: &mut Vec<MatchedProperty<'a>>,
    order_counter: &mut usize,
) {
    match rule {
        CSSRule::Style(ref style_rule) => {
            for selector in &style_rule.selectors {
                if selector.matches(doc, element_id) {
                    let spec = selector.specificity();
                    for prop in style_rule.style.properties.as_slice() {
                        *order_counter += 1;
                        let matched = MatchedProperty {
                            property: prop,
                            specificity: spec,
                            order: *order_counter,
                        };
                        if prop.important {
                            important_decls.push(matched);
                        } else {
                            normal_decls.push(matched);
                        }
                    }
                }
            }
        }
        CSSRule::Media {
            ref condition,
            ref rules,
        } => {
            if evaluate_media_query(condition.as_str(), media_ctx) {
                for inner_rule in rules {
                    collect_from_rule(
                        doc,
                        element_id,
                        inner_rule,
                        media_ctx,
                        normal_decls,
                        important_decls,
                        order_counter,
                    );
                }
            }
        }
        // @supports — CSS Conditional Rules Level 3.
        // Atualmente avaliado como sempre verdadeiro (stub de feature detection).
        // Em um engine com motor de valores computados completo, avaliar a declaração.
        CSSRule::Supports { ref rules, .. } => {
            for inner_rule in rules {
                collect_from_rule(
                    doc,
                    element_id,
                    inner_rule,
                    media_ctx,
                    normal_decls,
                    important_decls,
                    order_counter,
                );
            }
        }
        // @layer — CSS Cascade Layers.
        // Atualmente as regras de uma camada são incluídas na cascata sem distinção de camada.
        // Uma implementação completa exigiria rastrear a ordem de declaração das camadas.
        CSSRule::Layer { ref rules, .. } => {
            for inner_rule in rules {
                collect_from_rule(
                    doc,
                    element_id,
                    inner_rule,
                    media_ctx,
                    normal_decls,
                    important_decls,
                    order_counter,
                );
            }
        }
        // @container — CSS Container Queries.
        // Atualmente avaliado como sempre verdadeiro (sem contexto de layout de container).
        // Uma implementação completa requer propagação das dimensões do container.
        CSSRule::Container { ref rules, .. } => {
            for inner_rule in rules {
                collect_from_rule(
                    doc,
                    element_id,
                    inner_rule,
                    media_ctx,
                    normal_decls,
                    important_decls,
                    order_counter,
                );
            }
        }
        CSSRule::Keyframes { .. } | CSSRule::Import { .. } => {}
    }
}

/// Resolve o grafo de dependências de variáveis customizadas (`--*`) utilizando DFS e detecção de ciclos.
fn resolve_all_custom_properties(
    inherited: &FxHashMap<Atom, SmolStr>,
    declared: &[(Atom, SmolStr)],
) -> (FxHashMap<Atom, SmolStr>, FxHashSet<Atom>) {
    let mut raw_map: FxHashMap<Atom, SmolStr> = inherited.clone();
    for (k, v) in declared {
        raw_map.insert(k.clone(), v.clone());
    }

    let mut status_map: FxHashMap<Atom, PropStatus> = FxHashMap::default();
    let mut cycle_props: FxHashSet<Atom> = FxHashSet::default();
    let mut stack: Vec<Atom> = Vec::new();

    let all_keys: Vec<Atom> = raw_map.keys().cloned().collect();
    for key in all_keys {
        resolve_custom_prop_dfs(&key, &raw_map, &mut status_map, &mut cycle_props, &mut stack);
    }

    let mut resolved = FxHashMap::default();
    for (k, status) in status_map {
        match status {
            PropStatus::Valid(val) => {
                resolved.insert(k, val);
            }
            PropStatus::Invalid | PropStatus::Visiting => {
                cycle_props.insert(k);
            }
        }
    }

    (resolved, cycle_props)
}

/// Passo recursivo DFS para resolução de uma propriedade customizada.
fn resolve_custom_prop_dfs(
    name: &Atom,
    raw_map: &FxHashMap<Atom, SmolStr>,
    status_map: &mut FxHashMap<Atom, PropStatus>,
    cycle_props: &mut FxHashSet<Atom>,
    stack: &mut Vec<Atom>,
) -> Option<SmolStr> {
    if stack.contains(name) || status_map.get(name) == Some(&PropStatus::Visiting) {
        if let Some(pos) = stack.iter().position(|x| x == name) {
            for item in &stack[pos..] {
                cycle_props.insert(item.clone());
            }
        }
        cycle_props.insert(name.clone());
        status_map.insert(name.clone(), PropStatus::Invalid);
        return None;
    }

    if let Some(status) = status_map.get(name) {
        return match status {
            PropStatus::Valid(val) => Some(val.clone()),
            PropStatus::Invalid | PropStatus::Visiting => None,
        };
    }

    let raw_val = raw_map.get(name)?.clone();

    status_map.insert(name.clone(), PropStatus::Visiting);
    stack.push(name.clone());

    let substituted = substitute_var_tokens_in_custom_prop(
        raw_val.as_str(),
        raw_map,
        status_map,
        cycle_props,
        stack,
    );

    stack.pop();

    if let Some(final_str) = substituted {
        let smol = SmolStr::new(final_str);
        status_map.insert(name.clone(), PropStatus::Valid(smol.clone()));
        Some(smol)
    } else {
        cycle_props.insert(name.clone());
        status_map.insert(name.clone(), PropStatus::Invalid);
        None
    }
}

/// Substitui referências `var()` dentro de uma propriedade customizada.
fn substitute_var_tokens_in_custom_prop(
    input: &str,
    raw_map: &FxHashMap<Atom, SmolStr>,
    status_map: &mut FxHashMap<Atom, PropStatus>,
    cycle_props: &mut FxHashSet<Atom>,
    stack: &mut Vec<Atom>,
) -> Option<String> {
    if !input.contains("var(") {
        return Some(input.to_string());
    }

    let mut output = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        if i + 4 <= len && &input[i..i + 4] == "var(" {
            let close_pos = find_matching_paren(input, i + 3)?;
            let inner = &input[i + 4..close_pos];
            let (var_name, fallback) = split_var_args(inner);

            if !var_name.starts_with("--") {
                return None;
            }

            let dep_atom = Atom::new(var_name);
            if raw_map.contains_key(&dep_atom) {
                let resolved_dep = resolve_custom_prop_dfs(
                    &dep_atom,
                    raw_map,
                    status_map,
                    cycle_props,
                    stack,
                );

                let val = resolved_dep?;
                output.push_str(val.as_str());
            } else {
                let fb = fallback?;
                let resolved_fb = substitute_var_tokens_in_custom_prop(
                    fb,
                    raw_map,
                    status_map,
                    cycle_props,
                    stack,
                )?;
                output.push_str(&resolved_fb);
            }

            i = close_pos + 1;
        } else {
            let ch = input[i..].chars().next().unwrap();
            output.push(ch);
            i += ch.len_utf8();
        }
    }

    Some(output)
}

/// Substitui referências `var()` em declarações de propriedades padrão.
fn resolve_var_in_standard_property(
    input: &str,
    resolved_custom_props: &FxHashMap<Atom, SmolStr>,
    cycle_props: &FxHashSet<Atom>,
) -> Option<String> {
    if !input.contains("var(") {
        return Some(input.to_string());
    }

    let mut output = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        if i + 4 <= len && &input[i..i + 4] == "var(" {
            let close_pos = find_matching_paren(input, i + 3)?;
            let inner = &input[i + 4..close_pos];
            let (var_name, fallback) = split_var_args(inner);

            if !var_name.starts_with("--") {
                return None;
            }

            let var_atom = Atom::new(var_name);

            if cycle_props.contains(&var_atom) {
                return None;
            }

            if let Some(val) = resolved_custom_props.get(&var_atom) {
                output.push_str(val.as_str());
            } else {
                let fb = fallback?;
                let resolved_fb = resolve_var_in_standard_property(
                    fb,
                    resolved_custom_props,
                    cycle_props,
                )?;
                output.push_str(&resolved_fb);
            }

            i = close_pos + 1;
        } else {
            let ch = input[i..].chars().next().unwrap();
            output.push(ch);
            i += ch.len_utf8();
        }
    }

    Some(output)
}

/// Encontra o índice do parêntese de fechamento `)` correspondente ao aberto em `open_pos`.
fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let mut depth: usize = 1;
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<(usize, char)> = s[open_pos + 1..].char_indices().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let (byte_offset, ch) = chars[i];
        if ch == '\\' && (in_single || in_double) && i + 1 < len {
            i += 2;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + 1 + byte_offset);
                }
            }
        }
        i += 1;
    }
    None
}

/// Separa o nome da variável customizada e o fallback opcional em um `var(...)`.
fn split_var_args(inner: &str) -> (&str, Option<&str>) {
    let mut paren_depth: usize = 0;
    let mut in_single = false;
    let mut in_double = false;

    for (idx, ch) in inner.char_indices() {
        if ch == '\'' && !in_double {
            in_single = !in_single;
        } else if ch == '"' && !in_single {
            in_double = !in_double;
        } else if !in_single && !in_double {
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth = paren_depth.saturating_sub(1);
            } else if ch == ',' && paren_depth == 0 {
                let var_name = inner[..idx].trim();
                let fallback = inner[idx + 1..].trim();
                return (var_name, Some(fallback));
            }
        }
    }

    (inner.trim(), None)
}
