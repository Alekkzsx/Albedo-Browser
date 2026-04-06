# ACE-HTML Chrome-Level Design

## 1. Visão Geral da Arquitetura

### 1.1 Arquitetura Atual vs Proposta

**Arquitetura Atual (Single-threaded)**:
```
Input HTML → Lexer → Tokenizer → Tree Builder → DOM
              ↓
         (SIMD básico)
```

**Arquitetura Proposta (Multi-threaded + Speculative)**:
```
                    ┌─────────────────────────────────┐
                    │   Input HTML (Streaming)        │
                    └──────────┬──────────────────────┘
                               │
                    ┏━━━━━━━━━━┻━━━━━━━━━━┓
                    ┃   Thread Pool        ┃
                    ┃   (std::thread)      ┃
                    ┗━━━━━━━━━━┳━━━━━━━━━━┛
                               │
              ┌────────────────┼────────────────┐
              │                │                │
         [Thread 1]       [Thread 2]      [Main Thread]
              │                │                │
        Preload Scanner    Tokenizer      Tree Builder
              │                │                │
         (SIMD AVX-512)   (SIMD AVX-512)   (Arena Alloc)
              │                │                │
              ↓                ↓                ↓
         Preload Queue    Token Channel    DOM Output
```

### 1.2 Componentes Principais

1. **Lexer Otimizado** (src/ace/html/lexer.rs)
   - SIMD AVX-512 para whitespace/entities
   - Zero-copy string slicing
   - State machine otimizada

2. **Tokenizer Especulativo** (src/ace/html/tokenizer.rs)
   - Roda em thread separada
   - Produz tokens via channel
   - Backpressure handling

3. **Tree Builder Incremental** (src/ace/html/tree_builder.rs)
   - Consome tokens assincronamente
   - Arena allocator para nodes
   - String interner para tag names

4. **Preload Scanner** (src/ace/html/preload_scanner.rs)
   - Extração especulativa de recursos
   - Roda em paralelo com tokenizer
   - Zero allocations

5. **Benchmark Framework** (src/ace/html/bench/mod.rs)
   - Statistical analysis próprio
   - Comparison engine
   - HTML report generation

---

## 2. Design Detalhado por Componente


### 2.1 Lexer Otimizado com SIMD AVX-512

#### 2.1.1 Estrutura de Dados
```rust
// src/ace/html/lexer.rs
pub struct HtmlLexer {
    // Input buffer (zero-copy slices)
    input: &'static str,
    pos: usize,
    
    // State machine
    state: LexerState,
    return_state: LexerState,
    
    // SIMD optimization level
    simd_level: SimdLevel,
    
    // Character reference buffer
    char_ref_code: u32,
    temp_buffer: SmallVec<[u8; 32]>, // Stack-allocated
    
    // Position tracking
    line: u32,
    column: u32,
    
    // Token output
    pending_tokens: VecDeque<RawToken>,
}

#[derive(Clone, Copy)]
enum SimdLevel {
    Scalar,
    Sse2,
    Avx2,
    Avx512,
}
```

#### 2.1.2 SIMD Whitespace Detection (AVX-512)
```rust
#[target_feature(enable = "avx512f")]
unsafe fn simd_is_whitespace_avx512(data: &[u8]) -> u64 {
    debug_assert!(data.len() >= 64);
    
    // Load 64 bytes into SIMD register
    let chunk = _mm512_loadu_si512(data.as_ptr() as *const __m512i);
    
    // Whitespace: 0x09, 0x0A, 0x0C, 0x0D, 0x20
    let tab = _mm512_set1_epi8(0x09);
    let lf = _mm512_set1_epi8(0x0A);
    let ff = _mm512_set1_epi8(0x0C);
    let cr = _mm512_set1_epi8(0x0D);
    let space = _mm512_set1_epi8(0x20);
    
    // Compare with each whitespace character
    let mask_tab = _mm512_cmpeq_epi8_mask(chunk, tab);
    let mask_lf = _mm512_cmpeq_epi8_mask(chunk, lf);
    let mask_ff = _mm512_cmpeq_epi8_mask(chunk, ff);
    let mask_cr = _mm512_cmpeq_epi8_mask(chunk, cr);
    let mask_space = _mm512_cmpeq_epi8_mask(chunk, space);
    
    // Combine with OR
    mask_tab | mask_lf | mask_ff | mask_cr | mask_space
}
```

#### 2.1.3 SIMD Entity Lookup (Perfect Hashing)
```rust
// Perfect hash table para entidades comuns (top 100)
const ENTITY_HASH_TABLE: [Option<(u32, char)>; 128] = [
    // Gerado em build time via build.rs
    Some((hash("nbsp"), '\u{00A0}')),
    Some((hash("lt"), '<')),
    Some((hash("gt"), '>')),
    // ... 97 mais
];

#[inline]
pub fn fast_entity_lookup(entity: &str) -> Option<char> {
    let hash = compute_hash(entity);
    let idx = (hash & 0x7F) as usize;
    
    if let Some((stored_hash, ch)) = ENTITY_HASH_TABLE[idx] {
        if stored_hash == hash {
            return Some(ch);
        }
    }
    
    // Fallback para hash map completo (2231 entidades)
    FULL_ENTITY_MAP.get(entity).copied()
}
```


### 2.2 Tokenizer Especulativo (Multi-threaded)

#### 2.2.1 Thread Pool Próprio
```rust
// src/ace/html/thread_pool.rs
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        
        ThreadPool { workers, sender }
    }
    
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}
```

#### 2.2.2 Speculative Tokenizer
```rust
// src/ace/html/speculative.rs
pub struct SpeculativeTokenizer {
    lexer: HtmlLexer,
    token_sender: Sender<Token>,
    thread_pool: ThreadPool,
}

impl SpeculativeTokenizer {
    pub fn new(input: &'static str) -> (Self, Receiver<Token>) {
        let (tx, rx) = std::sync::mpsc::sync_channel(1024); // Bounded channel
        
        let tokenizer = Self {
            lexer: HtmlLexer::new(input),
            token_sender: tx,
            thread_pool: ThreadPool::new(1), // Single tokenizer thread
        };
        
        (tokenizer, rx)
    }
    
    pub fn start(&mut self) {
        let lexer = std::mem::replace(&mut self.lexer, HtmlLexer::empty());
        let sender = self.token_sender.clone();
        
        self.thread_pool.execute(move || {
            let mut lexer = lexer;
            loop {
                match lexer.next_token() {
                    Some(token) => {
                        if sender.send(token).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    None => break,
                }
            }
        });
    }
}
```

#### 2.2.3 Backpressure Handling
```rust
// Tree builder consome tokens com backpressure
pub struct TreeBuilder {
    token_receiver: Receiver<Token>,
    // ... outros campos
}

impl TreeBuilder {
    pub fn build_incremental(&mut self) -> Result<(), Error> {
        loop {
            // Timeout para evitar blocking infinito
            match self.token_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(token) => self.process_token(token)?,
                Err(RecvTimeoutError::Timeout) => {
                    // Yield para outras tasks
                    std::thread::yield_now();
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }
}
```


### 2.3 Arena Allocator Otimizado

#### 2.3.1 Design do Arena
```rust
// src/ace/html/arena.rs
pub struct NodeArena {
    chunks: Vec<Chunk>,
    current_chunk: usize,
    current_offset: usize,
}

struct Chunk {
    data: Box<[u8]>,
    capacity: usize,
    allocated: usize,
}

const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

impl NodeArena {
    pub fn new() -> Self {
        Self {
            chunks: vec![Chunk::new(CHUNK_SIZE)],
            current_chunk: 0,
            current_offset: 0,
        }
    }
    
    pub fn alloc<T>(&mut self, value: T) -> &mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        
        // Align offset
        let offset = (self.current_offset + align - 1) & !(align - 1);
        
        // Check if current chunk has space
        if offset + size > self.chunks[self.current_chunk].capacity {
            self.allocate_new_chunk();
            return self.alloc(value);
        }
        
        // Write value
        let ptr = unsafe {
            let chunk = &mut self.chunks[self.current_chunk];
            let ptr = chunk.data.as_mut_ptr().add(offset) as *mut T;
            ptr.write(value);
            ptr
        };
        
        self.current_offset = offset + size;
        unsafe { &mut *ptr }
    }
    
    fn allocate_new_chunk(&mut self) {
        self.chunks.push(Chunk::new(CHUNK_SIZE));
        self.current_chunk = self.chunks.len() - 1;
        self.current_offset = 0;
    }
}
```

#### 2.3.2 String Interner com Perfect Hashing
```rust
// src/ace/html/interner.rs
pub struct StringInterner {
    // Perfect hash para tag names comuns (top 100)
    perfect_hash: [Option<StringId>; 128],
    
    // Hash map para strings arbitrárias
    map: HashMap<&'static str, StringId>,
    
    // Arena para strings
    arena: Vec<u8>,
    
    // Estatísticas
    hits: usize,
    misses: usize,
}

impl StringInterner {
    pub fn intern(&mut self, s: &str) -> StringId {
        // Try perfect hash first
        let hash = compute_hash(s);
        let idx = (hash & 0x7F) as usize;
        
        if let Some(id) = self.perfect_hash[idx] {
            if self.get(id) == s {
                self.hits += 1;
                return id;
            }
        }
        
        // Try hash map
        if let Some(&id) = self.map.get(s) {
            self.hits += 1;
            return id;
        }
        
        // Allocate new string in arena
        self.misses += 1;
        let id = self.allocate_string(s);
        self.map.insert(self.get(id), id);
        id
    }
    
    fn allocate_string(&mut self, s: &str) -> StringId {
        let start = self.arena.len();
        self.arena.extend_from_slice(s.as_bytes());
        StringId { start, len: s.len() }
    }
}
```


### 2.4 Preload Scanner Robusto

#### 2.4.1 Scanner State Machine
```rust
// src/ace/html/preload_scanner.rs
pub struct PreloadScanner {
    state: ScannerState,
    current_tag: Option<String>,
    current_attrs: SmallVec<[(String, String); 4]>,
    base_url: Option<String>,
}

#[derive(Clone, Copy)]
enum ScannerState {
    Data,
    TagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    BeforeAttributeValue,
    AttributeValue,
}

impl PreloadScanner {
    pub fn scan(&mut self, html: &str) -> Vec<PreloadRequest> {
        let mut requests = Vec::new();
        let mut pos = 0;
        
        while pos < html.len() {
            // SIMD-accelerated tag scanning
            if let Some(tag_start) = self.find_next_tag_simd(&html[pos..]) {
                pos += tag_start;
                
                if let Some(req) = self.extract_preload(&html[pos..]) {
                    requests.push(req);
                }
            } else {
                break;
            }
        }
        
        requests
    }
    
    #[target_feature(enable = "avx2")]
    unsafe fn find_next_tag_simd(&self, data: &str) -> Option<usize> {
        let bytes = data.as_bytes();
        let mut pos = 0;
        
        while pos + 32 <= bytes.len() {
            let chunk = _mm256_loadu_si256(bytes[pos..].as_ptr() as *const __m256i);
            let lt = _mm256_set1_epi8(b'<' as i8);
            let mask = _mm256_cmpeq_epi8(chunk, lt);
            let bits = _mm256_movemask_epi8(mask) as u32;
            
            if bits != 0 {
                return Some(pos + bits.trailing_zeros() as usize);
            }
            
            pos += 32;
        }
        
        // Scalar fallback
        bytes[pos..].iter().position(|&b| b == b'<').map(|i| pos + i)
    }
    
    fn extract_preload(&mut self, html: &str) -> Option<PreloadRequest> {
        // Fast path: check if tag is preloadable
        if !self.is_preloadable_tag(html) {
            return None;
        }
        
        // Extract attributes
        let (tag, attrs) = self.parse_tag_fast(html)?;
        
        match tag.as_str() {
            "link" => self.extract_link_preload(&attrs),
            "script" => self.extract_script_preload(&attrs),
            "img" => self.extract_img_preload(&attrs),
            _ => None,
        }
    }
}
```

#### 2.4.2 Parallel Preload Scanning
```rust
pub struct ParallelPreloadScanner {
    thread_pool: ThreadPool,
}

impl ParallelPreloadScanner {
    pub fn scan_parallel(&self, html: &str) -> Vec<PreloadRequest> {
        // Split HTML into chunks
        let chunk_size = html.len() / 4; // 4 threads
        let chunks: Vec<_> = html
            .as_bytes()
            .chunks(chunk_size)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect();
        
        // Scan each chunk in parallel
        let (tx, rx) = std::sync::mpsc::channel();
        
        for chunk in chunks {
            let tx = tx.clone();
            self.thread_pool.execute(move || {
                let mut scanner = PreloadScanner::new();
                let requests = scanner.scan(chunk);
                tx.send(requests).unwrap();
            });
        }
        
        drop(tx); // Close channel
        
        // Collect results
        let mut all_requests = Vec::new();
        for requests in rx {
            all_requests.extend(requests);
        }
        
        // Deduplicate
        all_requests.sort_by(|a, b| a.url.cmp(&b.url));
        all_requests.dedup_by(|a, b| a.url == b.url);
        
        all_requests
    }
}
```


### 2.5 Benchmark Framework Próprio

#### 2.5.1 Statistical Analysis
```rust
// src/ace/html/bench/stats.rs
pub struct BenchmarkStats {
    samples: Vec<Duration>,
    mean: Duration,
    median: Duration,
    std_dev: Duration,
    p95: Duration,
    p99: Duration,
    outliers: usize,
}

impl BenchmarkStats {
    pub fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        
        let mean = Self::calculate_mean(&samples);
        let median = samples[samples.len() / 2];
        let std_dev = Self::calculate_std_dev(&samples, mean);
        let p95 = samples[(samples.len() as f64 * 0.95) as usize];
        let p99 = samples[(samples.len() as f64 * 0.99) as usize];
        let outliers = Self::count_outliers(&samples, mean, std_dev);
        
        Self {
            samples,
            mean,
            median,
            std_dev,
            p95,
            p99,
            outliers,
        }
    }
    
    fn calculate_mean(samples: &[Duration]) -> Duration {
        let sum: Duration = samples.iter().sum();
        sum / samples.len() as u32
    }
    
    fn calculate_std_dev(samples: &[Duration], mean: Duration) -> Duration {
        let variance: f64 = samples
            .iter()
            .map(|&d| {
                let diff = d.as_secs_f64() - mean.as_secs_f64();
                diff * diff
            })
            .sum::<f64>()
            / samples.len() as f64;
        
        Duration::from_secs_f64(variance.sqrt())
    }
    
    fn count_outliers(samples: &[Duration], mean: Duration, std_dev: Duration) -> usize {
        let threshold = mean.as_secs_f64() + 3.0 * std_dev.as_secs_f64();
        samples
            .iter()
            .filter(|&&d| d.as_secs_f64() > threshold)
            .count()
    }
}
```

#### 2.5.2 Benchmark Runner
```rust
// src/ace/html/bench/runner.rs
pub struct BenchmarkRunner {
    warmup_iterations: usize,
    measurement_iterations: usize,
    baseline: Option<BenchmarkStats>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            warmup_iterations: 10,
            measurement_iterations: 100,
            baseline: None,
        }
    }
    
    pub fn bench<F>(&mut self, name: &str, mut f: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        // Warmup
        for _ in 0..self.warmup_iterations {
            f();
        }
        
        // Measurement
        let mut samples = Vec::with_capacity(self.measurement_iterations);
        for _ in 0..self.measurement_iterations {
            let start = Instant::now();
            f();
            let elapsed = start.elapsed();
            samples.push(elapsed);
        }
        
        let stats = BenchmarkStats::from_samples(samples);
        
        // Compare with baseline
        let comparison = self.baseline.as_ref().map(|baseline| {
            let speedup = baseline.mean.as_secs_f64() / stats.mean.as_secs_f64();
            Comparison { speedup }
        });
        
        BenchmarkResult {
            name: name.to_string(),
            stats,
            comparison,
        }
    }
    
    pub fn set_baseline(&mut self, stats: BenchmarkStats) {
        self.baseline = Some(stats);
    }
}
```

#### 2.5.3 HTML Report Generation
```rust
// src/ace/html/bench/report.rs
pub struct HtmlReportGenerator {
    results: Vec<BenchmarkResult>,
}

impl HtmlReportGenerator {
    pub fn generate(&self) -> String {
        let mut html = String::new();
        
        html.push_str("<!DOCTYPE html><html><head>");
        html.push_str("<title>ACE-HTML Benchmark Report</title>");
        html.push_str("<style>");
        html.push_str(include_str!("report.css"));
        html.push_str("</style></head><body>");
        
        html.push_str("<h1>ACE-HTML Benchmark Report</h1>");
        html.push_str("<table>");
        html.push_str("<tr><th>Benchmark</th><th>Mean</th><th>Median</th>");
        html.push_str("<th>P95</th><th>P99</th><th>Speedup</th></tr>");
        
        for result in &self.results {
            html.push_str("<tr>");
            html.push_str(&format!("<td>{}</td>", result.name));
            html.push_str(&format!("<td>{:.3}ms</td>", result.stats.mean.as_secs_f64() * 1000.0));
            html.push_str(&format!("<td>{:.3}ms</td>", result.stats.median.as_secs_f64() * 1000.0));
            html.push_str(&format!("<td>{:.3}ms</td>", result.stats.p95.as_secs_f64() * 1000.0));
            html.push_str(&format!("<td>{:.3}ms</td>", result.stats.p99.as_secs_f64() * 1000.0));
            
            if let Some(ref cmp) = result.comparison {
                let color = if cmp.speedup > 1.0 { "green" } else { "red" };
                html.push_str(&format!(
                    "<td style='color:{}'>{:.2}x</td>",
                    color, cmp.speedup
                ));
            } else {
                html.push_str("<td>-</td>");
            }
            
            html.push_str("</tr>");
        }
        
        html.push_str("</table></body></html>");
        html
    }
}
```


### 2.6 html5lib Test Harness Próprio

#### 2.6.1 JSON Parser Próprio
```rust
// src/ace/html/tests/json_parser.rs
pub struct JsonParser {
    input: Vec<u8>,
    pos: usize,
}

#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonParser {
    pub fn parse(input: &str) -> Result<JsonValue, String> {
        let mut parser = Self {
            input: input.as_bytes().to_vec(),
            pos: 0,
        };
        parser.parse_value()
    }
    
    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        
        match self.peek()? {
            b'n' => self.parse_null(),
            b't' | b'f' => self.parse_bool(),
            b'"' => self.parse_string(),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err("unexpected character".to_string()),
        }
    }
    
    fn parse_string(&mut self) -> Result<JsonValue, String> {
        self.expect(b'"')?;
        let mut s = String::new();
        
        loop {
            match self.next()? {
                b'"' => break,
                b'\\' => {
                    match self.next()? {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let hex = self.parse_unicode_escape()?;
                            s.push(hex);
                        }
                        _ => return Err("invalid escape".to_string()),
                    }
                }
                c => s.push(c as char),
            }
        }
        
        Ok(JsonValue::String(s))
    }
    
    // ... outros métodos de parsing
}
```

#### 2.6.2 Test Runner
```rust
// src/ace/html/tests/html5lib_harness.rs
pub struct Html5libTestRunner {
    test_dir: PathBuf,
    parallel: bool,
}

pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<TestFailure>,
}

impl Html5libTestRunner {
    pub fn run_tokenizer_tests(&self) -> TestResults {
        let test_files = self.find_test_files("tokenizer");
        let mut results = TestResults::default();
        
        for file in test_files {
            let json = std::fs::read_to_string(&file).unwrap();
            let tests = JsonParser::parse(&json).unwrap();
            
            if let JsonValue::Object(obj) = tests {
                if let Some(JsonValue::Array(tests)) = obj.get("tests") {
                    for test in tests {
                        results.total += 1;
                        
                        if self.run_single_tokenizer_test(test) {
                            results.passed += 1;
                        } else {
                            results.failed += 1;
                            results.failures.push(TestFailure {
                                file: file.clone(),
                                test: test.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        results
    }
    
    fn run_single_tokenizer_test(&self, test: &JsonValue) -> bool {
        let JsonValue::Object(test_obj) = test else {
            return false;
        };
        
        // Extract input
        let Some(JsonValue::String(input)) = test_obj.get("input") else {
            return false;
        };
        
        // Extract expected output
        let Some(JsonValue::Array(expected)) = test_obj.get("output") else {
            return false;
        };
        
        // Run tokenizer
        let mut tokenizer = HtmlTokenizer::new(input);
        let mut actual = Vec::new();
        
        while let Some(token) = tokenizer.next_token() {
            actual.push(token);
        }
        
        // Compare
        self.compare_tokens(&actual, expected)
    }
    
    fn compare_tokens(&self, actual: &[Token], expected: &JsonValue) -> bool {
        // Deep comparison logic
        // ...
        true
    }
}
```


---

## 3. Algoritmos Críticos

### 3.1 Adoption Agency Algorithm (AAA) Completo

```rust
// src/ace/html/tree_builder.rs
impl HtmlTreeBuilder {
    fn run_adoption_agency_algorithm(&mut self, tag_name: &str) {
        // Outer loop: máximo 8 iterações
        for outer_loop_counter in 0..8 {
            // Step 1: Find formatting element
            let formatting_element = match self.find_formatting_element(tag_name) {
                Some(elem) => elem,
                None => return, // No formatting element found
            };
            
            // Step 2: Check if in stack of open elements
            if !self.is_in_open_elements(formatting_element) {
                self.remove_from_active_formatting_elements(formatting_element);
                return;
            }
            
            // Step 3: Check if in scope
            if !self.is_in_scope(formatting_element) {
                // Parse error
                return;
            }
            
            // Step 4: Find furthest block
            let furthest_block = match self.find_furthest_block(formatting_element) {
                Some(block) => block,
                None => {
                    // Pop until formatting element
                    self.pop_until(formatting_element);
                    self.remove_from_active_formatting_elements(formatting_element);
                    return;
                }
            };
            
            // Step 5: Bookmark
            let bookmark = self.get_afe_index(formatting_element);
            
            // Step 6: Common ancestor
            let common_ancestor = self.get_element_before(formatting_element);
            
            // Step 7: Inner loop (máximo 3 iterações)
            let mut node = furthest_block;
            let mut last_node = furthest_block;
            
            for inner_loop_counter in 0..3 {
                // Find node in open elements
                node = self.get_element_before(node);
                
                // If not in AFE, remove from open elements
                if !self.is_in_active_formatting_elements(node) {
                    self.remove_from_open_elements(node);
                    continue;
                }
                
                // If node is formatting element, break
                if node == formatting_element {
                    break;
                }
                
                // Clone node
                let clone = self.clone_element(node);
                
                // Replace in AFE and open elements
                self.replace_in_afe(node, clone);
                self.replace_in_open_elements(node, clone);
                node = clone;
                
                // Reparent last_node
                self.remove_child(last_node);
                self.append_child(node, last_node);
                
                last_node = node;
            }
            
            // Step 8: Insert last_node
            self.insert_node_at_appropriate_place(common_ancestor, last_node);
            
            // Step 9: Create new element
            let new_element = self.create_element_for_token(formatting_element);
            
            // Step 10: Reparent children
            let children = self.take_children(furthest_block);
            for child in children {
                self.append_child(new_element, child);
            }
            
            // Step 11: Append new element
            self.append_child(furthest_block, new_element);
            
            // Step 12: Remove from AFE and insert at bookmark
            self.remove_from_active_formatting_elements(formatting_element);
            self.insert_in_afe_at_bookmark(new_element, bookmark);
            
            // Step 13: Remove from open elements and insert after furthest block
            self.remove_from_open_elements(formatting_element);
            self.insert_in_open_elements_after(furthest_block, new_element);
        }
    }
}
```

### 3.2 Foster Parenting

```rust
impl HtmlTreeBuilder {
    fn foster_parent(&mut self, node: NodeId) {
        // Find last table in open elements
        let last_table = self.find_last_table_in_open_elements();
        
        match last_table {
            Some(table) => {
                // Insert before table
                let parent = self.get_parent(table);
                if let Some(parent) = parent {
                    self.insert_before(parent, table, node);
                } else {
                    // Table has no parent (fragment case)
                    let previous = self.get_element_before_in_stack(table);
                    self.append_child(previous, node);
                }
            }
            None => {
                // No table, append to first element in stack (html)
                let html = self.open_elements[0];
                self.append_child(html, node);
            }
        }
    }
}
```


---

## 4. Otimizações de Performance

### 4.1 Memory Layout Otimizado

```rust
// Compact node representation (32 bytes)
#[repr(C)]
pub struct CompactNode {
    // 8 bytes: node type + flags
    type_and_flags: u64,
    
    // 8 bytes: parent/sibling pointers (packed)
    parent: u32,
    next_sibling: u32,
    
    // 8 bytes: children pointers
    first_child: u32,
    last_child: u32,
    
    // 8 bytes: data pointer (tag name, text, etc.)
    data: *const u8,
}

// Flags encoding
const NODE_TYPE_MASK: u64 = 0xFF;
const NODE_NAMESPACE_SHIFT: u64 = 8;
const NODE_NAMESPACE_MASK: u64 = 0xFF << NODE_NAMESPACE_SHIFT;
```

### 4.2 Cache-Friendly Data Structures

```rust
// Structure of Arrays (SoA) para melhor cache locality
pub struct NodeStorage {
    // Separate arrays para cada campo
    types: Vec<NodeType>,
    parents: Vec<u32>,
    first_children: Vec<u32>,
    last_children: Vec<u32>,
    next_siblings: Vec<u32>,
    prev_siblings: Vec<u32>,
    
    // Data arrays
    tag_names: Vec<StringId>,
    text_data: Vec<String>,
}

// Acesso via NodeId (index)
impl NodeStorage {
    pub fn get_type(&self, id: NodeId) -> NodeType {
        self.types[id.0 as usize]
    }
    
    pub fn get_parent(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.parents[id.0 as usize];
        if parent == u32::MAX {
            None
        } else {
            Some(NodeId(parent))
        }
    }
}
```

### 4.3 Prefetching

```rust
// Prefetch próximos nodes durante traversal
impl NodeStorage {
    pub fn traverse_children(&self, parent: NodeId) -> ChildIterator {
        let first_child = self.first_children[parent.0 as usize];
        
        // Prefetch first child data
        if first_child != u32::MAX {
            unsafe {
                let ptr = &self.types[first_child as usize] as *const NodeType;
                std::intrinsics::prefetch_read_data(ptr, 3);
            }
        }
        
        ChildIterator {
            storage: self,
            current: if first_child == u32::MAX {
                None
            } else {
                Some(NodeId(first_child))
            },
        }
    }
}
```

### 4.4 Branch Prediction Hints

```rust
// Usar likely/unlikely para hot paths
#[inline(always)]
fn is_whitespace(c: char) -> bool {
    // Whitespace é comum, hint para branch predictor
    likely(matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C'))
}

#[inline(always)]
fn is_ascii_alpha(c: char) -> bool {
    // ASCII alpha é muito comum em tags
    likely(c.is_ascii_alphabetic())
}

// Macro helpers
#[inline(always)]
fn likely(b: bool) -> bool {
    if b {
        std::intrinsics::likely(true)
    } else {
        false
    }
}
```


---

## 5. Estratégia de Testes

### 5.1 Test Pyramid

```
                    ┌─────────────┐
                    │   E2E Tests │  (10%)
                    │  Real pages │
                    └─────────────┘
                  ┌───────────────────┐
                  │ Integration Tests │  (20%)
                  │  html5lib-tests   │
                  └───────────────────┘
              ┌─────────────────────────────┐
              │      Unit Tests             │  (70%)
              │  Lexer, Tokenizer, Builder  │
              └─────────────────────────────┘
```

### 5.2 Unit Tests

```rust
// src/ace/html/lexer.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lexer_simple_tag() {
        let mut lexer = HtmlLexer::new("<div>");
        let token = lexer.next_token().unwrap();
        
        assert!(matches!(token.kind, HtmlTokenKind::StartTag(_)));
    }
    
    #[test]
    fn test_lexer_entity_nbsp() {
        let mut lexer = HtmlLexer::new("&nbsp;");
        let token = lexer.next_token().unwrap();
        
        if let HtmlTokenKind::Character(text) = token.kind {
            assert_eq!(text, "\u{00A0}");
        } else {
            panic!("expected character token");
        }
    }
    
    #[test]
    fn test_lexer_all_states_reachable() {
        // Verificar que todos os 80 estados são alcançáveis
        let states = collect_reachable_states();
        assert_eq!(states.len(), 80);
    }
}
```

### 5.3 Integration Tests (html5lib)

```rust
// tests/html5lib_conformance.rs
#[test]
fn test_html5lib_tokenizer_suite() {
    let runner = Html5libTestRunner::new("tests/html5lib-tests");
    let results = runner.run_tokenizer_tests();
    
    println!("Tokenizer tests: {}/{} passed", results.passed, results.total);
    
    if !results.failures.is_empty() {
        for failure in &results.failures {
            eprintln!("FAILED: {:?}", failure);
        }
    }
    
    assert_eq!(results.pass_rate(), 1.0, "Expected 100% pass rate");
}

#[test]
fn test_html5lib_tree_construction_suite() {
    let runner = Html5libTestRunner::new("tests/html5lib-tests");
    let results = runner.run_tree_construction_tests();
    
    println!("Tree construction tests: {}/{} passed", results.passed, results.total);
    assert_eq!(results.pass_rate(), 1.0, "Expected 100% pass rate");
}
```

### 5.4 Benchmark Tests

```rust
// benches/parser_benchmarks.rs
fn main() {
    let mut runner = BenchmarkRunner::new();
    
    // Micro benchmarks
    runner.bench("tokenizer_simple", || {
        let mut tokenizer = HtmlTokenizer::new("<div>text</div>");
        while tokenizer.next_token().is_some() {}
    });
    
    runner.bench("tokenizer_attributes", || {
        let mut tokenizer = HtmlTokenizer::new(r#"<div class="x" id="y">"#);
        while tokenizer.next_token().is_some() {}
    });
    
    // Macro benchmarks
    let wikipedia = load_wikipedia_homepage();
    runner.bench("parse_wikipedia", || {
        parse_document(&wikipedia);
    });
    
    // Generate report
    let report = HtmlReportGenerator::new(runner.results());
    std::fs::write("target/bench_report.html", report.generate()).unwrap();
}
```

### 5.5 Fuzzing

```rust
// fuzz/fuzz_targets/parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(html) = std::str::from_utf8(data) {
        // Should never panic
        let _ = parse_document(html);
    }
});
```


---

## 6. Plano de Implementação

### 6.1 Fase 1: Conformidade WHATWG (4-6 semanas)

#### Semana 1-2: Lexer States Completos
- [ ] Implementar todos os 80 estados do lexer
- [ ] Character references: 2.231 entidades nomeadas
- [ ] Numeric references: decimal, hex, surrogate pairs
- [ ] Error recovery: 52 parse errors
- [ ] Testes: html5lib tokenizer suite

#### Semana 3-4: Tree Builder Insertion Modes
- [ ] Implementar 24 insertion modes
- [ ] Template elements com stack de modes
- [ ] Frameset handling (frameset-ok flag)
- [ ] Foreign content (SVG/MathML)
- [ ] Testes: html5lib tree construction suite

#### Semana 5-6: AAA e Foster Parenting
- [ ] Adoption Agency Algorithm completo
- [ ] Foster parenting robusto
- [ ] Edge cases e corner cases
- [ ] Testes: casos específicos do spec

**Entregável**: html5lib 100% pass rate

### 6.2 Fase 2: Performance Core (6-8 semanas)

#### Semana 7-8: SIMD AVX-512
- [ ] Whitespace detection (64-byte parallel)
- [ ] Entity lookup (perfect hashing)
- [ ] Tag name scanning
- [ ] Runtime CPU detection
- [ ] Benchmarks: 10-15% speedup vs AVX2

#### Semana 9-10: Zero-Copy Strings
- [ ] Arena allocator otimizado
- [ ] String interner com perfect hashing
- [ ] Memory pool management
- [ ] Benchmarks: 30% memory reduction

#### Semana 11-12: Memory Layout
- [ ] Compact node representation (32 bytes)
- [ ] Structure of Arrays (SoA)
- [ ] Cache-friendly traversal
- [ ] Prefetching
- [ ] Benchmarks: 20% speedup

#### Semana 13-14: Branch Prediction
- [ ] likely/unlikely hints
- [ ] Hot path optimization
- [ ] Profile-guided optimization
- [ ] Benchmarks: 5-10% speedup

**Entregável**: Throughput 300+ MB/s

### 6.3 Fase 3: Features Avançadas (8-10 semanas)

#### Semana 15-16: Thread Pool
- [ ] Thread pool próprio (std::thread)
- [ ] Work stealing scheduler
- [ ] Channel implementation
- [ ] Backpressure handling

#### Semana 17-18: Speculative Parsing
- [ ] Tokenizer em thread separada
- [ ] Tree builder consome via channel
- [ ] Fallback single-thread
- [ ] Benchmarks: 2-3x speedup

#### Semana 19-20: Preload Scanner
- [ ] Scanner state machine
- [ ] SIMD tag scanning
- [ ] Parallel scanning
- [ ] Benchmarks: < 0.1ms latency

#### Semana 21-22: Streaming Incremental
- [ ] Chunk processing (16KB)
- [ ] Latência < 1ms por chunk
- [ ] Backpressure handling
- [ ] Benchmarks: p99 < 1ms

#### Semana 23-24: Benchmark Framework
- [ ] Statistical analysis
- [ ] Comparison engine
- [ ] HTML report generation
- [ ] CI integration

**Entregável**: Throughput 500+ MB/s

### 6.4 Fase 4: Polish & Benchmarking (2-4 semanas)

#### Semana 25-26: Benchmarks Completos
- [ ] Micro benchmarks (lexer, tokenizer, builder)
- [ ] Macro benchmarks (real pages)
- [ ] Comparison com Chrome/Firefox
- [ ] Regression detection

#### Semana 27-28: Documentação
- [ ] API documentation (rustdoc)
- [ ] Architecture guide
- [ ] Performance guide
- [ ] Contributing guide

**Entregável**: Produção-ready

---

## 7. Métricas de Sucesso

### 7.1 Performance Targets

| Métrica | Baseline | Fase 2 | Fase 3 | Fase 4 |
|---------|----------|--------|--------|--------|
| Throughput | 50 MB/s | 300 MB/s | 500 MB/s | 600 MB/s |
| Latência (16KB) | 5ms | 2ms | 0.5ms | 0.3ms |
| Memória (10MB) | 80MB | 50MB | 40MB | 35MB |
| SIMD coverage | 15% | 40% | 60% | 70% |

### 7.2 Conformance Targets

| Métrica | Baseline | Fase 1 | Fase 2 | Fase 3 |
|---------|----------|--------|--------|--------|
| html5lib tokenizer | 95% | 100% | 100% | 100% |
| html5lib tree | 90% | 100% | 100% | 100% |
| WPT HTML parsing | 85% | 95% | 99% | 99.5% |

---

## 8. Riscos e Mitigações

### 8.1 Riscos Técnicos

| Risco | Probabilidade | Impacto | Mitigação |
|-------|---------------|---------|-----------|
| Speculative parsing complexo | Alta | Alto | Implementar incremental, fallback single-thread |
| AVX-512 bugs | Média | Médio | Extensive testing, fallback AVX2 |
| html5lib 100% difícil | Alta | Médio | Priorizar casos comuns, edge cases depois |
| Performance regressions | Média | Alto | CI com benchmarks, bisect |
| Memory leaks | Baixa | Alto | Valgrind, ASAN, extensive testing |

### 8.2 Riscos de Cronograma

| Risco | Probabilidade | Impacto | Mitigação |
|-------|---------------|---------|-----------|
| Underestimate complexity | Alta | Alto | Buffer de 20% no cronograma |
| Blocker bugs | Média | Médio | Daily standup, quick escalation |
| Scope creep | Média | Médio | Strict scope control, MVP first |

---

## 9. Dependências Zero

### 9.1 Implementações Próprias Obrigatórias

**Já Implementado**:
- ✅ Lexer (src/ace/html/lexer.rs)
- ✅ Tokenizer (src/ace/html/tokenizer.rs)
- ✅ Tree Builder (src/ace/html/tree_builder.rs)
- ✅ Arena Allocator (src/ace/html/arena.rs)
- ✅ String Interner (src/ace/html/interner.rs)
- ✅ SIMD básico (src/ace/html/simd.rs)

**A Implementar**:
- ❌ Thread Pool (src/ace/html/thread_pool.rs)
- ❌ JSON Parser (src/ace/html/tests/json_parser.rs)
- ❌ Benchmark Framework (src/ace/html/bench/)
- ❌ Test Harness (src/ace/html/tests/html5lib_harness.rs)
- ❌ SIMD AVX-512 (src/ace/html/simd.rs - extend)

### 9.2 Apenas std Permitido

```toml
# Cargo.toml - ZERO dependências externas
[dependencies]
# NADA aqui!

[dev-dependencies]
# NADA aqui também!
```

---

## 10. Conclusão

Este design técnico detalha a arquitetura e implementação necessária para elevar o ACE-HTML ao nível Chrome ou superior, mantendo a filosofia **100% PRÓPRIA DO ALBEDO - ZERO DEPENDÊNCIAS EXTERNAS**.

**Principais Inovações**:
1. **Speculative Parsing**: Tokenização paralela para 2-3x speedup
2. **SIMD AVX-512**: 64-byte parallel processing para 10-15% speedup
3. **Zero-Copy Strings**: Arena allocator + string interner para 30% memory reduction
4. **Benchmark Framework Próprio**: Statistical analysis sem Criterion
5. **Test Harness Próprio**: JSON parser + test runner sem serde_json

**Cronograma**: 20-28 semanas (~5-7 meses)

**Resultado Esperado**:
- ✅ Throughput ≥ 500 MB/s (Chrome-level)
- ✅ Conformidade 100% WHATWG
- ✅ Memória ≤ 50% do Chrome
- ✅ Zero dependências externas

---

**Documento criado**: 2026-04-06  
**Versão**: 1.0  
**Autor**: Kiro AI Assistant  
**Status**: Draft → Aguardando revisão
