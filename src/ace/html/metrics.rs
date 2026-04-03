//! Metrics e Profiling para o Parser HTML
//!
//! Este módulo fornece instrumentação detalhada para medir performance,
//! uso de memória e eficiência do parser ACE-HTML.
//!
//! ## Métricas Coletadas
//! - Tempo de tokenização
//! - Tempo de construção da árvore DOM
//! - Hit rate do string interner
//! - Alocações da arena
//! - Throughput (bytes/ms)
//! - Latência por chunk (streaming)

use std::time::{Duration, Instant};
use crate::html::arena::ArenaStats;
use crate::html::interner::InternerStats;
use crate::html::small_attr_map::SmallAttributeMapStats;

/// Estatísticas completas do parser
#[derive(Debug, Clone)]
pub struct ParserMetrics {
    /// Timestamp de início do parsing
    pub parse_start: Instant,
    
    /// Tempo total de tokenização
    pub tokenization_time: Duration,
    
    /// Tempo total de construção da árvore
    pub tree_building_time: Duration,
    
    /// Tempo total (tokenização + tree building)
    pub total_time: Duration,
    
    /// Número de tokens gerados
    pub tokens_generated: usize,
    
    /// Número de nodes criados na DOM
    pub nodes_created: usize,
    
    /// Bytes totais de input
    pub input_bytes: usize,
    
    /// Throughput (bytes por segundo)
    pub throughput_bps: f64,
    
    /// Estatísticas da arena
    pub arena_stats: Option<ArenaStats>,
    
    /// Estatísticas do string interner
    pub interner_stats: Option<InternerStats>,
    
    /// Contagem de attribute maps small vs large
    pub small_attr_count: usize,
    pub large_attr_count: usize,
}

impl ParserMetrics {
    /// Cria novas métricas zeradas
    pub fn new() -> Self {
        Self {
            parse_start: Instant::now(),
            tokenization_time: Duration::ZERO,
            tree_building_time: Duration::ZERO,
            total_time: Duration::ZERO,
            tokens_generated: 0,
            nodes_created: 0,
            input_bytes: 0,
            throughput_bps: 0.0,
            arena_stats: None,
            interner_stats: None,
            small_attr_count: 0,
            large_attr_count: 0,
        }
    }
    
    /// Finaliza a medição e calcula métricas derivadas
    pub fn finish(&mut self, input_bytes: usize) {
        self.input_bytes = input_bytes;
        self.total_time = self.parse_start.elapsed();
        
        if self.total_time.as_secs_f64() > 0.0 {
            self.throughput_bps = input_bytes as f64 / self.total_time.as_secs_f64();
        }
    }
    
    /// Retorna estatísticas formatadas para display
    pub fn summary(&self) -> ParserStats {
        ParserStats {
            total_time_ms: self.total_time.as_secs_f64() * 1000.0,
            tokenization_ms: self.tokenization_time.as_secs_f64() * 1000.0,
            tree_building_ms: self.tree_building_time.as_secs_f64() * 1000.0,
            throughput_mbps: (self.throughput_bps / 1_000_000.0),
            tokens_per_sec: if self.total_time.as_secs_f64() > 0.0 {
                self.tokens_generated as f64 / self.total_time.as_secs_f64()
            } else {
                0.0
            },
            nodes_per_sec: if self.total_time.as_secs_f64() > 0.0 {
                self.nodes_created as f64 / self.total_time.as_secs_f64()
            } else {
                0.0
            },
            bytes_per_token: if self.tokens_generated > 0 {
                self.input_bytes / self.tokens_generated
            } else {
                0
            },
            bytes_per_node: if self.nodes_created > 0 {
                self.input_bytes / self.nodes_created
            } else {
                0
            },
        }
    }
    
    /// Imprime relatório detalhado
    pub fn print_report(&self) {
        let stats = self.summary();
        
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║         ACE-HTML Parser Performance Report              ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Input Size: {:>12} bytes                                 ", self.input_bytes);
        println!("║ Total Time: {:>12.3} ms                                  ", stats.total_time_ms);
        println!("║ Throughput: {:>12.3} MB/s                                ", stats.throughput_mbps);
        println!("╠──────────────────────────────────────────────────────────╢");
        println!("║ Tokenization: {:>10.3} ms ({:>5.1}%)                     ", 
                 stats.tokenization_ms,
                 if stats.total_time_ms > 0.0 { stats.tokenization_ms / stats.total_time_ms * 100.0 } else { 0.0 });
        println!("║ Tree Building:{:>10.3} ms ({:>5.1}%)                     ",
                 stats.tree_building_ms,
                 if stats.total_time_ms > 0.0 { stats.tree_building_ms / stats.total_time_ms * 100.0 } else { 0.0 });
        println!("╠──────────────────────────────────────────────────────────╢");
        println!("║ Tokens Generated: {:>8}                                   ", self.tokens_generated);
        println!("║ Nodes Created:    {:>8}                                   ", self.nodes_created);
        println!("║ Tokens/sec:       {:>12.0}                               ", stats.tokens_per_sec);
        println!("║ Nodes/sec:        {:>12.0}                               ", stats.nodes_per_sec);
        println!("╠──────────────────────────────────────────────────────────╢");
        println!("║ Bytes/Token:      {:>8}                                   ", stats.bytes_per_token);
        println!("║ Bytes/Node:       {:>8}                                   ", stats.bytes_per_node);
        
        if let Some(ref arena) = self.arena_stats {
            println!("╠──────────────────────────────────────────────────────────╢");
            println!("║ Arena Allocator Stats:                                   ");
            println!("║   Chunks:         {:>8}                                   ", arena.chunk_count);
            println!("║   Capacity:       {:>8} bytes                             ", arena.total_capacity);
            println!("║   Allocated:      {:>8} bytes                             ", arena.total_allocated);
            println!("║   Utilization:    {:>8.1}%                                ", arena.utilization * 100.0);
        }
        
        if let Some(ref interner) = self.interner_stats {
            println!("╠──────────────────────────────────────────────────────────╢");
            println!("║ String Interner Stats:                                   ");
            println!("║   Unique Strings: {:>8}                                   ", interner.unique_strings);
            println!("║   Hits:           {:>8}                                   ", interner.hit_count);
            println!("║   Misses:         {:>8}                                   ", interner.miss_count);
            println!("║   Hit Rate:       {:>8.1}%                                ", interner.hit_rate * 100.0);
        }
        
        println!("╚══════════════════════════════════════════════════════════╝\n");
    }
}

impl Default for ParserMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Estatísticas sumarizadas para display rápido
#[derive(Debug, Clone)]
pub struct ParserStats {
    pub total_time_ms: f64,
    pub tokenization_ms: f64,
    pub tree_building_ms: f64,
    pub throughput_mbps: f64,
    pub tokens_per_sec: f64,
    pub nodes_per_sec: f64,
    pub bytes_per_token: usize,
    pub bytes_per_node: usize,
}

/// Builder para coletar métricas durante o parsing
pub struct MetricsCollector {
    metrics: ParserMetrics,
    tokenization_start: Option<Instant>,
    tree_building_start: Option<Instant>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: ParserMetrics::new(),
            tokenization_start: None,
            tree_building_start: None,
        }
    }
    
    pub fn start_tokenization(&mut self) {
        self.tokenization_start = Some(Instant::now());
    }
    
    pub fn end_tokenization(&mut self, tokens: usize) {
        if let Some(start) = self.tokenization_start.take() {
            self.metrics.tokenization_time = start.elapsed();
            self.metrics.tokens_generated = tokens;
        }
    }
    
    pub fn start_tree_building(&mut self) {
        self.tree_building_start = Some(Instant::now());
    }
    
    pub fn end_tree_building(&mut self, nodes: usize) {
        if let Some(start) = self.tree_building_start.take() {
            self.metrics.tree_building_time = start.elapsed();
            self.metrics.nodes_created = nodes;
        }
    }
    
    pub fn set_arena_stats(&mut self, stats: ArenaStats) {
        self.metrics.arena_stats = Some(stats);
    }
    
    pub fn set_interner_stats(&mut self, stats: InternerStats) {
        self.metrics.interner_stats = Some(stats);
    }
    
    pub fn count_small_attribute_map(&mut self) {
        self.metrics.small_attr_count += 1;
    }
    
    pub fn count_large_attribute_map(&mut self) {
        self.metrics.large_attr_count += 1;
    }
    
    pub fn finish(mut self, input_bytes: usize) -> ParserMetrics {
        self.metrics.finish(input_bytes);
        self.metrics
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_new() {
        let metrics = ParserMetrics::new();
        assert_eq!(metrics.tokens_generated, 0);
        assert_eq!(metrics.nodes_created, 0);
        assert_eq!(metrics.input_bytes, 0);
    }
    
    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        
        collector.start_tokenization();
        std::thread::sleep(Duration::from_millis(10));
        collector.end_tokenization(1000);
        
        collector.start_tree_building();
        std::thread::sleep(Duration::from_millis(15));
        collector.end_tree_building(500);
        
        let metrics = collector.finish(50000);
        
        assert!(metrics.tokenization_time.as_millis() >= 10);
        assert!(metrics.tree_building_time.as_millis() >= 15);
        assert_eq!(metrics.tokens_generated, 1000);
        assert_eq!(metrics.nodes_created, 500);
        assert!(metrics.throughput_bps > 0.0);
    }
    
    #[test]
    fn test_parser_stats() {
        let mut metrics = ParserMetrics::new();
        metrics.tokens_generated = 1000;
        metrics.nodes_created = 500;
        metrics.input_bytes = 50000;
        metrics.total_time = Duration::from_millis(100);
        
        let stats = metrics.summary();
        
        assert!((stats.total_time_ms - 100.0).abs() < 0.1);
        assert_eq!(stats.bytes_per_token, 50);
        assert_eq!(stats.bytes_per_node, 100);
        assert!(stats.throughput_mbps > 0.0);
    }
}
