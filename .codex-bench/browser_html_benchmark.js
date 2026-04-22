const fs = require('fs');
const path = require('path');
const { chromium, firefox } = require('playwright');

const repoRoot = path.resolve(__dirname, '..');

const browsers = [
  {
    name: 'chrome',
    type: 'chromium',
    executablePath: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  },
  {
    name: 'firefox',
    type: 'firefox',
    executablePath: 'C:\\Program Files\\Mozilla Firefox\\firefox.exe',
  },
];

const requiredCases = JSON.parse(
  fs.readFileSync(path.join(repoRoot, 'tests', 'ace_html_conformance', 'required.json'), 'utf8')
).cases;

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
  return requiredCases
    .filter((testCase) => testCase.mode === 'document' || testCase.mode === 'fragment')
    .map((testCase) => ({
      id: testCase.case_id,
      mode: testCase.mode,
      context: testCase.context || null,
      input: testCase.input,
      expectedTree: testCase.expected_tree.trimEnd(),
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

async function launchBrowser(def) {
  const engine = def.type === 'firefox' ? firefox : chromium;
  return engine.launch({
    headless: true,
    executablePath: def.executablePath,
  });
}

async function benchmarkBrowser(def, cases, largeHtml) {
  if (!fs.existsSync(def.executablePath)) {
    return { available: false, reason: 'browser not installed' };
  }

  const browser = await launchBrowser(def);
  const page = await browser.newPage();

  await page.setContent('<!DOCTYPE html><html><body></body></html>');
  await page.addScriptTag({
    content: `
function aceSerializeNode(node, depth, out) {
  const indent = '  '.repeat(depth);
  if (node.nodeType === Node.ELEMENT_NODE) {
    const tag = node.localName;
    let ns = '';
    if (node.namespaceURI === 'http://www.w3.org/2000/svg') ns = 'svg ';
    else if (node.namespaceURI === 'http://www.w3.org/1998/Math/MathML') ns = 'math ';
    out.push('| ' + indent + '<' + ns + tag + '>');
    const attrs = Array.from(node.attributes).sort((a, b) => a.name.localeCompare(b.name));
    for (const attr of attrs) {
      out.push('| ' + indent + '  ' + attr.name + '="' + attr.value + '"');
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
  return out.join('\\n');
}

function serializeFragment(root) {
  const out = [];
  for (const child of root.childNodes) aceSerializeNode(child, 0, out);
  return out.join('\\n');
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

function benchmarkParse(html, iterations) {
  const parser = new DOMParser();
  for (let i = 0; i < 3; i++) parser.parseFromString(html, 'text/html');
  const start = performance.now();
  for (let i = 0; i < iterations; i++) parser.parseFromString(html, 'text/html');
  const elapsedMs = performance.now() - start;
  const avgMs = elapsedMs / iterations;
  const throughputMbps = (html.length / 1000000) / (avgMs / 1000);
  return { avgMs, throughputMbps };
}
`,
  });

  const conformance = { total: cases.length, passed: 0, failures: [] };
  const timings = [];
  for (const testCase of cases) {
    const start = Date.now();
    const actual = await page.evaluate((payload) => {
      if (payload.mode === 'document') return parseDocumentCase(payload.input);
      return parseFragmentCase(payload.input, payload.context);
    }, testCase);
    const elapsed = Date.now() - start;
    timings.push(elapsed);
    const expected = (testCase.expectedTree || '').trimEnd();
    if (actual.trimEnd() === expected) conformance.passed += 1;
    else if (conformance.failures.length < 10) {
      conformance.failures.push({
        id: testCase.id,
        expected,
        actual: actual.trimEnd(),
      });
    }
  }

  const perf = await page.evaluate(
    ({ html, iterations }) => benchmarkParse(html, iterations),
    { html: largeHtml, iterations: 20 }
  );

  await browser.close();

  return {
    available: true,
    conformance,
    avgCaseMs:
      timings.length === 0 ? 0 : timings.reduce((a, b) => a + b, 0) / timings.length,
    perf,
  };
}

async function main() {
  const requiredTreeCases = buildRequiredTreeCases();
  const html5libSampleCases = loadTreeCases(100);
  const largeHtml = makeLargeHtml(1200000);

  const results = {
    generatedAt: new Date().toISOString(),
    datasets: {
      requiredTreeCases: requiredTreeCases.length,
      html5libTreeSampleCases: html5libSampleCases.length,
      perfHtmlBytes: largeHtml.length,
    },
    browsers: {},
  };

  for (const browserDef of browsers) {
    const requiredResult = await benchmarkBrowser(browserDef, requiredTreeCases, largeHtml);
    let sampleResult = { available: false, reason: requiredResult.reason || 'not run' };
    if (requiredResult.available) {
      sampleResult = await benchmarkBrowser(browserDef, html5libSampleCases, largeHtml);
    }
    results.browsers[browserDef.name] = {
      requiredTree: requiredResult,
      html5libTreeSample: sampleResult,
    };
  }

  fs.writeFileSync(
    path.join(repoRoot, 'browser_html_benchmark_results.json'),
    JSON.stringify(results, null, 2)
  );
  console.log(JSON.stringify(results, null, 2));
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
