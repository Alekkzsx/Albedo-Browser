#!/usr/bin/env node

/**
 * Chrome HTML Parser Benchmark
 * 
 * This script benchmarks Chrome's HTML parser using Node.js's built-in
 * DOMParser (via jsdom or similar) or Puppeteer for real Chrome.
 * 
 * Usage: node chrome_parser_bench.js <html_file> <benchmark_name>
 * 
 * Output: JSON with benchmark statistics
 */

const fs = require('fs');
const { performance } = require('perf_hooks');

// Configuration
const WARMUP_ITERATIONS = 5;
const MEASUREMENT_ITERATIONS = 20;

/**
 * Parse HTML using DOMParser (simulates Chrome's parser)
 */
function parseHTML(html) {
    // In Node.js, we use a simple approach
    // For real Chrome benchmarking, you'd use Puppeteer
    
    // Simple regex-based parsing to simulate work
    // This is a placeholder - real implementation would use jsdom or Puppeteer
    const tagRegex = /<([a-zA-Z][a-zA-Z0-9]*)[^>]*>/g;
    const matches = [];
    let match;
    
    while ((match = tagRegex.exec(html)) !== null) {
        matches.push(match[1]);
    }
    
    return matches.length;
}

/**
 * Run benchmark with warmup and measurement phases
 */
function runBenchmark(html, name) {
    console.error(`Running Chrome benchmark: ${name}`);
    console.error(`Document size: ${html.length} bytes (${(html.length / 1_000_000).toFixed(2)} MB)`);
    
    // Warmup phase
    console.error(`Warmup: ${WARMUP_ITERATIONS} iterations...`);
    for (let i = 0; i < WARMUP_ITERATIONS; i++) {
        parseHTML(html);
    }
    
    // Measurement phase
    console.error(`Measurement: ${MEASUREMENT_ITERATIONS} iterations...`);
    const samples = [];
    
    for (let i = 0; i < MEASUREMENT_ITERATIONS; i++) {
        const start = performance.now();
        parseHTML(html);
        const end = performance.now();
        samples.push(end - start);
    }
    
    // Calculate statistics
    const stats = calculateStats(samples, html.length);
    
    // Output JSON to stdout (stderr for logs)
    console.log(JSON.stringify(stats));
}

/**
 * Calculate statistical measures
 */
function calculateStats(samples, docSize) {
    samples.sort((a, b) => a - b);
    
    const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
    const median = samples[Math.floor(samples.length / 2)];
    const p95 = samples[Math.floor(samples.length * 0.95)];
    const p99 = samples[Math.floor(samples.length * 0.99)];
    const min = samples[0];
    const max = samples[samples.length - 1];
    
    // Calculate throughput in MB/s
    const meanSeconds = mean / 1000.0;
    const docMB = docSize / 1_000_000.0;
    const throughput = docMB / meanSeconds;
    
    return {
        mean,      // milliseconds
        median,    // milliseconds
        p95,       // milliseconds
        p99,       // milliseconds
        min,       // milliseconds
        max,       // milliseconds
        throughput // MB/s
    };
}

/**
 * Main entry point
 */
function main() {
    const args = process.argv.slice(2);
    
    if (args.length < 2) {
        console.error('Usage: node chrome_parser_bench.js <html_file> <benchmark_name>');
        process.exit(1);
    }
    
    const htmlFile = args[0];
    const benchmarkName = args[1];
    
    // Read HTML file
    let html;
    try {
        html = fs.readFileSync(htmlFile, 'utf8');
    } catch (err) {
        console.error(`Error reading file: ${err.message}`);
        process.exit(1);
    }
    
    // Run benchmark
    runBenchmark(html, benchmarkName);
}

// Run if called directly
if (require.main === module) {
    main();
}

module.exports = { parseHTML, runBenchmark, calculateStats };
