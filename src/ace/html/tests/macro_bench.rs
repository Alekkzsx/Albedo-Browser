//! Macro benchmarks for HTML parser
//! 
//! Tests realistic documents: Wikipedia, GitHub, Twitter, Amazon, YouTube

#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use crate::ace::html::{
    build_document,
    bench::{BenchRunner, BenchConfig},
};

// ============================================================================
// MACRO BENCHMARKS - Real-world documents
// ============================================================================

/// Generate Wikipedia-like homepage (1.2 MB)
#[allow(dead_code)]
fn generate_wikipedia_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Wikipedia - The Free Encyclopedia</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
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
    
    // Generate multiple articles
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
        html.push_str("<div class='padding'>Content padding</div>");
    }
    
    html
}

/// Generate GitHub README-like page (500 KB)
#[allow(dead_code)]
fn generate_github_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>GitHub - Repository</title>
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
    
    // Generate README content
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
        html.push_str("<div>Padding content</div>");
    }
    
    html
}

/// Generate Twitter timeline-like page (2 MB)
#[allow(dead_code)]
fn generate_twitter_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Twitter / Home</title>
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
                    <img src="/avatar/{}.jpg" alt="User Avatar" />
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
#[allow(dead_code)]
fn generate_amazon_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Amazon.com: Product</title>
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
                    <img src="/images/product-{}-1.jpg" alt="Product" />
                    <img src="/images/product-{}-2.jpg" alt="Product" />
                    <img src="/images/product-{}-3.jpg" alt="Product" />
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
#[allow(dead_code)]
fn generate_youtube_html() -> String {
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>YouTube - Watch</title>
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
                    <video src="/videos/{}.mp4" controls></video>
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
                        <img src="/avatar/user1.jpg" alt="Avatar" />
                        <div class="comment-content">
                            <span class="username">User 1</span>
                            <p>Great video!</p>
                        </div>
                    </div>
                    <div class="comment">
                        <img src="/avatar/user2.jpg" alt="Avatar" />
                        <div class="comment-content">
                            <span class="username">User 2</span>
                            <p>Thanks for sharing!</p>
                        </div>
                    </div>
                </div>
                <div class="recommendations">
                    <h3>Recommended Videos</h3>
                    <div class="video-thumbnail">
                        <img src="/thumbnails/rec1.jpg" alt="Thumbnail" />
                        <span>Recommended Video 1</span>
                    </div>
                    <div class="video-thumbnail">
                        <img src="/thumbnails/rec2.jpg" alt="Thumbnail" />
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
// BENCHMARK TESTS
// ============================================================================

/// Benchmark 1: Wikipedia homepage (1.2 MB)
#[test]
fn bench_macro_wikipedia() {
    let html = generate_wikipedia_html();
    
    println!("\n=== Macro Benchmark: Wikipedia (1.2 MB) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    
    let config = BenchConfig::new("Wikipedia Homepage")
        .with_warmup(2)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    // Should parse in reasonable time
    assert!(result.stats.mean < Duration::from_millis(300));
}

/// Benchmark 2: GitHub README (500 KB)
#[test]
fn bench_macro_github() {
    let html = generate_github_html();
    
    println!("\n=== Macro Benchmark: GitHub README (500 KB) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    
    let config = BenchConfig::new("GitHub README")
        .with_warmup(3)
        .with_measurements(15);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(200));
}

/// Benchmark 3: Twitter timeline (2 MB)
#[test]
fn bench_macro_twitter() {
    let html = generate_twitter_html();
    
    println!("\n=== Macro Benchmark: Twitter Timeline (2 MB) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    
    let config = BenchConfig::new("Twitter Timeline")
        .with_warmup(2)
        .with_measurements(5);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(500));
}

/// Benchmark 4: Amazon product (800 KB)
#[test]
fn bench_macro_amazon() {
    let html = generate_amazon_html();
    
    println!("\n=== Macro Benchmark: Amazon Product (800 KB) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    
    let config = BenchConfig::new("Amazon Product")
        .with_warmup(3)
        .with_measurements(10);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(250));
}

/// Benchmark 5: YouTube watch (1.5 MB)
#[test]
fn bench_macro_youtube() {
    let html = generate_youtube_html();
    
    println!("\n=== Macro Benchmark: YouTube Watch (1.5 MB) ===");
    println!("Document size: {} bytes ({:.2} MB)", html.len(), html.len() as f64 / 1_000_000.0);
    
    let config = BenchConfig::new("YouTube Watch")
        .with_warmup(2)
        .with_measurements(8);
    
    let runner = BenchRunner::new(config);
    let result = runner.run(|| {
        let _doc = build_document(&html);
    });
    
    print_stats(&result.stats);
    
    assert!(result.stats.mean < Duration::from_millis(400));
}

// ============================================================================
// Helper Functions
// ============================================================================

#[allow(dead_code)]
fn print_stats(stats: &crate::ace::html::bench::BenchStats) {
    println!("  Mean:   {:?}", stats.mean);
    println!("  Median: {:?}", stats.median);
    println!("  P95:    {:?}", stats.p95);
    println!("  P99:    {:?}", stats.p99);
    println!("  Min:    {:?}", stats.min);
    println!("  Max:    {:?}", stats.max);
    println!("  CV:     {:.2}%", stats.coefficient_of_variation());
    println!("  Stable: {}", if stats.is_stable() { "Yes" } else { "No" });
}
