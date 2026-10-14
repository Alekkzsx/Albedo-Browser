# Project: ace_layout Extreme Excellence (Phase 7 v2)

## Architecture
O subsistema `ace_layout` implementa a arquitetura de árvore dupla de ponta (estilo Chromium LayoutNG, Gecko Stylo e WebKit LFC):
- **BoxTree mutável:** Mapeia a `RenderTree` do `ace_style` para uma hierarquia estrutural de `LayoutBox`.
- **FragmentTree imutável (`Arc<LayoutFragment>`):** Produto imutável e thread-safe do layout com coordenadas físicas e lógicas exatas base-60 (`LayoutUnit`).
- **Contextos de Formatação Isolados:** BFC (`MarginStrut` linear $O(1)$ + `ExclusionSpace` $O(F)$), IFC (UAX #14 + BiDi UAX #9 + baselines), FFC (Flexbox L1 two-pass), GFC (CSS Grid L1/L2 com Subgrid), TFC (Tabelas com distribuição min/max + colapso de bordas §17.6 + colspan/rowspan 2D), MCFC (Multicol com balanceamento binário).
- **Geometria & Posicionamento:** Coordenadas de fluxo normal, Absolute, Fixed, Sticky com clamp em scrollport, Stacking Contexts de 7 camadas canônicas (CSS 2.2 App E) e 2D affine transforms.
- **Invalidação & Reflow:** Bitset atômico de dirty flags com poda de subárvore sub-15µs e alocação zero no hot loop via `ace_core::arena::BumpArena`.
- **Garantia Central:** 100% Safe Rust com `#![forbid(unsafe_code)]`, zero warnings no Clippy, e determinismo matemático rigoroso.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | `Exclusion` Struct & Interval | Modelagem 2D de caixas flutuantes com `LogicalRect` e `FloatType::Left/Right` | M2.1 | Survey F1 |
| 2 | `ExclusionSpace` Shelf Search | Algoritmo `find_layout_opportunity()` em tempo $O(F)$ calculando prateleiras horizontais livres | M2.1 | Survey F1 |
| 3 | BFC Float Placement & Clearance | Resolução de `clear: left/right/both` com truncamento de `MarginStrut` e inserção de clearance | M2.1 | Survey F1 |
| 4 | IFC Dynamic Available Inline Size | Ajuste dinâmico de largura disponível por linha no `LineBreaker` em função de exclusões ativas | M2.1 | Survey F1 |
| 5 | Inline Float Promotion | Elevação de float no fluxo inline para o topo da linha corrente restringindo linhas subsequentes | M2.1 | Survey F1 |
| 6 | Sticky Positioning Clamping | `StickyConstraintPositioner` com detecção de scrollport e clamp na borda inferior do containing block | M2.1 | Survey F4 |
| 7 | 2D Affine Transforms | `Transform2D<f32>` (translate, rotate, scale, shear) e expansão do transformed visual overflow | M2.1 | Survey F4 |
| 8 | Suite 9 Float Wrapping Test | `stress_exclusion_space_float_wrapping.rs` (1.000 floats intercalados com 20.000 palavras em IFC, SLA < 100ms) | M2.1 | Survey F1/Tests |
| 9 | Suite 12 Sticky Scroll Test | `stress_sticky_positioning_scroll_matrix.rs` (5.000 nós sticky em 100 scrollports, SLA < 40ms) | M2.1 | Survey F4/Tests |
| 10 | CSS Subgrid Track Adoption | Detecção de `grid-template-columns/rows: subgrid` e herança da fatia de trilhas do pai | M2.2 | Survey F3 |
| 11 | Subgrid Bidirectional Sizing | Contribuição de dimensões intrínsecas de itens do subgrid para o pool de trilhas do grid pai | M2.2 | Survey F3 |
| 12 | Multi-Track Spanning (§11.5) | Distribuição ponderada de contribuições de tamanho para itens ocupando múltiplas trilhas | M2.2 | Survey F3 |
| 13 | Grid Named Areas & Alignments | Mapeamento matricial de `grid-template-areas` e alinhamentos expandidos (`space-evenly`) | M2.2 | Survey F3 |
| 14 | Collapsed Borders Conflict Matrix | Matriz de Conflito §17.6.2.1 (prioridade em 4 dimensões: hidden, largura, estilo, elemento, direção) | M2.2 | Survey F6 |
| 15 | Half-Border Offset Calculation | Deslocamento de meia-borda com divisão conservativa em `LayoutUnit` eliminando gaps de 1px | M2.2 | Survey F6 |
| 16 | 2D Colspan & Rowspan Occupancy | Matriz 2D de ocupação de células com particionamento de larguras mínimas | M2.2 | Survey F6 |
| 17 | Table Caption Positioning | Posicionamento de `caption-side: top/bottom` fora da grade participando do BFC da tabela | M2.2 | Survey F6 |
| 18 | Suite 11 Subgrid Test | `stress_subgrid_bidirectional_sizing.rs` (Subgrid aninhado em 5 níveis, 1.000 itens, SLA < 50ms) | M2.2 | Survey F3/Tests |
| 19 | Suite 14 Collapsed Borders Test | `stress_table_collapsed_borders_conflict.rs` (Grade 50x50 com matriz de conflito normativa, SLA < 60ms) | M2.2 | Survey F6/Tests |
| 20 | Hierarchical BreakTokens | `BlockBreakToken` e `InlineBreakToken` para retenção de estado de fragmentação | M2.3 | Survey F2 |
| 21 | Multicol Container Geometry | `MulticolFormattingContext` calculando colunas, larguras e gaps (`column-count/width/gap`) | M2.3 | Survey F2 |
| 22 | Column Balancing Binary Search | Algoritmo de balanceamento de altura de colunas via busca binária logarítmica | M2.3 | Survey F2 |
| 23 | Column Break Avoidance & Forces | Suporte a `break-inside: avoid`, `break-before: column` e `break-after: column` | M2.3 | Survey F2 |
| 24 | Column Physical Fragment Emission | Emissão de fragmentos de colunas posicionados lado a lado com espaçamento de gap | M2.3 | Survey F2 |
| 25 | Unicode BiDi UAX #9 Complete | Algoritmo BiDi completo com classes direcionais e níveis de embebimento 0–125 | M2.3 | Survey F5 |
| 26 | BiDi Rule L2 & L4 Resolution | Reordenação visual de fragmentos de texto (Regra L2) e espelhamento de glifos RTL (Regra L4) | M2.3 | Survey F5 |
| 27 | Baseline Metrics Synthesis | Síntese de baselines alfabética e central para alinhamento em Flexbox e Grid (`align-items: baseline`) | M2.3 | Survey F5 |
| 28 | Replaced Element Sizing | Resolução de `aspect-ratio: w / h` e dimensões naturais para `<img>`, `<video>`, `<canvas>` | M2.3 | Survey F5 |
| 29 | Object-Fit & Object-Position | Resolução de `object-fit: fill/contain/cover/none/scale-down` e alinhamento geométrico | M2.3 | Survey F5 |
| 30 | Suite 10 Multicol Balancing Test | `stress_multicolumn_column_balancing.rs` (50.000 palavras em 3-12 colunas balanceadas, SLA < 80ms) | M2.3 | Survey F2/Tests |
| 31 | Suite 13 BiDi UAX#9 Levels Test | `stress_unicode_bidi_uax9_levels.rs` (20.000 sentenças mistas com 15 níveis BiDi) | M2.3 | Survey F5/Tests |
| 32 | Atomic Bitset Dirty Flags | Flags atômicas `SelfNeedsLayout`, `ChildNeedsLayout`, `DescendantNeedsLayout` em `LayoutBox` | M2.4 | Survey F7 |
| 33 | Surgical Subtree Reflow | Algoritmo de reflow podando subárvores idênticas em $O(1)$ atingindo SLA sub-15µs | M2.4 | Survey F7 |
| 34 | BumpArena Zero-Allocation | Buffer transitório via `ace_core::arena::BumpArena` com reset $O(1)$ eliminando alocações de heap | M2.4 | Survey F7 |
| 35 | Reverse 7-Layer Hit-Testing | `LayoutEngine::hit_test(Point) -> Option<NodeId>` via travessia inversa de Stacking Contexts (App E) | M2.4 | Survey F4/F7 |
| 36 | `get_bounding_client_rect` API | Coordenadas absolutas físicas de qualquer nó após projeção e pixel snapping | M2.4 | Survey F4/F7 |
| 37 | BrowserPipeline Integration | `BrowserPipeline::apply_mutation()` disparando reflow cirúrgico e expondo `hit_test()` | M2.4 | Survey F7/Pipeline |
| 38 | Suite 15 Hit-Testing Test | `stress_hit_test_stacking_contexts.rs` (10.000 nós com transforms 2D e z-order, SLA < 10µs/query) | M2.4 | Survey F4/Tests |
| 39 | Suite 16 Surgical Reflow Test | `stress_dirty_flags_surgical_reflow.rs` (20.000 nós com 10.000 mutações, SLA < 15µs/mutação) | M2.4 | Survey F7/Tests |
| 40 | Workspace 16-Suite Homologation | Execução das 16 suítes de estresse atingindo $> 1.350$ testes aprovados sem regressões | M2.5 | Master Plan M2.5 |
| 41 | Strict Memory Safety & Lints | Verificação formal de `#![forbid(unsafe_code)]` e 0 warnings no Clippy workspace-wide | M2.5 | Master Plan M2.5 |
| 42 | Architecture Docs Sync | Atualização e sincronização de `docs/architecture/ace_layout.md` com as novas capacidades | M2.5 | Master Plan M2.5 |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M2.1 | Floats & Exclusões (F1) + Sticky & Transforms (F4) | `exclusion.rs`, `bfc.rs`, `line_breaker.rs`, `ifc.rs`, `sticky.rs`, `transform.rs`, Suítes 9 & 12 | M0 | PLANNED |
| M2.2 | CSS Subgrid (F3) + Tabelas Colapsadas & Spans (F6) | `subgrid.rs`, `placement.rs`, `track.rs`, `collapsed_borders.rs`, `spanning_cells.rs`, Suítes 11 & 14 | M2.1 | PLANNED |
| M2.3 | Multicol & BreakTokens (F2) + Unicode BiDi & Replaced (F5) | `break_token.rs`, `multicol/`, `bidi.rs`, `replaced.rs`, Suítes 10 & 13 | M2.2 | PLANNED |
| M2.4 | Dirty Flags & BumpArena (F7) + Engine & Pipeline Integration | `dirty_flags.rs`, `hit_test.rs`, `LayoutEngine`, `BrowserPipeline`, Suítes 15 & 16 | M2.3 | PLANNED |
| M2.5 | Homologação Global das 16 Suítes + Auditoria Final de Vitória | Validação workspace (>1.350 testes), Clippy, `#![forbid(unsafe_code)]`, Docs sync | M2.4 | PLANNED |

## Interface Contracts

### BFC ↔ ExclusionSpace
- `ExclusionSpace::new() -> Self`
- `ExclusionSpace::add_exclusion(&mut self, exclusion: Exclusion)`
- `ExclusionSpace::find_layout_opportunity(&self, search_offset: LayoutUnit, available_inline_size: LayoutUnit, min_inline_size: LayoutUnit) -> LayoutOpportunity`
- `ExclusionSpace::clear_offset(&self, clear: ClearType) -> LayoutUnit`

### IFC ↔ ExclusionSpace
- `LineBreaker::break_lines(&self, text: &str, style: &ComputedStyle, exclusion_space: &ExclusionSpace, current_block_offset: LayoutUnit) -> Vec<LineBoxFragment>`

### Positioning ↔ Scrollport & Containing Block
- `StickyConstraintPositioner::compute_sticky_offset(natural_rect: LogicalRect, insets: &StickyInsets, scrollport: &Scrollport, containing_block: &LogicalRect) -> Vector2D<LayoutUnit>`
- `Transform2D<f32>::project_rect(&self, rect: Rect<LayoutUnit>) -> Rect<LayoutUnit>`

### Grid ↔ Subgrid
- `SubgridTrackSlice::from_parent(parent_tracks: &[Track], range: Range<usize>) -> Self`
- `SubgridItemContribution::collect_intrinsic_sizes(&self) -> MinMaxSizes`

### Table ↔ Collapsed Borders
- `CollapsedBorderResolver::resolve_shared_border(border_a: &BorderSide, border_b: &BorderSide, origin_a: ElementOrigin, origin_b: ElementOrigin) -> CollapsedBorderResult`
- `CollapsedBorderResult::half_offsets(&self) -> (LayoutUnit, LayoutUnit)`

### Multicol ↔ Fragmentation
- `MulticolFormattingContext::balance_columns(children: &[LayoutBox], column_count: usize, column_gap: LayoutUnit) -> (LayoutUnit, Vec<BoxFragment>)`
- `BlockBreakToken::new(child_index: usize, consumed_block_size: LayoutUnit) -> Self`

### Inline ↔ Unicode BiDi
- `BidiResolver::resolve_visual_runs(text: &str, paragraph_direction: Direction) -> Vec<VisualRun>`

### Engine ↔ Dirty Flags & BumpArena
- `LayoutBox::mark_dirty(&self, flag: DirtyFlag)`
- `LayoutEngine::reflow_dirty_subtrees(&mut self, root: &LayoutBox, arena: &BumpArena) -> FragmentTree`
- `LayoutEngine::hit_test(&self, point: Point2D<LayoutUnit>) -> Option<NodeId>`
- `BrowserPipeline::hit_test(&self, point: Point2D<LayoutUnit>) -> Option<NodeId>`

## Code Layout
```
Albedo_Core_Engine/ace_layout/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tree/
│   │   ├── layout_box.rs
│   │   ├── constraint_space.rs
│   │   ├── min_max.rs
│   │   └── replaced.rs                [Novo - M2.3]
│   ├── block/
│   │   ├── bfc.rs
│   │   ├── margin_strut.rs
│   │   └── exclusion.rs               [Novo - M2.1]
│   ├── inline/
│   │   ├── ifc.rs
│   │   ├── line_breaker.rs
│   │   ├── line_box.rs
│   │   └── bidi.rs                    [Novo - M2.3]
│   ├── flex/
│   │   ├── ffc.rs
│   │   ├── algorithm.rs
│   │   └── freeze_loop.rs
│   ├── grid/
│   │   ├── gfc.rs
│   │   ├── track.rs
│   │   ├── placement.rs
│   │   └── subgrid.rs                 [Novo - M2.2]
│   ├── table/
│   │   ├── tfc.rs
│   │   ├── distribution.rs
│   │   ├── collapsed_borders.rs       [Novo - M2.2]
│   │   └── spanning_cells.rs          [Novo - M2.2]
│   ├── multicol/                      [Novo - M2.3]
│   │   ├── mod.rs
│   │   └── balance.rs
│   ├── positioning/
│   │   ├── absolute.rs
│   │   ├── fixed.rs
│   │   ├── stacking.rs
│   │   ├── sticky.rs                  [Novo - M2.1]
│   │   └── transform.rs               [Novo - M2.1]
│   ├── fragment/
│   │   ├── layout_fragment.rs
│   │   ├── fragment_tree.rs
│   │   └── break_token.rs             [Novo - M2.3]
│   └── engine/
│       ├── layout_engine.rs
│       ├── cache.rs
│       ├── hit_test.rs                [Novo - M2.4]
│       └── dirty_flags.rs             [Novo - M2.4]
└── tests/
    ├── common/mod.rs
    ├── stress_margin_strut_deep_hierarchy.rs       (Suíte 1)
    ├── stress_inline_formatting_multilingual.rs    (Suíte 2)
    ├── stress_flexbox_complex_matrix.rs            (Suíte 3)
    ├── stress_grid_track_sizing_fuzz.rs            (Suíte 4)
    ├── stress_table_column_distribution.rs         (Suíte 5)
    ├── stress_incremental_layout_reactive.rs       (Suíte 6)
    ├── stress_subpixel_snapping_accuracy.rs        (Suíte 7)
    ├── stress_full_page_realworld_layout.rs        (Suíte 8)
    ├── stress_exclusion_space_float_wrapping.rs    [Novo - Suíte 9 - M2.1]
    ├── stress_multicolumn_column_balancing.rs     [Novo - Suíte 10 - M2.3]
    ├── stress_subgrid_bidirectional_sizing.rs     [Novo - Suíte 11 - M2.2]
    ├── stress_sticky_positioning_scroll_matrix.rs  [Novo - Suíte 12 - M2.1]
    ├── stress_unicode_bidi_uax9_levels.rs         [Novo - Suíte 13 - M2.3]
    ├── stress_table_collapsed_borders_conflict.rs  [Novo - Suíte 14 - M2.2]
    ├── stress_hit_test_stacking_contexts.rs       [Novo - Suíte 15 - M2.4]
    └── stress_dirty_flags_surgical_reflow.rs      [Novo - Suíte 16 - M2.4]
```
