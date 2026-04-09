//! Throughput validation tests for ACE HTML parser
//! 
//! Task 4.3.1.1: Validate that HTML parser achieves throughput ≥ 500 MB/s
//! 
//! This test suite validates the final performance requirement:
//! - Throughput ≥ 500 MB/s on various document types
//! - Tests with Wikipedia, GitHub, Twitter, Amazon, YouTube pages
//! - Measures and reports throughput in MB/s
//! - Documents results and performance characteristics

#[cfg(test)]
use std::time::{Duration, Instant};
#[cfg(test)]
use crate::ace::html::{
    build_document,
    bench::{BenchRunner, BenchConfig, BenchStats},
};

// ============================================================================
// THROUGHPUT CALCULATION
// ============================================================================

/// Calculate throughput in MB/s
#[cfg(test)]
fn calculate_throughput(bytes: usize, duration: Duration) -> f64 {
    let megabytes = bytes as f64 / 1_000_000.0;
    let seconds = duration.as_secs_f64();
    
    if seconds > 0.0 {
        megabytes / seconds
    } else {
        0.0
    }
}

/// Throughput result with detailed metrics
#[cfg(test)]
#[derive(Debug, Clone)]
struct ThroughputResult {
    name: String,
    document_size: usize,
    stats: BenchStats,
    mean_throughput: f64,
    median_throughput: f64,
    min_throughput: f64,
    max_throughput: f64,
}

#[cfg(test)]
impl ThroughputResult {
    fn from_bench(name: String, document_size: usize, stats: BenchStats) -> Self {
        let mean_throughput = calculate_throughput(document_size, stats.mean);
        let median_throughput = calculate_throughput(document_size, stats.median);
        let min_throughput = calculate_throughput(document_size, stats.max); // max time = min throughput
        let max_throughput = calculate_throughput(document_size, stats.min); // min time = max throughput
        
        Self {
            name,
            document_size,
            stats,
            mean_throughput,
            median_throughput,
            min_throughput,
            max_throughput,
        }
    }
    
    fn print_report(&self) {
        println!("\n{}", "=".repeat(70));
        println!("Throughput Validation: {}", self.name);
        println!("{}", "=".repeat(70));
        println!("Document Size:      {:.2} MB ({} bytes)", 
            self.document_size as f64 / 1_000_000.0, 
            self.document_size
        );
        println!("\nTiming Statistics:");
        println!("  Mean:             {:?}", self.stats.mean);
        println!("  Median:           {:?}", self.stats.median);
        println!("  P95:              {:?}", self.stats.p95);
        println!("  P99:              {:?}", self.stats.p99);
        println!("  Min:              {:?}", self.stats.min);
        println!("  Max:              {:?}", self.stats.max);
        println!("  Std Dev:          {:?}", self.stats.std_dev);
        println!("  CV:               {:.2}%", self.stats.coefficient_of_variation());
        
        println!("\nThroughput Statistics:");
        println!("  Mean Throughput:  {:.2} MB/s", self.mean_throughput);
        println!("  Median Throughput:{:.2} MB/s", self.median_throughput);
        println!("  Min Throughput:   {:.2} MB/s", self.min_throughput);
        println!("  Max Throughput:   {:.2} MB/s", self.max_throughput);
        
        println!("\nPerformance Assessment:");
        if self.mean_throughput >= 500.0 {
            println!("  ✓ PASSED: Mean throughput ≥ 500 MB/s");
        } else {
            println!("  ✗ FAILED: Mean throughput < 500 MB/s");
        }
        
        if self.stats.is_stable() {
            println!("  ✓ Results are stable (CV < 5%)");
        } else {
            println!("  ⚠ Results are unstable (CV ≥ 5%)");
        }
        
        if self.stats.outlier_percentage() < 5.0 {
            println!("  ✓ Low outlier rate ({:.1}%)", self.stats.outlier_percentage());
        } else {
            println!("  ⚠ High outlier rate ({:.1}%)", self.stats.outlier_percentage());
        }
        
        println!("{}", "=".repeat(70));
    }
}

// ============================================================================
// DOCUMENT GENERATORS
// ============================================================================

/// Generate Wikipedia-like homepage (1.2 MB)
#[cfg(test)]
fn generate_wikipedia_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Wikipedia - The Free Encyclopedia</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="/static/css/main.css">
    <script src="/static/js/app.js" async></script>
</head>
<body>
    <header>
        <nav>
            <ul>
                <li><a href="/wiki/Main_Page">Main Page</a></li>
                <li><a href="/wiki/Special:Random">Random Article</a></li>
            </ul>
        </nav>
    </header>
    <main>
"#);
    
    // Generate multiple articles with realistic content
    for i in 0..50 {
        html.push_str(&format!(r#"
        <article id="article-{}">
            <h2>Article Title {}</h2>
            <div class="content">
                <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
                <p>Ut enim ad minim veniam, quis nostrud exercitation ullamco 
                laboris nisi ut aliquip ex ea commodo consequat.</p>
                <table class="infobox">
                    <tr><th>Property</th><th>Value</th></tr>
                    <tr><td>Type</td><td>Article</td></tr>
                    <tr><td>Category</td><td>General</td></tr>
                </table>
                <ul>
                    <li>Point 1</li>
                    <li>Point 2</li>
                    <li>Point 3</li>
                </ul>
            </div>
        </article>
"#, i, i));
    }
    
    html.push_str(r#"
    </main>
    <footer>
        <p>&copy; 2026 Wikipedia Foundation</p>
    </footer>
</body>
</html>
"#);
    
    // Pad to reach ~1.2 MB
    while html.len() < 1_200_000 {
        html.push_str("<div class='padding'>Content padding for realistic document size</div>");
    }
    
    html
}

/// Generate GitHub README-like page (500 KB)
#[cfg(test)]
fn generate_github_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>GitHub - Repository</title>
    <link rel="stylesheet" href="/assets/github.css">
</head>
<body>
    <header>
        <nav>
            <a href="/">Home</a>
            <a href="/pulls">Pull Requests</a>
            <a href="/issues">Issues</a>
        </nav>
    </header>
    <main>
        <div class="repository-content">
"#);
    
    // Generate README content with code blocks
    for i in 0..30 {
        html.push_str(&format!(r#"
            <section id="section-{}">
                <h2>Section {}</h2>
                <pre><code class="language-rust">
fn main() {{
    println!("Hello, world!");
    let x = 42;
    let y = x * 2;
}}
                </code></pre>
                <p>This is a description of the code above.</p>
                <ul>
                    <li>Feature 1</li>
                    <li>Feature 2</li>
                    <li>Feature 3</li>
                </ul>
            </section>
"#, i, i));
    }
    
    html.push_str(r#"
        </div>
    </main>
</body>
</html>
"#);
    
    // Pad to reach ~500 KB
    while html.len() < 500_000 {
        html.push_str("<div>Padding content for realistic size</div>");
    }
    
    html
}

/// Generate Twitter timeline-like page (2 MB)
#[cfg(test)]
fn generate_twitter_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Twitter / Home</title>
    <link rel="preload" href="/assets/main.css" as="style">
    <script src="/assets/react.js" defer></script>
</head>
<body>
    <div id="react-root">
        <div class="timeline">
"#);
    
    // Generate tweets
    for i in 0..200 {
        html.push_str(&format!(r#"
            <article class="tweet" data-tweet-id="{}">
                <div class="tweet-header">
                    <img src="/avatar/{}.jpg" alt="User Avatar" loading="lazy" />
                    <span class="username">@user{}</span>
                    <span class="timestamp">2h ago</span>
                </div>
                <div class="tweet-content">
                    <p>This is tweet number {}. Lorem ipsum dolor sit amet, 
                    consectetur adipiscing elit. #hashtag #trending</p>
                </div>
                <div class="tweet-actions">
                    <button class="reply">Reply</button>
                    <button class="retweet">Retweet</button>
                    <button class="like">Like</button>
                    <button class="share">Share</button>
                </div>
                <div class="tweet-stats">
                    <span>{} Retweets</span>
                    <span>{} Likes</span>
                </div>
            </article>
"#, i, i, i, i, i * 10, i * 15));
    }
    
    html.push_str(r#"
        </div>
    </div>
</body>
</html>
"#);
    
    // Pad to reach ~2 MB
    while html.len() < 2_000_000 {
        html.push_str("<div class='ad'>Advertisement content</div>");
    }
    
    html
}

/// Generate Amazon product page-like (800 KB)
#[cfg(test)]
fn generate_amazon_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Amazon.com: Product</title>
    <link rel="stylesheet" href="/styles/product.css">
</head>
<body>
    <header>
        <nav>
            <a href="/">Home</a>
            <a href="/cart">Cart</a>
        </nav>
    </header>
    <main>
        <div class="product-page">
"#);
    
    // Generate product details
    for i in 0..20 {
        html.push_str(&format!(r#"
            <div class="product" data-asin="B00{}">
                <h1>Product Title {}</h1>
                <div class="product-images">
                    <img src="/images/product-{}-1.jpg" alt="Product" loading="lazy" />
                    <img src="/images/product-{}-2.jpg" alt="Product" loading="lazy" />
                    <img src="/images/product-{}-3.jpg" alt="Product" loading="lazy" />
                </div>
                <div class="product-info">
                    <span class="price">$99.99</span>
                    <span class="rating">4.5 stars</span>
                    <p class="description">
                        This is a detailed product description. Lorem ipsum dolor 
                        sit amet, consectetur adipiscing elit. Sed do eiusmod tempor.
                    </p>
                </div>
                <div class="product-details">
                    <table>
                        <tr><td>Brand</td><td>Brand Name</td></tr>
                        <tr><td>Model</td><td>Model {}</td></tr>
                        <tr><td>Color</td><td>Black</td></tr>
                        <tr><td>Size</td><td>Medium</td></tr>
                    </table>
                </div>
                <div class="reviews">
                    <h3>Customer Reviews</h3>
                    <div class="review">
                        <span class="reviewer">John Doe</span>
                        <span class="rating">5 stars</span>
                        <p>Great product! Highly recommended.</p>
                    </div>
                    <div class="review">
                        <span class="reviewer">Jane Smith</span>
                        <span class="rating">4 stars</span>
                        <p>Good quality, fast shipping.</p>
                    </div>
                </div>
            </div>
"#, i, i, i, i, i, i));
    }
    
    html.push_str(r#"
        </div>
    </main>
</body>
</html>
"#);
    
    // Pad to reach ~800 KB
    while html.len() < 800_000 {
        html.push_str("<div class='recommendation'>Recommended product</div>");
    }
    
    html
}

/// Generate YouTube watch page-like (1.5 MB)
#[cfg(test)]
fn generate_youtube_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>YouTube - Watch</title>
    <link rel="preload" href="/assets/player.js" as="script">
</head>
<body>
    <div id="page">
        <header>
            <nav>
                <a href="/">Home</a>
                <a href="/trending">Trending</a>
            </nav>
        </header>
        <main>
            <div class="video-page">
"#);
    
    // Generate video content
    for i in 0..50 {
        html.push_str(&format!(r#"
            <div class="video" data-video-id="vid{}">
                <div class="video-player">
                    <video src="/videos/{}.mp4" controls preload="metadata"></video>
                </div>
                <div class="video-info">
                    <h1>Video Title {}</h1>
                    <div class="video-stats">
                        <span>{},000 views</span>
                        <span>{} likes</span>
                        <span>{} dislikes</span>
                    </div>
                    <div class="video-description">
                        <p>This is the video description. Lorem ipsum dolor sit amet.</p>
                    </div>
                </div>
                <div class="comments">
                    <h3>Comments</h3>
                    <div class="comment">
                        <img src="/avatar/user1.jpg" alt="Avatar" loading="lazy" />
                        <div class="comment-content">
                            <span class="username">User 1</span>
                            <p>Great video!</p>
                        </div>
                    </div>
                    <div class="comment">
                        <img src="/avatar/user2.jpg" alt="Avatar" loading="lazy" />
                        <div class="comment-content">
                            <span class="username">User 2</span>
                            <p>Thanks for sharing!</p>
                        </div>
                    </div>
                </div>
                <div class="recommendations">
                    <h3>Recommended Videos</h3>
                    <div class="video-thumbnail">
                        <img src="/thumbnails/rec1.jpg" alt="Thumbnail" loading="lazy" />
                        <span>Recommended Video 1</span>
                    </div>
                    <div class="video-thumbnail">
                        <img src="/thumbnails/rec2.jpg" alt="Thumbnail" loading="lazy" />
                        <span>Recommended Video 2</span>
                    </div>
                </div>
            </div>
"#, i, i, i, i * 10, i * 100, i * 5));
    }
    
    html.push_str(r#"
        </div>
    </main>
</body>
</html>
"#);
    
    // Pad to reach ~1.5 MB
    while html.len() < 1_500_000 {
        html.push_str("<div class='ad-banner'>Advertisement</div>");
    }
    
    html
}

// ============================================================================
// THROUGHPUT VALIDATION TESTS
// ============================================================================

/// Test 1: Wikipedia homepage throughput (1.2 MB)
#[test]
fn test_throughput_wikipedia() {
    let html = generate_wikipedia_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION TEST 1: Wikipedia Homepage");
    println!("{}", "=".repeat(70));
    
    let config = BenchConfig::new("Wikipedia Homepage Throughput")
        .with_warmup(5)
        .with_measurements(20);
    
    let runner = BenchRunner::new(config);
    let bench_result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    let result = ThroughputResult::from_bench(
        "Wikipedia Homepage (1.2 MB)".to_string(),
        size,
        bench_result.stats,
    );
    
    result.print_report();
    
    // Validate throughput requirement
    assert!(
        result.mean_throughput >= 500.0,
        "FAILED: Mean throughput {:.2} MB/s < 500 MB/s",
        result.mean_throughput
    );
}

/// Test 2: GitHub README throughput (500 KB)
#[test]
fn test_throughput_github() {
    let html = generate_github_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION TEST 2: GitHub README");
    println!("{}", "=".repeat(70));
    
    let config = BenchConfig::new("GitHub README Throughput")
        .with_warmup(5)
        .with_measurements(25);
    
    let runner = BenchRunner::new(config);
    let bench_result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    let result = ThroughputResult::from_bench(
        "GitHub README (500 KB)".to_string(),
        size,
        bench_result.stats,
    );
    
    result.print_report();
    
    assert!(
        result.mean_throughput >= 500.0,
        "FAILED: Mean throughput {:.2} MB/s < 500 MB/s",
        result.mean_throughput
    );
}

/// Test 3: Twitter timeline throughput (2 MB)
#[test]
fn test_throughput_twitter() {
    let html = generate_twitter_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION TEST 3: Twitter Timeline");
    println!("{}", "=".repeat(70));
    
    let config = BenchConfig::new("Twitter Timeline Throughput")
        .with_warmup(3)
        .with_measurements(15);
    
    let runner = BenchRunner::new(config);
    let bench_result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    let result = ThroughputResult::from_bench(
        "Twitter Timeline (2 MB)".to_string(),
        size,
        bench_result.stats,
    );
    
    result.print_report();
    
    assert!(
        result.mean_throughput >= 500.0,
        "FAILED: Mean throughput {:.2} MB/s < 500 MB/s",
        result.mean_throughput
    );
}

/// Test 4: Amazon product page throughput (800 KB)
#[test]
fn test_throughput_amazon() {
    let html = generate_amazon_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION TEST 4: Amazon Product Page");
    println!("{}", "=".repeat(70));
    
    let config = BenchConfig::new("Amazon Product Throughput")
        .with_warmup(5)
        .with_measurements(20);
    
    let runner = BenchRunner::new(config);
    let bench_result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    let result = ThroughputResult::from_bench(
        "Amazon Product Page (800 KB)".to_string(),
        size,
        bench_result.stats,
    );
    
    result.print_report();
    
    assert!(
        result.mean_throughput >= 500.0,
        "FAILED: Mean throughput {:.2} MB/s < 500 MB/s",
        result.mean_throughput
    );
}

/// Test 5: YouTube watch page throughput (1.5 MB)
#[test]
fn test_throughput_youtube() {
    let html = generate_youtube_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION TEST 5: YouTube Watch Page");
    println!("{}", "=".repeat(70));
    
    let config = BenchConfig::new("YouTube Watch Throughput")
        .with_warmup(3)
        .with_measurements(15);
    
    let runner = BenchRunner::new(config);
    let bench_result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    let result = ThroughputResult::from_bench(
        "YouTube Watch Page (1.5 MB)".to_string(),
        size,
        bench_result.stats,
    );
    
    result.print_report();
    
    assert!(
        result.mean_throughput >= 500.0,
        "FAILED: Mean throughput {:.2} MB/s < 500 MB/s",
        result.mean_throughput
    );
}

/// Test 6: Comprehensive throughput validation across all document types
#[test]
fn test_comprehensive_throughput_validation() {
    println!("\n{}", "=".repeat(70));
    println!("COMPREHENSIVE THROUGHPUT VALIDATION");
    println!("{}", "=".repeat(70));
    println!("\nRunning throughput tests on all document types...\n");
    
    let test_cases = vec![
        ("Wikipedia (1.2 MB)", generate_wikipedia_html(), 5, 20),
        ("GitHub (500 KB)", generate_github_html(), 5, 25),
        ("Twitter (2 MB)", generate_twitter_html(), 3, 15),
        ("Amazon (800 KB)", generate_amazon_html(), 5, 20),
        ("YouTube (1.5 MB)", generate_youtube_html(), 3, 15),
    ];
    
    let mut all_results = Vec::new();
    let mut all_passed = true;
    
    for (name, html, warmup, measurements) in test_cases {
        let size = html.len();
        
        let config = BenchConfig::new(format!("{} Throughput", name))
            .with_warmup(warmup)
            .with_measurements(measurements);
        
        let runner = BenchRunner::new(config);
        let bench_result = runner.run(|| {
            let _doc = build_document(&html);
        });
        
        let result = ThroughputResult::from_bench(
            name.to_string(),
            size,
            bench_result.stats,
        );
        
        if result.mean_throughput < 500.0 {
            all_passed = false;
        }
        
        all_results.push(result);
    }
    
    // Print summary report
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT VALIDATION SUMMARY");
    println!("{}", "=".repeat(70));
    println!("\n{:<25} {:>12} {:>15} {:>10}", "Document Type", "Size (MB)", "Throughput", "Status");
    println!("{}", "-".repeat(70));
    
    for result in &all_results {
        let status = if result.mean_throughput >= 500.0 { "✓ PASS" } else { "✗ FAIL" };
        println!(
            "{:<25} {:>12.2} {:>12.2} MB/s {:>10}",
            result.name,
            result.document_size as f64 / 1_000_000.0,
            result.mean_throughput,
            status
        );
    }
    
    println!("{}", "=".repeat(70));
    
    // Calculate overall statistics
    let mean_throughputs: Vec<f64> = all_results.iter().map(|r| r.mean_throughput).collect();
    let overall_mean = mean_throughputs.iter().sum::<f64>() / mean_throughputs.len() as f64;
    let overall_min = mean_throughputs.iter().cloned().fold(f64::INFINITY, f64::min);
    let overall_max = mean_throughputs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    
    println!("\nOverall Throughput Statistics:");
    println!("  Mean:    {:.2} MB/s", overall_mean);
    println!("  Min:     {:.2} MB/s", overall_min);
    println!("  Max:     {:.2} MB/s", overall_max);
    
    println!("\nFinal Assessment:");
    if all_passed {
        println!("  ✓ ALL TESTS PASSED: Throughput requirement ≥ 500 MB/s met");
    } else {
        println!("  ✗ SOME TESTS FAILED: Throughput requirement not met");
    }
    println!("{}", "=".repeat(70));
    
    assert!(all_passed, "Throughput validation failed: Not all documents meet 500 MB/s requirement");
}

// ============================================================================
// PERFORMANCE CHARACTERISTICS TESTS
// ============================================================================

/// Test: Throughput consistency across multiple runs
#[test]
fn test_throughput_consistency() {
    let html = generate_wikipedia_html();
    let size = html.len();
    
    println!("\n{}", "=".repeat(70));
    println!("THROUGHPUT CONSISTENCY TEST");
    println!("{}", "=".repeat(70));
    
    let mut throughputs = Vec::new();
    
    for run in 1..=5 {
        let start = Instant::now();
        let _doc = build_document(&html);
        let elapsed = start.elapsed();
        
        let throughput = calculate_throughput(size, elapsed);
        throughputs.push(throughput);
        
        println!("Run {}: {:.2} MB/s ({:?})", run, throughput, elapsed);
    }
    
    let mean = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
    let variance = throughputs.iter()
        .map(|&t| (t - mean).powi(2))
        .sum::<f64>() / throughputs.len() as f64;
    let std_dev = variance.sqrt();
    let cv = (std_dev / mean) * 100.0;
    
    println!("\nConsistency Analysis:");
    println!("  Mean:    {:.2} MB/s", mean);
    println!("  Std Dev: {:.2} MB/s", std_dev);
    println!("  CV:      {:.2}%", cv);
    
    if cv < 5.0 {
        println!("  ✓ Throughput is consistent (CV < 5%)");
    } else {
        println!("  ⚠ Throughput varies significantly (CV ≥ 5%)");
    }
    
    println!("{}", "=".repeat(70));
    
    assert!(mean >= 500.0, "Mean throughput {:.2} MB/s < 500 MB/s", mean);
}
