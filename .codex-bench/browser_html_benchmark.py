import json
import os
import time
from pathlib import Path

from selenium import webdriver
from selenium.webdriver.chrome.options import Options as ChromeOptions
from selenium.webdriver.firefox.options import Options as FirefoxOptions


REPO_ROOT = Path(__file__).resolve().parent.parent

BROWSERS = [
    {
        "name": "chrome",
        "path": r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        "kind": "chrome",
    },
    {
        "name": "firefox",
        "path": r"C:\Program Files\Mozilla Firefox\firefox.exe",
        "kind": "firefox",
    },
]

JS_BOOTSTRAP = r"""
window.aceBench = {
  serializeNode(node, depth, out) {
    const indent = '  '.repeat(depth);
    if (node.nodeType === Node.ELEMENT_NODE) {
      const tag = node.localName;
      let ns = '';
      if (node.namespaceURI === 'http://www.w3.org/2000/svg') ns = 'svg ';
      else if (node.namespaceURI === 'http://www.w3.org/1998/Math/MathML') ns = 'math ';
      out.push('| ' + indent + '<' + ns + tag + '>');
      const attrs = Array.from(node.attributes).sort((a, b) => a.name.localeCompare(b.name));
      for (const attr of attrs) out.push('| ' + indent + '  ' + attr.name + '="' + attr.value + '"');
      for (const child of node.childNodes) this.serializeNode(child, depth + 1, out);
      return;
    }
    if (node.nodeType === Node.TEXT_NODE) {
      out.push('| ' + indent + '"' + node.data + '"');
      return;
    }
    if (node.nodeType === Node.COMMENT_NODE) {
      out.push('| ' + indent + '<!-- ' + node.data + ' -->');
    }
  },
  serializeDocumentFromHtml(html) {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');
    const out = [];
    if (doc.doctype) {
      let line = '| <!DOCTYPE ' + (doc.doctype.name || '');
      const publicId = doc.doctype.publicId || '';
      const systemId = doc.doctype.systemId || '';
      if (publicId || systemId) line += ' "' + publicId + '" "' + systemId + '"';
      line += '>';
      out.push(line);
    }
    if (doc.documentElement) this.serializeNode(doc.documentElement, 0, out);
    return out.join('\n');
  },
  serializeFragment(html, contextTag) {
    const doc = document.implementation.createHTMLDocument('');
    const context = doc.createElement(contextTag || 'div');
    context.innerHTML = html;
    const out = [];
    for (const child of context.childNodes) this.serializeNode(child, 0, out);
    return out.join('\n');
  },
  benchmarkParse(html, iterations) {
    const parser = new DOMParser();
    for (let i = 0; i < 3; i++) parser.parseFromString(html, 'text/html');
    const start = performance.now();
    for (let i = 0; i < iterations; i++) parser.parseFromString(html, 'text/html');
    const elapsedMs = performance.now() - start;
    const avgMs = elapsedMs / iterations;
    const throughputMbps = (html.length / 1000000) / (avgMs / 1000);
    return { avgMs, throughputMbps };
  }
};
"""


def build_large_html(target_bytes: int) -> str:
    html = "<!DOCTYPE html><html><head><title>v</title></head><body>"
    i = 0
    while len(html) < target_bytes - 32:
        html += f'<div class="item"><span>content {i}</span></div>'
        i += 1
    html += "</body></html>"
    return html


def parse_dat(content: str):
    cases = []
    data = ""
    fragment_ctx = None
    scripting_enabled = True
    expected_tree = ""
    section = ""
    index = 0

    for line in content.splitlines():
        if line == "#data":
            if data or fragment_ctx or expected_tree:
                cases.append(
                    {
                        "index": index,
                        "data": data.rstrip("\n"),
                        "fragment_ctx": fragment_ctx,
                        "scripting_enabled": scripting_enabled,
                        "expected_tree": expected_tree.rstrip("\n"),
                    }
                )
                data = ""
                fragment_ctx = None
                expected_tree = ""
                scripting_enabled = True
                index += 1
            section = "data"
        elif line in ("#errors", "#new-errors"):
            section = "errors"
        elif line == "#document":
            section = "document"
        elif line == "#document-fragment":
            section = "fragment"
        elif line == "#script-off":
            scripting_enabled = False
            section = "skip"
        elif line == "#script-on":
            scripting_enabled = True
            section = "skip"
        elif line.startswith("#"):
            section = "skip"
        elif section == "data":
            data += line + "\n"
        elif section == "fragment":
            fragment_ctx = line.strip()
        elif section == "document":
            expected_tree += line + "\n"

    if data or fragment_ctx or expected_tree:
        cases.append(
            {
                "index": index,
                "data": data.rstrip("\n"),
                "fragment_ctx": fragment_ctx,
                "scripting_enabled": scripting_enabled,
                "expected_tree": expected_tree.rstrip("\n"),
            }
        )
    return cases


def load_required_tree_cases():
    data = json.loads((REPO_ROOT / "tests" / "ace_html_conformance" / "required.json").read_text("utf8"))
    out = []
    for case in data["cases"]:
        if case["mode"] in ("document", "fragment"):
            out.append(
                {
                    "id": case["case_id"],
                    "mode": case["mode"],
                    "context": case.get("context"),
                    "input": case["input"],
                    "expectedTree": case["expected_tree"].rstrip(),
                }
            )
    return out


def load_html5lib_sample(limit=100):
    dat_dir = REPO_ROOT / "tests" / "html5lib" / "tree-construction"
    out = []
    for file in sorted(dat_dir.glob("*.dat")):
        for case in parse_dat(file.read_text("utf8")):
            out.append(
                {
                    "id": f"{file.name}#{case['index']}",
                    "mode": "fragment" if case["fragment_ctx"] else "document",
                    "context": case["fragment_ctx"],
                    "input": case["data"],
                    "expectedTree": case["expected_tree"].rstrip(),
                }
            )
            if len(out) >= limit:
                return out
    return out


def new_driver(browser):
    if browser["kind"] == "chrome":
        options = ChromeOptions()
        options.binary_location = browser["path"]
        options.add_argument("--headless=new")
        options.add_argument("--disable-gpu")
        options.add_argument("--no-sandbox")
        driver = webdriver.Chrome(options=options)
    else:
        options = FirefoxOptions()
        options.binary_location = browser["path"]
        options.add_argument("-headless")
        driver = webdriver.Firefox(options=options)
    driver.get("data:text/html,<html><body></body></html>")
    driver.execute_script(JS_BOOTSTRAP)
    return driver


def run_conformance(driver, cases):
    passed = 0
    failures = []
    elapsed_values = []
    for case in cases:
        start = time.perf_counter()
        actual = driver.execute_script(
            """
            const mode = arguments[0];
            const html = arguments[1];
            const context = arguments[2];
            if (mode === 'document') return window.aceBench.serializeDocumentFromHtml(html);
            return window.aceBench.serializeFragment(html, context);
            """,
            case["mode"],
            case["input"],
            case["context"],
        )
        elapsed_values.append((time.perf_counter() - start) * 1000)
        if actual.rstrip() == case["expectedTree"]:
            passed += 1
        elif len(failures) < 5:
            failures.append(
                {
                    "id": case["id"],
                    "expected": case["expectedTree"],
                    "actual": actual.rstrip(),
                }
            )
    avg_case_ms = sum(elapsed_values) / len(elapsed_values) if elapsed_values else 0.0
    return {
        "total": len(cases),
        "passed": passed,
        "passRate": (passed / len(cases) * 100.0) if cases else 0.0,
        "avgCaseMs": avg_case_ms,
        "failures": failures,
    }


def run_perf(driver, html):
    return driver.execute_script(
        "return window.aceBench.benchmarkParse(arguments[0], arguments[1]);",
        html,
        20,
    )


def main():
    required_cases = load_required_tree_cases()
    sample_cases = load_html5lib_sample(100)
    perf_html = build_large_html(1_200_000)
    results = {
        "generatedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "datasets": {
            "requiredTreeCases": len(required_cases),
            "html5libTreeSampleCases": len(sample_cases),
            "perfHtmlBytes": len(perf_html),
        },
        "browsers": {},
    }

    for browser in BROWSERS:
        if not os.path.exists(browser["path"]):
            results["browsers"][browser["name"]] = {"available": False, "reason": "not installed"}
            continue
        driver = new_driver(browser)
        try:
            results["browsers"][browser["name"]] = {
                "available": True,
                "requiredTree": run_conformance(driver, required_cases),
                "html5libTreeSample": run_conformance(driver, sample_cases),
                "performance": run_perf(driver, perf_html),
            }
        finally:
            driver.quit()

    out_path = REPO_ROOT / "browser_html_benchmark_results.json"
    out_path.write_text(json.dumps(results, indent=2), "utf8")
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
