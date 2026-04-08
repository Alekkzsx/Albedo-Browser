#!/usr/bin/env node

/**
 * Firefox HTML Parser Benchmark (Playwright Version)
 * 
 * This script benchmarks Firefox's REAL HTML parser using Playwright.
 * This provides accurate comparison with Firefox's actual parser.
 * 
 * Installation: npm install playwright
 * Usage: node firefox_parser_bench_playwright.js <html_file> <benchmark_name>
 * 
 * Output: JSON with benchmark statistics
 */

const fs = require('fs');

// Configuration
const WARMUP_ITERATIONS = 5;
const MEASUREMENT_ITERATIONS = 20;

/**
 * Check if Playwright is available
 */
function checkPlaywright() {
    try {
        require.resolve('playwright');
        return true;
    } catch (e) {
        return false;
    }
}

/**
 * Run benchmark using real Firefox via Playwright
 */
async function runPlaywrightBenchmark(html, name) {
    const { firefox } = require('playwright');
    
    console.error(`Running Firefox benchmark with Playwright: ${name}`);
    console.error(`Document size: ${html.length} bytes (${(html.length / 1_000_000).toFixed(2)} MB)`);
    
    // Launch headless Firefox
    const browser = await firefox.launch({
        headless: true,
        args: []
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
    
    for (let i = 0; i < MEASUREMENT_ITERATIONS; i++) {
        // Measure parsing time using Firefox's Performance API
        const timing = await page.evaluate((htmlContent) => {
            const start = performance.now();
            
            // Force re-parse by setting innerHTML
            const div = document.createElement('div');
            div.innerHTML = htmlContent;
            
            const end = performance.now();
            return end - start;
        }, html);
        
        samples.push(timing);
    }
    
    await browser.close();
    
    // Calculate statistics
    const stats = calculateStats(samples, html.length);
    
    // Output JSON to stdout
    console.log(JSON.stringify(stats));
}

/**
 * Fallback: Run benchmark without Playwright (simple parsing)
 */
function runFallbackBenchmark(html, name) {
    console.error(`Playwright not available, using fallback parser`);
    console.error(`Running Firefox benchmark: ${name}`);
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
 * Main entry point
 */
async function main() {
    const args = process.argv.slice(2);
    
    if (args.length < 2) {
        console.error('Usage: node firefox_parser_bench_playwright.js <html_file> <benchmark_name>');
        console.error('');
        console.error('Note: Requires Playwright. Install with: npm install playwright');
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
    
    // Check if Playwright is available
    if (checkPlaywright()) {
        await runPlaywrightBenchmark(html, benchmarkName);
    } else {
        console.error('⚠️  Playwright not installed. Using fallback parser.');
        console.error('   For accurate Firefox benchmarks, install: npm install playwright');
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

module.exports = { runPlaywrightBenchmark, runFallbackBenchmark, calculateStats };
