use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use albedo::ace::html::{parse_document, parse_document_with_options, parse_fragment_with_context, FragmentContext, Namespace, ParserOptions};
use serde_json::json;

struct CountingAllocator;

static ALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static REALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static CURRENT_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
static PEAK_LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            TOTAL_ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            let live = CURRENT_LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            update_peak(live);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        CURRENT_LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, old_layout, new_size);
        if !new_ptr.is_null() {
            REALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
            if new_size > old_layout.size() {
                let delta = new_size - old_layout.size();
                TOTAL_ALLOCATED_BYTES.fetch_add(delta, Ordering::Relaxed);
                let live = CURRENT_LIVE_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
                update_peak(live);
            } else {
                let delta = old_layout.size() - new_size;
                CURRENT_LIVE_BYTES.fetch_sub(delta, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

fn update_peak(live: usize) {
    let _ = PEAK_LIVE_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        if live > current {
            Some(live)
        } else {
            None
        }
    });
}

#[derive(Clone, Debug)]
struct TreeCase {
    mode: String,
    context: Option<String>,
    input: String,
    scripting_enabled: bool,
}

fn main() {
    let limit = std::env::var("ACE_HTML_MEMORY_CASE_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1000);

    reset_stats();
    let started = Instant::now();

    let cases = load_html5lib_cases(Some(limit));
    let mut parsed = 0usize;
    for case in &cases {
        let options = ParserOptions {
            scripting_enabled: case.scripting_enabled,
            ..ParserOptions::default()
        };
        if case.mode == "document" {
            let _ = parse_document_with_options(&case.input, &options);
        } else {
            let context = case.context.as_ref().map(|ctx| {
                let mut context = FragmentContext::new(ctx).with_scripting(case.scripting_enabled);
                context.namespace = match ctx.as_str() {
                    "svg" => Namespace::Svg,
                    "math" | "mi" | "mo" | "mn" | "ms" | "mtext" | "annotation-xml" => {
                        Namespace::MathMl
                    }
                    _ => Namespace::Html,
                };
                context
            });
            let _ = parse_fragment_with_context(&case.input, context.as_ref(), &options);
        }
        parsed += 1;
    }

    let mut large_html = String::from("<!DOCTYPE html><html><head><title>mem</title></head><body>");
    for i in 0..20000usize {
        large_html.push_str("<div class=\"item\"><span>payload ");
        large_html.push_str(&i.to_string());
        large_html.push_str("</span></div>");
    }
    large_html.push_str("</body></html>");
    let _ = parse_document(&large_html);

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let out = json!({
        "generatedAt": format!("{:?}", std::time::SystemTime::now()),
        "casesParsed": parsed,
        "largeHtmlBytes": large_html.len(),
        "elapsedMs": elapsed_ms,
        "allocatorStats": {
            "allocCalls": ALLOC_CALLS.load(Ordering::Relaxed),
            "deallocCalls": DEALLOC_CALLS.load(Ordering::Relaxed),
            "reallocCalls": REALLOC_CALLS.load(Ordering::Relaxed),
            "totalAllocatedBytes": TOTAL_ALLOCATED_BYTES.load(Ordering::Relaxed),
            "peakLiveBytes": PEAK_LIVE_BYTES.load(Ordering::Relaxed),
            "liveBytesAtEnd": CURRENT_LIVE_BYTES.load(Ordering::Relaxed),
            "avgAllocatedBytesPerCase": if parsed == 0 {
                0.0
            } else {
                TOTAL_ALLOCATED_BYTES.load(Ordering::Relaxed) as f64 / parsed as f64
            }
        }
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

fn reset_stats() {
    ALLOC_CALLS.store(0, Ordering::Relaxed);
    DEALLOC_CALLS.store(0, Ordering::Relaxed);
    REALLOC_CALLS.store(0, Ordering::Relaxed);
    TOTAL_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    CURRENT_LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
}

fn load_html5lib_cases(limit: Option<usize>) -> Vec<TreeCase> {
    let dat_dir = Path::new("tests/html5lib/tree-construction");
    let mut entries: Vec<_> = fs::read_dir(dat_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "dat"))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());

    let mut out = Vec::new();
    for entry in entries {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let mut data = String::new();
        let mut fragment_ctx: Option<String> = None;
        let mut scripting_enabled = true;
        let mut section = "";

        for line in content.lines() {
            match line {
                "#data" => {
                    if !data.is_empty() || fragment_ctx.is_some() {
                        out.push(TreeCase {
                            mode: if fragment_ctx.is_some() {
                                "fragment".into()
                            } else {
                                "document".into()
                            },
                            context: fragment_ctx.take(),
                            input: data.trim_end_matches('\n').to_string(),
                            scripting_enabled,
                        });
                        if limit.is_some_and(|max| out.len() >= max) {
                            return out;
                        }
                        data.clear();
                        scripting_enabled = true;
                    }
                    section = "data";
                }
                "#document" => section = "document",
                "#document-fragment" => section = "fragment",
                "#script-off" => {
                    scripting_enabled = false;
                    section = "skip";
                }
                "#script-on" => {
                    scripting_enabled = true;
                    section = "skip";
                }
                "#errors" | "#new-errors" => section = "skip",
                _ if line.starts_with('#') => section = "skip",
                _ => match section {
                    "data" => {
                        data.push_str(line);
                        data.push('\n');
                    }
                    "fragment" => fragment_ctx = Some(line.trim().to_string()),
                    _ => {}
                },
            }
        }

        if !data.is_empty() || fragment_ctx.is_some() {
            out.push(TreeCase {
                mode: if fragment_ctx.is_some() {
                    "fragment".into()
                } else {
                    "document".into()
                },
                context: fragment_ctx,
                input: data.trim_end_matches('\n').to_string(),
                scripting_enabled,
            });
            if limit.is_some_and(|max| out.len() >= max) {
                return out;
            }
        }
    }
    out
}
