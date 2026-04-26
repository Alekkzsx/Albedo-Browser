const fs = require('fs');
const path = require('path');
const { execFileSync, spawnSync } = require('child_process');
const { chromium, firefox } = require('playwright');

const repoRoot = path.resolve(__dirname, '..');

const browserDefs = [
  {
    name: 'chrome',
    engine: 'chromium',
    candidates: [
      process.env.CHROME_BIN,
      '/usr/bin/google-chrome',
      '/usr/bin/google-chrome-stable',
      '/snap/bin/chromium',
      '/usr/bin/chromium',
      '/usr/bin/chromium-browser',
      'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
      'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
      '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    ].filter(Boolean),
  },
  {
    name: 'firefox',
    engine: 'firefox',
    candidates: [
      process.env.FIREFOX_BIN,
      '/usr/bin/firefox',
      '/snap/bin/firefox',
      'C:\\Program Files\\Mozilla Firefox\\firefox.exe',
      'C:\\Program Files (x86)\\Mozilla Firefox\\firefox.exe',
      '/Applications/Firefox.app/Contents/MacOS/firefox',
    ].filter(Boolean),
  },
  {
    name: 'brave',
    engine: 'chromium',
    candidates: [
      process.env.BRAVE_BIN,
      '/usr/bin/brave-browser',
      '/usr/bin/brave',
      '/snap/bin/brave',
      'C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe',
      'C:\\Program Files (x86)\\BraveSoftware\\Brave-Browser\\Application\\brave.exe',
      '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser',
    ].filter(Boolean),
  },
];

function hasCommand(name) {
  const result = spawnSync('bash', ['-lc', `command -v ${name}`], { stdio: 'ignore' });
  return result.status === 0;
}

function firstExisting(paths) {
  for (const p of paths) {
    try {
      if (fs.existsSync(p)) return p;
    } catch (_) {}
  }
  return null;
}

function parseDat(content) {
  const cases = [];
  let data = '';
  let fragmentCtx = null;
  let scriptingEnabled = true;
  let expectedTree = '';
  let section = '';
  let index = 0;

  for (const line of content.split(/\r?\n/)) {
    switch (line) {
      case '#data':
        if (data || fragmentCtx || expectedTree) {
          cases.push({
            index,
            data: data.replace(/\n$/, ''),
            fragmentCtx,
            scriptingEnabled,
            expectedTree: expectedTree.replace(/\n$/, ''),
          });
          data = '';
          fragmentCtx = null;
          expectedTree = '';
          scriptingEnabled = true;
          index += 1;
        }
        section = 'data';
        break;
      case '#errors':
      case '#new-errors':
        section = 'errors';
        break;
      case '#document':
        section = 'document';
        break;
      case '#document-fragment':
        section = 'fragment';
        break;
      case '#script-off':
        scriptingEnabled = false;
        section = 'skip';
        break;
      case '#script-on':
        scriptingEnabled = true;
        section = 'skip';
        break;
      default:
        if (line.startsWith('#')) {
          section = 'skip';
          break;
        }
        if (section === 'data') data += `${line}\n`;
        else if (section === 'fragment') fragmentCtx = line.trim();
        else if (section === 'document') expectedTree += `${line}\n`;
        break;
    }
  }

  if (data || fragmentCtx || expectedTree) {
    cases.push({
      index,
      data: data.replace(/\n$/, ''),
      fragmentCtx,
      scriptingEnabled,
      expectedTree: expectedTree.replace(/\n$/, ''),
    });
  }
  return cases;
}

function loadTreeCases(limit = 100) {
  const dir = path.join(repoRoot, 'tests', 'html5lib', 'tree-construction');
  const files = fs.readdirSync(dir).filter((name) => name.endsWith('.dat')).sort();
  const out = [];
  for (const file of files) {
    const cases = parseDat(fs.readFileSync(path.join(dir, file), 'utf8'));
    for (const testCase of cases) {
      out.push({
        id: `${file}#${testCase.index}`,
        mode: testCase.fragmentCtx ? 'fragment' : 'document',
        context: testCase.fragmentCtx,
        input: testCase.data,
        expectedTree: testCase.expectedTree,
        scriptingEnabled: testCase.scriptingEnabled,
      });
      if (out.length >= limit) return out;
    }
  }
  return out;
}

function buildRequiredTreeCases() {
  const requiredCases = JSON.parse(
    fs.readFileSync(path.join(repoRoot, 'tests', 'ace_html_conformance', 'required.json'), 'utf8')
  ).cases;
  return requiredCases
    .filter((testCase) => testCase.mode === 'document' || testCase.mode === 'fragment')
    .map((testCase) => ({
      id: testCase.case_id,
      mode: testCase.mode,
      context: testCase.context || null,
      input: testCase.input,
      expectedTree: (testCase.expected_tree || '').trimEnd(),
      scriptingEnabled: true,
    }));
}

function makeLargeHtml(targetBytes) {
  let html = '<!DOCTYPE html><html><head><title>v</title></head><body>';
  let i = 0;
  while (html.length < targetBytes - 32) {
    html += `<div class="item"><span>content ${i}</span></div>`;
    i += 1;
  }
  html += '</body></html>';
  return html;
}

function percentile(values, q) {
  if (!values || values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const idx = Math.round((sorted.length - 1) * Math.min(Math.max(q, 0), 1));
  return sorted[idx];
}

function summarize(values) {
  if (!values || values.length === 0) {
    return {
      mean: 0,
      stddev: 0,
      p50: 0,
      p95: 0,
      p99: 0,
      min: 0,
      max: 0,
    };
  }
  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const variance = values.reduce((acc, v) => acc + (v - mean) ** 2, 0) / values.length;
  return {
    mean,
    stddev: Math.sqrt(variance),
    p50: percentile(values, 0.50),
    p95: percentile(values, 0.95),
    p99: percentile(values, 0.99),
    min: Math.min(...values),
    max: Math.max(...values),
  };
}

function normalizeTreeTemplateMarkers(tree) {
  return String(tree || '')
    .split(/\r?\n/)
    .map((line) => {
      if (!line.startsWith('| ')) return line;
      const afterPipe = line.slice(2);
      const indentMatch = afterPipe.match(/^ */);
      const indent = indentMatch ? indentMatch[0] : '';
      const content = afterPipe.slice(indent.length);
      if (content === '<template-content>' || content === 'content') {
        return `| ${indent}content`;
      }
      return line;
    })
    .join('\n');
}

async function launchBrowser(def, executablePath) {
  const engine = def.engine === 'firefox' ? firefox : chromium;

  if (executablePath) {
    try {
      return await engine.launch({
        headless: true,
        executablePath,
      });
    } catch (err) {
      if (def.engine === 'firefox') {
        // Fallback to Playwright bundled firefox for better compatibility.
        return await engine.launch({ headless: true });
      }
      throw err;
    }
  }
  return await engine.launch({ headless: true });
}

async function benchmarkCaseSet(page, cases) {
  const failures = [];
  let passed = 0;
  const timings = [];

  for (const testCase of cases) {
    const start = Date.now();
    const actual = await page.evaluate((payload) => {
      function aceSerializeNode(node, depth, out) {
        const indent = '  '.repeat(depth);
        if (node.nodeType === Node.ELEMENT_NODE) {
          const tag = node.localName;
          let ns = '';
          if (node.namespaceURI === 'http://www.w3.org/2000/svg') ns = 'svg ';
          else if (node.namespaceURI === 'http://www.w3.org/1998/Math/MathML') ns = 'math ';
          out.push('| ' + indent + '<' + ns + tag + '>');
          const attrs = Array.from(node.attributes).sort((a, b) => a.name.localeCompare(b.name));
          for (const attr of attrs) out.push('| ' + indent + '  ' + attr.name + '="' + attr.value + '"');
          if (tag === 'template' && node.content) {
            const templateIndent = '  '.repeat(depth + 1);
            out.push('| ' + templateIndent + '<template-content>');
            for (const child of node.content.childNodes) aceSerializeNode(child, depth + 2, out);
            return;
          }
          for (const child of node.childNodes) aceSerializeNode(child, depth + 1, out);
          return;
        }
        if (node.nodeType === Node.TEXT_NODE) {
          out.push('| ' + indent + '"' + node.data + '"');
          return;
        }
        if (node.nodeType === Node.COMMENT_NODE) {
          out.push('| ' + indent + '<!-- ' + node.data + ' -->');
        }
      }

      function serializeDocument(doc) {
        const out = [];
        if (doc.doctype) {
          let line = '| <!DOCTYPE ' + (doc.doctype.name || '');
          const publicId = doc.doctype.publicId || '';
          const systemId = doc.doctype.systemId || '';
          if (publicId || systemId) line += ' "' + publicId + '" "' + systemId + '"';
          line += '>';
          out.push(line);
        }
        if (doc.documentElement) aceSerializeNode(doc.documentElement, 0, out);
        return out.join('\n');
      }

      function serializeFragment(root) {
        const out = [];
        for (const child of root.childNodes) aceSerializeNode(child, 0, out);
        return out.join('\n');
      }

      function parseDocumentCase(html) {
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, 'text/html');
        return serializeDocument(doc);
      }

      function parseFragmentCase(html, contextTag) {
        const doc = document.implementation.createHTMLDocument('');
        const context = doc.createElement(contextTag || 'div');
        context.innerHTML = html;
        return serializeFragment(context);
      }

      if (payload.mode === 'document') return parseDocumentCase(payload.input);
      return parseFragmentCase(payload.input, payload.context);
    }, testCase);

    timings.push(Date.now() - start);
    const actualTrimmed = (actual || '').trimEnd();
    const expectedTrimmed = (testCase.expectedTree || '').trimEnd();
    const actualNormalized = normalizeTreeTemplateMarkers(actualTrimmed);
    const expectedNormalized = normalizeTreeTemplateMarkers(expectedTrimmed);
    if (actualNormalized === expectedNormalized) {
      passed += 1;
    } else if (failures.length < 20) {
      failures.push({
        id: testCase.id,
        expected: expectedTrimmed,
        actual: actualTrimmed,
        expectedNormalized,
        actualNormalized,
      });
    }
  }

  const timingStats = summarize(timings);
  return {
    total: cases.length,
    passed,
    passRate: cases.length ? (passed * 100) / cases.length : 0,
    avgCaseMs: timingStats.mean,
    p50CaseMs: timingStats.p50,
    p95CaseMs: timingStats.p95,
    p99CaseMs: timingStats.p99,
    failures,
  };
}

async function benchmarkParseDistribution(page, html, warmup, iterationsPerSample, samples) {
  const sampleAvgMs = [];
  const sampleThroughputMbps = [];

  for (let i = 0; i < warmup; i += 1) {
    await page.evaluate((payload) => {
      const parser = new DOMParser();
      parser.parseFromString(payload.html, 'text/html');
    }, { html });
  }

  for (let i = 0; i < samples; i += 1) {
    const perf = await page.evaluate(({ html, iterations }) => {
      const parser = new DOMParser();
      const started = performance.now();
      for (let j = 0; j < iterations; j += 1) parser.parseFromString(html, 'text/html');
      const elapsedMs = performance.now() - started;
      const avgMs = elapsedMs / iterations;
      const throughputMbps = (html.length / 1000000) / (avgMs / 1000);
      return { avgMs, throughputMbps };
    }, { html, iterations: iterationsPerSample });

    sampleAvgMs.push(perf.avgMs);
    sampleThroughputMbps.push(perf.throughputMbps);
  }

  const avgStats = summarize(sampleAvgMs);
  const throughputStats = summarize(sampleThroughputMbps);
  return {
    warmupIterations: warmup,
    iterationsPerSample,
    samples,
    avgMs: avgStats.mean,
    throughputMbps: throughputStats.mean,
    stddevMs: avgStats.stddev,
    p50Ms: avgStats.p50,
    p95Ms: avgStats.p95,
    p99Ms: avgStats.p99,
    minMs: avgStats.min,
    maxMs: avgStats.max,
    throughputP50Mbps: throughputStats.p50,
    throughputP95Mbps: throughputStats.p95,
    throughputP99Mbps: throughputStats.p99,
    throughputMinMbps: throughputStats.min,
    throughputMaxMbps: throughputStats.max,
    sampleAvgMs,
    sampleThroughputMbps,
  };
}

async function benchmarkBrowser(def, executablePath, requiredCases, sampleCases, largeHtml, perfConfig) {
  const browser = await launchBrowser(def, executablePath);
  try {
    const page = await browser.newPage();
    await page.setContent('<!DOCTYPE html><html><body></body></html>');

    const requiredTree = await benchmarkCaseSet(page, requiredCases);
    const html5libTreeSample = await benchmarkCaseSet(page, sampleCases);
    const performance = await benchmarkParseDistribution(
      page,
      largeHtml,
      perfConfig.warmup,
      perfConfig.iterationsPerSample,
      perfConfig.samples
    );

    return {
      available: true,
      executablePath: executablePath || '(playwright-bundled)',
      requiredTree,
      html5libTreeSample,
      performance,
    };
  } finally {
    await browser.close();
  }
}

function parseJsonFromOutput(output) {
  const jsonStart = output.indexOf('{');
  if (jsonStart < 0) throw new Error('JSON output not found');
  return JSON.parse(output.slice(jsonStart));
}

function runAceSuperBenchmark() {
  const cpuPin = process.env.ACE_HTML_CPU_CORE;
  const env = { ...process.env };
  const args = ['run', '--release', '--bin', 'ace_html_super_benchmark'];

  const cmd = [];
  if (cpuPin && process.platform === 'linux' && hasCommand('taskset')) {
    cmd.push('taskset', '-c', String(cpuPin), 'cargo', ...args);
  } else {
    cmd.push('cargo', ...args);
  }

  const shellCmd = cmd.join(' ');
  const output = execFileSync('bash', ['-lc', shellCmd], {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    env,
  });
  return parseJsonFromOutput(output);
}

function fmt(n, digits = 2) {
  if (typeof n !== 'number' || Number.isNaN(n)) return '-';
  return n.toFixed(digits);
}

function fmtInt(n) {
  if (typeof n !== 'number' || Number.isNaN(n)) return '-';
  return Math.round(n).toLocaleString('en-US');
}

function mdSectionForEngine(title, obj) {
  if (!obj || obj.available === false) {
    return `### ${title}\n\n- Status: indisponível (${obj?.reason || 'não encontrado'})\n`;
  }
  const req = obj.requiredTree || {};
  const sample = obj.html5libTreeSample || {};
  const perf = obj.performance || {};

  let md = `### ${title}\n\n`;
  if (obj.executablePath) md += `- Executável: \`${obj.executablePath}\`\n`;
  md += '\n';
  md += '| Métrica | Required Tree | html5lib Sample |\n';
  md += '|---|---:|---:|\n';
  md += `| Total casos | ${req.total ?? '-'} | ${sample.total ?? '-'} |\n`;
  md += `| Passou | ${req.passed ?? '-'} | ${sample.passed ?? '-'} |\n`;
  md += `| Pass rate (%) | ${fmt(req.passRate)} | ${fmt(sample.passRate)} |\n`;
  md += `| Tempo médio por caso (ms) | ${fmt(req.avgCaseMs, 3)} | ${fmt(sample.avgCaseMs, 3)} |\n`;
  md += `| p95 por caso (ms) | ${fmt(req.p95CaseMs, 3)} | ${fmt(sample.p95CaseMs, 3)} |\n\n`;

  md += '| Performance parser | Valor |\n';
  md += '|---|---:|\n';
  md += `| Avg parse (ms) | ${fmt(perf.avgMs, 3)} |\n`;
  md += `| Stddev parse (ms) | ${fmt(perf.stddevMs, 3)} |\n`;
  md += `| p50 parse (ms) | ${fmt(perf.p50Ms, 3)} |\n`;
  md += `| p95 parse (ms) | ${fmt(perf.p95Ms, 3)} |\n`;
  md += `| p99 parse (ms) | ${fmt(perf.p99Ms, 3)} |\n`;
  md += `| Throughput médio (MB/s) | ${fmt(perf.throughputMbps, 3)} |\n\n`;

  const fail = [...(req.failures || []), ...(sample.failures || [])].slice(0, 10);
  if (fail.length) {
    md += 'Falhas amostrais:\n';
    for (const f of fail) md += `- \`${f.id}\`\n`;
    md += '\n';
  }
  return md;
}

function generateMarkdownReport(aceSuper, aceMemory, browsers, datasets) {
  const ace = aceSuper.ace_html || {};
  const perf = ace.performance || {};
  const mem = aceMemory?.allocatorStats || {};
  const lines = [];
  lines.push('# ACE-HTML Verification Benchmarks');
  lines.push('');
  lines.push(`- Generated at: ${new Date().toISOString()}`);
  lines.push(`- Host platform: ${process.platform}`);
  lines.push(`- Node: ${process.version}`);
  lines.push('');
  lines.push('## Dataset');
  lines.push('');
  lines.push(`- Required tree cases: ${datasets.requiredTreeCases}`);
  lines.push(`- html5lib tree cases (ACE full): ${datasets.html5libTreeCases}`);
  lines.push(`- html5lib tree sample cases (browsers): ${datasets.html5libTreeSampleCases}`);
  lines.push(`- Tokenizer subset cases: ${datasets.tokenizerSubsetCases}`);
  lines.push(`- Perf HTML bytes: ${datasets.perfHtmlBytes}`);
  lines.push('');
  lines.push('## Results');
  lines.push('');
  lines.push('### ACE-HTML (Albedo)');
  lines.push('');
  lines.push('| Métrica | Required Tree | html5lib Full | Tokenizer Subset |');
  lines.push('|---|---:|---:|---:|');
  lines.push(`| Total casos | ${ace.requiredTree?.total ?? '-'} | ${ace.html5libTreeFull?.total ?? '-'} | ${ace.tokenizerSubset?.total ?? '-'} |`);
  lines.push(`| Passou | ${ace.requiredTree?.passed ?? '-'} | ${ace.html5libTreeFull?.passed ?? '-'} | ${ace.tokenizerSubset?.passed ?? '-'} |`);
  lines.push(`| Pass rate (%) | ${fmt(ace.requiredTree?.passRate)} | ${fmt(ace.html5libTreeFull?.passRate)} | ${fmt(ace.tokenizerSubset?.passRate)} |`);
  lines.push(`| Tempo médio por caso (ms) | ${fmt(ace.requiredTree?.avgCaseMs, 3)} | ${fmt(ace.html5libTreeFull?.avgCaseMs, 3)} | ${fmt(ace.tokenizerSubset?.avgCaseMs, 3)} |`);
  lines.push('');
  lines.push('| Performance parser | Valor |');
  lines.push('|---|---:|');
  lines.push(`| Avg parse (ms) | ${fmt(perf.avgMs, 3)} |`);
  lines.push(`| Stddev parse (ms) | ${fmt(perf.stddevMs, 3)} |`);
  lines.push(`| p50 parse (ms) | ${fmt(perf.p50Ms, 3)} |`);
  lines.push(`| p95 parse (ms) | ${fmt(perf.p95Ms, 3)} |`);
  lines.push(`| p99 parse (ms) | ${fmt(perf.p99Ms, 3)} |`);
  lines.push(`| Throughput médio (MB/s) | ${fmt(perf.throughputMbps, 3)} |`);
  lines.push('');
  lines.push('### ACE-HTML Memory Profile');
  lines.push('');
  lines.push('| Métrica | Valor |');
  lines.push('|---|---:|');
  lines.push(`| Cases parsed | ${fmtInt(aceMemory?.casesParsed)} |`);
  lines.push(`| Elapsed (ms) | ${fmt(aceMemory?.elapsedMs, 3)} |`);
  lines.push(`| Large HTML bytes | ${fmtInt(aceMemory?.largeHtmlBytes)} |`);
  lines.push(`| Alloc calls | ${fmtInt(mem.allocCalls)} |`);
  lines.push(`| Realloc calls | ${fmtInt(mem.reallocCalls)} |`);
  lines.push(`| Dealloc calls | ${fmtInt(mem.deallocCalls)} |`);
  lines.push(`| Total allocated bytes | ${fmtInt(mem.totalAllocatedBytes)} |`);
  lines.push(`| Peak live bytes | ${fmtInt(mem.peakLiveBytes)} |`);
  lines.push(`| Live bytes at end | ${fmtInt(mem.liveBytesAtEnd)} |`);
  lines.push(`| Avg allocated bytes/case | ${fmt(mem.avgAllocatedBytesPerCase, 1)} |`);
  lines.push('');
  lines.push(mdSectionForEngine('Chrome', browsers.chrome));
  lines.push(mdSectionForEngine('Firefox', browsers.firefox));
  lines.push(mdSectionForEngine('Brave', browsers.brave));
  lines.push('## Quick Comparison');
  lines.push('');
  lines.push('| Engine | Required Tree (%) | html5lib (%) | Throughput (MB/s) |');
  lines.push('|---|---:|---:|---:|');
  lines.push(`| ACE-HTML | ${fmt(ace.requiredTree?.passRate)} | ${fmt(ace.html5libTreeFull?.passRate)} | ${fmt(perf.throughputMbps, 3)} |`);
  lines.push(`| Chrome | ${fmt(browsers.chrome?.requiredTree?.passRate)} | ${fmt(browsers.chrome?.html5libTreeSample?.passRate)} | ${fmt(browsers.chrome?.performance?.throughputMbps, 3)} |`);
  lines.push(`| Firefox | ${fmt(browsers.firefox?.requiredTree?.passRate)} | ${fmt(browsers.firefox?.html5libTreeSample?.passRate)} | ${fmt(browsers.firefox?.performance?.throughputMbps, 3)} |`);
  lines.push(`| Brave | ${fmt(browsers.brave?.requiredTree?.passRate)} | ${fmt(browsers.brave?.html5libTreeSample?.passRate)} | ${fmt(browsers.brave?.performance?.throughputMbps, 3)} |`);
  lines.push('');
  lines.push('## Raw Files');
  lines.push('');
  lines.push('- `ace_html_super_benchmark_results.json`');
  lines.push('- `ace_html_memory_profile_results.json`');
  lines.push('- `browser_html_benchmark_results.json`');
  return `${lines.join('\n')}\n`;
}

async function main() {
  const browserTreeLimit = Number(process.env.BROWSER_TREE_LIMIT || 100);
  const perfHtmlBytes = Number(process.env.BROWSER_PERF_HTML_BYTES || 1200000);
  const perfConfig = {
    warmup: Number(process.env.BROWSER_PERF_WARMUP || 5),
    iterationsPerSample: Number(process.env.BROWSER_PERF_ITER || 20),
    samples: Number(process.env.BROWSER_PERF_SAMPLES || 10),
  };

  const requiredTreeCases = buildRequiredTreeCases();
  const html5libSampleCases = loadTreeCases(browserTreeLimit);
  const largeHtml = makeLargeHtml(perfHtmlBytes);

  console.log('[1/5] Running ACE-HTML super benchmark...');
  const aceSuper = runAceSuperBenchmark();

  console.log('[2/5] Running ACE-HTML memory profile...');
  const memOutput = execFileSync(
    'bash',
    ['-lc', 'cargo run --release --bin ace_html_memory_profile'],
    { cwd: repoRoot, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }
  );
  const aceMemory = parseJsonFromOutput(memOutput);

  console.log('[3/5] Running browser benchmarks (Chrome/Firefox/Brave)...');
  const browserResults = {};
  for (const def of browserDefs) {
    const executablePath = firstExisting(def.candidates);
    try {
      browserResults[def.name] = await benchmarkBrowser(
        def,
        executablePath,
        requiredTreeCases,
        html5libSampleCases,
        largeHtml,
        perfConfig
      );
    } catch (err) {
      browserResults[def.name] = {
        available: false,
        reason: err?.message || String(err),
        executablePath: executablePath || '(not-found)',
      };
    }
  }

  const datasets = {
    requiredTreeCases: aceSuper.datasets?.requiredTreeCases ?? requiredTreeCases.length,
    html5libTreeCases: aceSuper.datasets?.html5libTreeCases ?? 0,
    html5libTreeSampleCases: html5libSampleCases.length,
    tokenizerSubsetCases: aceSuper.datasets?.tokenizerSubsetCases ?? 0,
    perfHtmlBytes: perfHtmlBytes,
  };

  console.log('[4/5] Writing JSON artifacts...');
  const superJson = {
    generatedAt: new Date().toISOString(),
    ...aceSuper,
  };
  const browserJson = {
    generatedAt: new Date().toISOString(),
    datasets,
    perfConfig,
    browsers: browserResults,
  };
  fs.writeFileSync(
    path.join(repoRoot, 'ace_html_super_benchmark_results.json'),
    JSON.stringify(superJson, null, 2)
  );
  fs.writeFileSync(
    path.join(repoRoot, 'ace_html_memory_profile_results.json'),
    JSON.stringify(aceMemory, null, 2)
  );
  fs.writeFileSync(
    path.join(repoRoot, 'browser_html_benchmark_results.json'),
    JSON.stringify(browserJson, null, 2)
  );

  console.log('[5/5] Writing markdown report...');
  const md = generateMarkdownReport(aceSuper, aceMemory, browserResults, datasets);
  fs.writeFileSync(path.join(repoRoot, 'ACE_HTML_VERIFICATION_BENCHMARKS.md'), md);

  console.log('Done. Files generated:');
  console.log('- ace_html_super_benchmark_results.json');
  console.log('- ace_html_memory_profile_results.json');
  console.log('- browser_html_benchmark_results.json');
  console.log('- ACE_HTML_VERIFICATION_BENCHMARKS.md');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
