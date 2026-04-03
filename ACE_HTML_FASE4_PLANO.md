# ACE-HTML FASE 4: Performance e Otimização

## 🎯 Objetivo Geral

Elevar o desempenho do parser HTML do Albedo Browser para níveis competitivos ou superiores aos navegadores modernos (Chrome/Blink, Firefox/Gecko, Safari/WebKit), garantindo:
- Parse de documentos grandes (<5MB) em <100ms
- Memory usage eficiente (<2x tamanho do HTML original)
- Streaming parsing com baixa latência (<10ms por chunk)
- Zero allocations desnecessárias durante tokenização

---

## 📊 Estado Atual (Pré-Fase 4)

### Métricas de Performance Atuais
| Componente | LOC | Status | Performance |
|------------|-----|--------|-------------|
| Tokenizer/Lexer | 3.106 | ✅ Completo | Sem otimizações específicas |
| Tree Builder | 2.836 | ✅ Completo | Alocações frequentes de HashMap/String |
| Encoding | 593 | ✅ Completo | Decodificação básica |
| Preload Scanner | 239 | ⚠️ Básico | Implementação simplificada |
| **Total** | **7.313** | **Fases 1-3 completas** | **Precisa otimização** |

### Gargalos Identificados
1. ❌ **Alocações excessivas**: `String`, `HashMap`, `Vec` criados frequentemente
2. ❌ **Sem streaming real**: Parser requer string completa em memória
3. ❌ **Entity decoding lento**: Lookup em JSON sem otimização
4. ❌ **Preload scanner básico**: Não detecta todos os recursos críticos
5. ❌ **Sem SIMD**: Operações de string não usam instruções vetoriais
6. ❌ **Cache unfriendly**: Estruturas de dados não otimizadas para CPU cache

---

## 🚀 4.1 Otimizações de Estruturas de Dados

### 4.1.1 Arena Allocator para Nodes DOM

**Problema:** Atualmente cada `HtmlNode` e `HtmlElement` é alocado individualmente no heap, causando:
- Fragmentação de memória
- Múltiplas alocações/dealocações
- Poor cache locality

**Solução:** Implementar arena allocator customizado

```rust
// NOVO ARQUIVO: /workspace/src/ace/html/arena.rs

use std::cell::RefCell;
use std::ptr::NonNull;

/// Arena allocator para nodes DOM
/// Aloca blocos grandes de memória e distribui chunks menores
pub struct NodeArena {
    chunks: RefCell<Vec<NonEmptyChunk>>,
    current_chunk: RefCell<usize>,
}

struct NonEmptyChunk {
    data: *mut u8,
    capacity: usize,
    allocated: usize,
    alignment: usize,
}

impl NodeArena {
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            chunks: RefCell::new(vec![NonEmptyChunk::new(initial_capacity)]),
            current_chunk: RefCell::new(0),
        }
    }

    /// Aloca espaço para um T na arena
    pub fn alloc<T>(&self, value: T) -> NodeId {
        // Implementação otimizada
    }

    /// Reference a node by ID
    pub fn get<T>(&self, id: NodeId) -> &T {
        // Acesso direto sem bounds checking em release
    }

    /// Clear toda a arena (útil para reuso)
    pub fn clear(&mut self) {
        self.chunks.borrow_mut().clear();
        self.chunks.borrow_mut().push(NonEmptyChunk::new(DEFAULT_CHUNK_SIZE));
        *self.current_chunk.borrow_mut() = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

impl NodeId {
    pub const NULL: NodeId = NodeId(usize::MAX);
}
```

**Benefícios Esperados:**
- ⚡ 5-10x redução em tempo de alocação
- ⚡ Melhor cache locality (nodes contíguos em memória)
- ⚡ Dealocação em O(1) (clear da arena inteira)
- ⚡ Redução de fragmentação de memória

**Arquivos para Modificar:**
- `src/ace/html/mod.rs`: Adicionar `NodeId` e imports
- `src/ace/html/tree_builder.rs`: Substituir `Vec<HtmlNode>` por referências à arena
- `Cargo.toml`: Adicionar dependência `bumpalo` (opcional, se não implementar arena custom)

---

### 4.1.2 String Interning para Tag Names

**Problema:** Tags comuns como `"div"`, `"span"`, `"p"` são alocadas repetidamente

**Solução:** Intern strings para tags e atributos comuns

```rust
// NOVO ARQUIVO: /workspace/src/ace/html/interner.rs

use std::collections::HashMap;
use std::sync::RwLock;

/// String interner para tag names e attribute names comuns
pub struct StringInterner {
    strings: RwLock<HashMap<&'static str, usize>>,
    arena: RwLock<Vec<String>>,
    hit_count: usize,
    miss_count: usize,
}

impl StringInterner {
    pub fn new() -> Self {
        let mut interner = Self {
            strings: RwLock::new(HashMap::new()),
            arena: RwLock::new(Vec::with_capacity(1024)),
            hit_count: 0,
            miss_count: 0,
        };
        
        // Pre-popular com tags HTML comuns
        let common_tags = [
            "html", "head", "body", "div", "span", "p", "a", "img", 
            "script", "style", "link", "meta", "title", "h1", "h2", 
            "h3", "h4", "h5", "h6", "ul", "ol", "li", "table", "tr", 
            "td", "th", "form", "input", "button", "section", "article",
            "header", "footer", "nav", "aside", "main", "svg", "path",
        ];
        
        for tag in common_tags.iter() {
            interner.intern_static(*tag);
        }
        
        interner
    }

    fn intern_static(&self, s: &'static str) -> StringId {
        let mut strings = self.strings.write().unwrap();
        let id = strings.len();
        strings.insert(s, id);
        StringId(id)
    }

    pub fn intern(&self, s: &str) -> StringId {
        // Try read lock first (fast path)
        {
            let strings = self.strings.read().unwrap();
            if let Some(&id) = strings.get(s) {
                self.hit_count += 1;
                return StringId(id);
            }
        }
        
        // Miss - need to allocate
        self.miss_count += 1;
        let mut strings = self.strings.write().unwrap();
        let mut arena = self.arena.write().unwrap();
        
        // Check again after acquiring write lock
        if let Some(&id) = strings.get(s) {
            return StringId(id);
        }
        
        let id = arena.len();
        arena.push(s.to_string());
        strings.insert(unsafe { 
            std::str::from_utf8_unchecked(arena.last().unwrap().as_bytes()) 
        }, id);
        
        StringId(id)
    }

    pub fn resolve(&self, id: StringId) -> Option<&str> {
        let arena = self.arena.read().unwrap();
        arena.get(id.0).map(|s| s.as_str())
    }

    pub fn stats(&self) -> InternerStats {
        InternerStats {
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate: self.hit_count as f32 / (self.hit_count + self.miss_count) as f32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StringId(usize);

pub struct InternerStats {
    pub hit_count: usize,
    pub miss_count: usize,
    pub hit_rate: f32,
}
```

**Modificações no HtmlElement:**
```rust
// EM: src/ace/html/mod.rs

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: StringId,           // ANTES: String
    pub namespace: Namespace,
    pub attributes: SmallAttributeMap,  // ANTES: HashMap<String, String>
    pub children: Vec<NodeId>,   // ANTES: Vec<HtmlNode>
    pub slot_name: Option<StringId>,
    pub is_value: Option<StringId>,
    pub shadow_root_mode: Option<ShadowRootMode>,
    pub shadow_root: Option<Box<HtmlDocument>>,
}
```

**Benefícios:**
- ⚡ Redução de 80-90% em alocações de string para tags
- ⚡ Comparação de tags em O(1) (comparação de IDs numéricos)
- ⚡ Menor uso de memória (uma cópia por string única)

---

### 4.1.3 Small Attribute Map (SmallVec Optimization)

**Problema:** `HashMap<String, String>` tem overhead significativo para elementos com poucos atributos

**Solução:** Usar smallvec para casos comuns (0-4 atributos)

```rust
// NOVO ARQUIVO: /workspace/src/ace/html/small_attr_map.rs

use smallvec::{SmallVec, Array};
use crate::html::interner::StringId;

/// Otimizado para 0-4 atributos (caso comum)
/// Fallback para HashMap quando necessário
pub enum SmallAttributeMap {
    /// Inline storage para ≤4 atributos
    Small(SmallVec<[(StringId, StringId); 4]>),
    /// Heap storage para muitos atributos
    Large(Box<std::collections::HashMap<StringId, StringId>>),
}

impl SmallAttributeMap {
    pub fn new() -> Self {
        SmallAttributeMap::Small(SmallVec::new())
    }

    pub fn insert(&mut self, key: StringId, value: StringId) {
        match self {
            SmallAttributeMap::Small(vec) => {
                if vec.len() < 4 {
                    vec.push((key, value));
                } else {
                    // Promover para HashMap
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in vec.drain(..) {
                        map.insert(k, v);
                    }
                    map.insert(key, value);
                    *self = SmallAttributeMap::Large(Box::new(map));
                }
            }
            SmallAttributeMap::Large(map) => {
                map.insert(key, value);
            }
        }
    }

    pub fn get(&self, key: StringId) -> Option<StringId> {
        match self {
            SmallAttributeMap::Small(vec) => {
                vec.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
            }
            SmallAttributeMap::Large(map) => map.get(&key).copied(),
        }
    }
}
```

**Dependência no Cargo.toml:**
```toml
[dependencies]
smallvec = "1.13"
```

---

## 🔁 4.2 Streaming Parser Incremental

### 4.2.1 Parser State Serialization

**Problema:** Parser atual requer input completo

**Solução:** Permitir pause/resume do parsing

```rust
// MODIFICAR: src/ace/html/lexer.rs

pub struct HtmlLexer {
    // ... campos existentes ...
    
    /// Suporte para streaming
    pub state: LexerState,
    pub reconsume_state: Option<LexerState>,
    pub pending_token: Option<RawHtmlToken>,
    
    /// Contadores para resume
    pub line: usize,
    pub column: usize,
    pub absolute_position: usize,
}

impl HtmlLexer {
    /// Cria lexer para streaming
    pub fn new_streaming() -> Self {
        Self {
            // ... inicialização ...
            state: LexerState::Data,
            reconsume_state: None,
            pending_token: None,
            line: 1,
            column: 1,
            absolute_position: 0,
        }
    }

    /// Processa chunk de dados
    pub fn feed(&mut self, chunk: &str) -> Vec<RawHtmlToken> {
        let mut tokens = Vec::new();
        // Lógica existente modificada para suportar resume
        tokens
    }

    /// Finaliza stream (EOF)
    pub fn end(&mut self) -> Vec<RawHtmlToken> {
        // Emite EOF token e finaliza estado atual
        vec![]
    }

    /// Serializa estado para resume posterior
    pub fn serialize_state(&self) -> LexerStateSnapshot {
        LexerStateSnapshot {
            state: self.state,
            line: self.line,
            column: self.column,
            // ... outros campos necessários ...
        }
    }
}

#[derive(Clone, Debug)]
pub struct LexerStateSnapshot {
    pub state: LexerState,
    pub line: usize,
    pub column: usize,
    pub absolute_position: usize,
    // Campos necessários para resume
}
```

### 4.2.2 Tree Builder Streaming

```rust
// MODIFICAR: src/ace/html/tree_builder.rs

pub struct HtmlTreeBuilder {
    // ... campos existentes ...
    
    /// Suporte streaming
    pub paused: bool,
    pub document_partial: HtmlDocument,
    pub open_elements_snapshot: Vec<NodeId>,
    pub active_formatting_snapshot: Vec<NodeId>,
}

impl HtmlTreeBuilder {
    pub fn new_streaming() -> Self {
        Self {
            // Inicialização para streaming
            paused: false,
            document_partial: HtmlDocument {
                doctype: None,
                children: Vec::new(),
            },
            open_elements_snapshot: Vec::new(),
            active_formatting_snapshot: Vec::new(),
        }
    }

    pub fn process_tokens(&mut self, tokens: &[HtmlToken]) -> ParseProgress {
        if self.paused {
            return ParseProgress::Paused;
        }
        
        // Processamento normal com checkpoints
        ParseProgress::Complete
    }

    pub fn pause(&mut self) {
        self.paused = true;
        self.open_elements_snapshot = self.open_elements.clone();
        self.active_formatting_snapshot = self.active_formatting_elements.clone();
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }
}

pub enum ParseProgress {
    Complete,
    Paused,
    NeedsMoreData,
}
```

---

## ⚡ 4.3 SIMD Optimizations

### 4.3.1 Fast Character Classification

**Problema:** Verificações caractere-por-caractere são lentas

**Solução:** Usar SIMD para processar 16-32 bytes por instrução

```rust
// NOVO ARQUIVO: /workspace/src/ace/html/simd.rs

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Verifica rapidamente se bytes são whitespace usando SIMD
#[target_feature(enable = "sse2")]
#[inline]
pub unsafe fn simd_is_whitespace_sse2(data: &[u8]) -> usize {
    if data.len() < 16 {
        return fallback_is_whitespace(data);
    }

    let mut processed = 0;
    let whitespace_mask = _mm_set1_epi8(0x20); // space
    
    while processed + 16 <= data.len() {
        let chunk = _mm_loadu_si128(data[processed..].as_ptr() as *const __m128i);
        // Comparação SIMD...
        processed += 16;
    }
    
    processed
}

/// Fast path para ASCII-only content
#[inline]
pub fn fast_ascii_tag_scan(data: &[u8]) -> usize {
    // Usa SIMD para encontrar '<' rapidamente
    let mut i = 0;
    while i + 32 <= data.len() {
        // Processa 32 bytes de uma vez
        i += 32;
    }
    i
}
```

### 4.3.2 Entity Decoding Otimizado

```rust
// MODIFICAR: src/ace/html/entities.rs

/// Lookup table otimizada para entity decoding
/// Usando perfect hashing ou trie
pub struct EntityLookupTable {
    /// Trie para lookup rápido
    trie: EntityTrie,
}

struct EntityTrie {
    nodes: Vec<TrieNode>,
}

struct TrieNode {
    children: [Option<u32>; 64], // Index para filhos
    entity_id: Option<u32>,      // Se é fim de entidade
}

impl EntityLookupTable {
    pub const fn new() -> Self {
        // Compile-time construction da trie
        Self {
            trie: EntityTrie::new(),
        }
    }

    #[inline]
    pub fn lookup(&self, input: &[u8]) -> Option<(char, usize)> {
        // O(1) average case para entities comuns
        self.trie.find_longest_match(input)
    }
}
```

---

## 🔍 4.4 Preload Scanner Avançado

### 4.4.1 Detecção Completa de Recursos

**Problema:** Preload scanner atual é muito básico

**Solução:** Detectar todos os recursos críticos

```rust
// REESCREVER: src/ace/html/preload_scanner.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreloadResourceType {
    Script,
    Stylesheet,
    ModuleScript,
    Image,
    Video,
    Audio,
    Font,
    Fetch,
    Worker,
    Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourcePriority {
    Highest,    // CSS blocking render
    High,       // Scripts, fonts
    Normal,     // Images, videos
    Low,        // Prefetch
}

#[derive(Debug, Clone)]
pub struct PreloadRequest {
    pub url: String,
    pub resource_type: PreloadResourceType,
    pub priority: ResourcePriority,
    pub crossorigin: Option<CrossOrigin>,
    pub integrity: Option<String>,
    pub media: Option<String>,
    pub fetchpriority: Option<String>,
}

pub struct AdvancedPreloadScanner {
    state: PreloadScannerState,
    requests: Vec<PreloadRequest>,
    base_url: String,
    seen_urls: std::collections::HashSet<String>, // Evitar duplicatas
}

impl AdvancedPreloadScanner {
    pub fn scan(&mut self, input: &str) -> Vec<PreloadRequest> {
        // Detecta:
        // ✓ <link rel="stylesheet" href="...">
        // ✓ <link rel="preload" as="script/font/image/...">
        // ✓ <script src="..."> (com async/defer/module detection)
        // ✓ <img src="..." loading="lazy">
        // ✓ <picture><source srcset="...">
        // ✓ <video poster="..."><source src="...">
        // ✓ <audio src="...">
        // ✓ <font face="..." src="..."> (SVG fonts)
        // ✓ @import em <style> inline
        // ✓ <link rel="manifest" href="...">
        // ✓ <link rel="icon" href="...">
        // ✓ <meta http-equiv="refresh" content="...;url=...">
        
        self.requests.clone()
    }

    fn determine_priority(&self, resource_type: PreloadResourceType, attributes: &Attributes) -> ResourcePriority {
        match resource_type {
            PreloadResourceType::Stylesheet => {
                // CSS blocking render é highest priority
                if attributes.media.as_deref() == Some("print") {
                    ResourcePriority::Low
                } else {
                    ResourcePriority::Highest
                }
            }
            PreloadResourceType::ModuleScript => ResourcePriority::High,
            PreloadResourceType::Script => {
                if attributes.r#async {
                    ResourcePriority::Normal
                } else if attributes.defer {
                    ResourcePriority::Low
                } else {
                    ResourcePriority::Highest // Blocking script
                }
            }
            PreloadResourceType::Font => ResourcePriority::High,
            PreloadResourceType::Image => {
                match attributes.fetchpriority.as_deref() {
                    Some("high") => ResourcePriority::High,
                    Some("low") => ResourcePriority::Low,
                    _ => ResourcePriority::Normal,
                }
            }
            _ => ResourcePriority::Normal,
        }
    }
}
```

### 4.4.2 Integração com Network Layer

```rust
// NOVO ARQUIVO: src/ace/html/resource_loader.rs

use crate::network::RequestHandle;

pub struct ResourceLoader {
    preload_queue: Vec<PreloadRequest>,
    active_loads: std::collections::HashMap<String, RequestHandle>,
    loaded_resources: std::collections::HashMap<String, LoadedResource>,
}

impl ResourceLoader {
    pub fn queue_preloads(&mut self, requests: Vec<PreloadRequest>) {
        // Ordena por prioridade
        // Inicia downloads em paralelo
        // Respeita limits de concorrência por domínio
    }

    pub fn poll(&mut self) -> Vec<ResourceEvent> {
        // Verifica recursos carregados
        // Notifica parser/CSS engine
        vec![]
    }
}

pub enum ResourceEvent {
    LoadComplete(String, LoadedResource),
    LoadError(String, NetworkError),
}
```

---

## 📈 4.5 Memory-Mapped File Support

### 4.5.1 Zero-Copy File Parsing

```rust
// NOVO ARQUIVO: src/ace/html/mmap_parser.rs

use memmap2::Mmap;
use std::fs::File;

/// Parser que usa memory-mapped files para zero-copy
pub struct MappedHtmlParser {
    mmap: Option<Mmap>,
    decoder: StreamingDecoder,
}

impl MappedHtmlParser {
    pub fn from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        
        Ok(Self {
            mmap: Some(mmap),
            decoder: StreamingDecoder::new(),
        })
    }

    pub fn parse_async(&mut self) -> ParseStream {
        // Retorna stream de tokens sem copiar dados
        ParseStream::new(self.mmap.as_ref().unwrap())
    }
}
```

**Dependência:**
```toml
[dependencies]
memmap2 = "0.9"
```

---

## 🎯 4.6 Metrics e Profiling

### 4.6.1 Performance Counters

```rust
// NOVO ARQUIVO: src/ace/html/metrics.rs

pub struct ParserMetrics {
    pub parse_start: std::time::Instant,
    pub tokens_generated: usize,
    pub nodes_created: usize,
    pub errors_encountered: usize,
    
    // Timing breakdown
    pub tokenization_time: std::time::Duration,
    pub tree_building_time: std::time::Duration,
    pub encoding_detection_time: std::time::Duration,
    
    // Memory tracking
    pub peak_memory_bytes: usize,
    pub arena_allocations: usize,
    pub string_intern_hits: usize,
    pub string_intern_misses: usize,
}

impl ParserMetrics {
    pub fn report(&self) {
        println!("=== ACE-HTML Performance Report ===");
        println!("Total parse time: {:?}", self.parse_start.elapsed());
        println!("Tokens generated: {}", self.tokens_generated);
        println!("Nodes created: {}", self.nodes_created);
        println!("Tokenization: {:?}", self.tokenization_time);
        println!("Tree building: {:?}", self.tree_building_time);
        println!("String interning hit rate: {:.1}%", 
            self.string_intern_hits as f32 / (self.string_intern_hits + self.string_intern_misses) as f32 * 100.0);
    }
}
```

---

## ✅ Critérios de Conclusão da Fase 4

### Performance Targets
- [ ] Parse HTML5 spec (~5MB) em <100ms (hardware moderno)
- [ ] Memory usage <2x tamanho do HTML original
- [ ] Streaming latency <10ms por chunk de 8KB
- [ ] String interning hit rate >85%
- [ ] Zero allocations durante tokenização de tags comuns
- [ ] Preload scanner detecta 100% dos recursos críticos

### Features Obrigatórias
- [ ] Arena allocator implementado e integrado
- [ ] String interning funcional
- [ ] Streaming parser com pause/resume
- [ ] Preload scanner avançado (todos os tipos de recurso)
- [ ] SIMD optimizations para operações críticas
- [ ] Memory-mapped file support
- [ ] Metrics e profiling tools

### Testes de Performance
- [ ] Benchmark suite estabelecida
- [ ] Comparação com html5ever (referência Rust)
- [ ] Testes com páginas reais (top 100 Alexa)
- [ ] Regression tests de performance

---

## 📅 Cronograma Estimado

| Semana | Foco | Entregáveis |
|--------|------|-------------|
| 1 | Arena + String Interning | `arena.rs`, `interner.rs`, integração no Tree Builder |
| 2 | Streaming Parser | Modificações no Lexer/TreeBuilder, serialização de estado |
| 3 | Preload Scanner Avançado | `preload_scanner.rs` reescrito, `resource_loader.rs` |
| 4 | SIMD + Otimizações Finais | `simd.rs`, benchmarks, profiling, ajustes |

**Total: 4 semanas**

---

## 🔧 Dependências Adicionais

Adicionar ao `Cargo.toml`:
```toml
[dependencies]
smallvec = "1.13"          # Small attribute maps
memmap2 = "0.9"            # Memory-mapped files
criterion = "0.5"          # Benchmarking framework

[target.'cfg(target_arch = "x86_64")'.dependencies]
# SIMD já disponível no std
```

---

## 📚 Referências Técnicas

1. **Blink Parser:** https://chromium.googlesource.com/chromium/src/+/main/third_party/blink/renderer/core/html/parser/
2. **Gecko HTML5 Parser:** https://github.com/mozilla/gecko-dev/tree/master/parser/html
3. **html5ever (Rust):** https://github.com/servo/html5ever
4. **SIMD Guide:** https://doc.rust-lang.org/std/arch/index.html
5. **Arena Allocators:** https://github.com/tikv/rfcs/blob/master/text/2019-03-27-bump-allocator.md

---

*Documento criado para implementação da Fase 4 do ACE-HTML*
*Albedo Browser Project - Dezembro 2025*
