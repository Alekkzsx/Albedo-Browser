#!/usr/bin/env node

/**
 * Chrome HTML Parser Benchmark (Puppeteer Version)
 * 
 * This script benchmarks Chrome's REAL HTML parser using Puppeteer.
 * This provides accurate comparison with Chrome's actual parser.
 * 
 * Installation: npm install puppeteer
 * Usage: node chrome_parser_bench_puppeteer.js <html_file> <benchmark_name>
 * 
 * Output: JSON with benchmark statistics
 */

const fs = require('fs');

// Configuration
const WARMUP_ITERATIONS = 5;
const MEASUREMENT_ITERATIONS = 20;

/**
 * Check if Puppeteer is available
 */
function checkPuppeteer() {
    try {
        require.resolve('puppeteer');
        return true;
    } catch (e) {
        return false;
    }
}

/**
 * Run benchmark using real Chrome via Puppeteer
 */
async function runPuppeteerBenchmark(html, name) {
    const puppeteer = require('puppeteer');
    
    console.error(`Running Chrome benchmark with Puppeteer: ${name}`);
    console.error(`Document size: ${html.length} bytes (${(html.length / 1_000_000).toFixed(2)} MB)`);
    
    // Launch headless Chrome
    const browser = await puppeteer.launch({
        headless: 'new',
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
    
    const page = await browser.newPage();
    
    // Warmup phase
    console.error(`Warmup: ${WARMUP_ITERATIONS} iterations...`);
    for (let i = 0; i < WARMUP_ITERATIONS; i++) {
        await page.setContent(html, { waitUntil: 'domcontentloaded' });
    }
    
    // Measurement phase
    console.error(`Measurement: ${MEASUREMENT_ITERATIONS} iterations...`);
    const samples = [];
    const memorySamples = [];
    
    for (let i = 0; i < MEASUREMENT_ITERATIONS; i++) {
        // Clear previous content and force GC
        await page.evaluate(() => {
            document.body.innerHTML = '';
            if (window.gc) window.gc();
        });
        
        // Measure memory before parsing
        const memoryBefore = await page.metrics();
        
        // Measure parsing time using Chrome's Performance API
        const timing = await page.evaluate((htmlContent) => {
            const start = performance.now();
            
            // Force re-parse by setting innerHTML
            const div = document.createElement('div');
            div.innerHTML = htmlContent;
            
            const end = performance.now();
            return end - start;
        }, html);
        
        // Measure memory after parsing
        const memoryAfter = await page.metrics();
        
        // Calculate memory delta (in bytes)
        const memoryDelta = memoryAfter.JSHeapUsedSize - memoryBefore.JSHeapUsedSize;
        
        samples.push(timing);
        memorySamples.push(memoryDelta);
    }
    
    await browser.close();
    
    // Calculate statistics
    const stats = calculateStats(samples, html.length);
    const memoryStats = calculateMemoryStats(memorySamples);
    
    // Merge stats
    const result = {
        ...stats,
        memory: memoryStats
    };
    
    // Output JSON to stdout
    console.log(JSON.stringify(result));
}

/**
 * Fallback: Run benchmark without Puppeteer (simple parsing)
 */
function runFallbackBenchmark(html, name) {
    console.error(`Puppeteer not available, using fallback parser`);
    console.error(`Running Chrome benchmark: ${name}`);
    console.error(`Document size: ${html.length} bytes (${(html.length / 1_000_000).toFixed(2)} MB)`);
    
    // Simple parsing simulation
    const parseHTML = (html) => {
        const tagRegex = /<([a-zA-Z][a-zA-Z0-9]*)[^>]*>/g;
        const matches = [];
        let match;
        
        while ((match = tagRegex.exec(html)) !== null) {
            matches.push(match[1]);
        }
        
        return matches.length;
    };
    
    // Warmup
    console.error(`Warmup: ${WARMUP_ITERATIONS} iterations...`);
    for (let i = 0; i < WARMUP_ITERATIONS; i++) {
        parseHTML(html);
    }
    
    // Measurement
    console.error(`Measurement: ${MEASUREMENT_ITERATIONS} iterations...`);
    const samples = [];
    
    for (let i = 0; i < MEASUREMENT_ITERATIONS; i++) {
        const start = Date.now();
        parseHTML(html);
        const end = Date.now();
        samples.push(end - start);
    }
    
    const stats = calculateStats(samples, html.length);
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
    
    // Calculate standard deviation
    const variance = samples.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) / samples.length;
    const stdDev = Math.sqrt(variance);
    
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
        stdDev,    // milliseconds
        throughput // MB/s
    };
}

/**
 * Calculate memory statistics
 */
function calculateMemoryStats(samples) {
    samples.sort((a, b) => a - b);
    
    const mean = samples.reduce((a, b) => a + b, 0) / samples.length;
    const median = samples[Math.floor(samples.length / 2)];
    const p95 = samples[Math.floor(samples.length * 0.95)];
    const p99 = samples[Math.floor(samples.length * 0.99)];
    const min = samples[0];
    const max = samples[samples.length - 1];
    
    // Calculate standard deviation
    const variance = samples.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) / samples.length;
    const stdDev = Math.sqrt(variance);
    
    return {
        mean_bytes: mean,
        median_bytes: median,
        p95_bytes: p95,
        p99_bytes: p99,
        min_bytes: min,
        max_bytes: max,
        stdDev_bytes: stdDev,
        mean_mb: mean / 1_000_000.0,
        median_mb: median / 1_000_000.0,
        p95_mb: p95 / 1_000_000.0,
        p99_mb: p99 / 1_000_000.0
    };
}

/**
 * Main entry point
 */
async function main() {
    const args = process.argv.slice(2);
    
    if (args.length < 2) {
        console.error('Usage: node chrome_parser_bench_puppeteer.js <html_file> <benchmark_name>');
        console.error('');
        console.error('Note: Requires Puppeteer. Install with: npm install puppeteer');
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
    
    // Check if Puppeteer is available
    if (checkPuppeteer()) {
        await runPuppeteerBenchmark(html, benchmarkName);
    } else {
        console.error('⚠️  Puppeteer not installed. Using fallback parser.');
        console.error('   For accurate Chrome benchmarks, install: npm install puppeteer');
        runFallbackBenchmark(html, benchmarkName);
    }
}

// Run if called directly
if (require.main === module) {
    main().catch(err => {
        console.error(`Error: ${err.message}`);
        process.exit(1);
    });
}

module.exports = { runPuppeteerBenchmark, runFallbackBenchmark, calculateStats };
