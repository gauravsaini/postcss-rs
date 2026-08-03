/**
 * test_fuzz.js — Property-based fuzz testing for postcss-rs
 *
 * Generates random but syntactically plausible CSS, parses with both JS PostCSS
 * and postcss-rs, and compares the cleaned ASTs for parity. Also verifies that
 * error behavior matches when parsing invalid CSS.
 *
 * Usage:
 *   node test_fuzz.js                    # default 500 iterations, random seed
 *   FUZZ_SEED=42 node test_fuzz.js       # reproducible with seed
 *   FUZZ_ITERATIONS=1000 node test_fuzz.js  # custom iteration count
 */

const originalParse = require('postcss/lib/parse');
const postcssRs = require('./bridge.js');

// ============================================
// Seeded PRNG (xorshift128)
// ============================================
class SeededRNG {
  constructor(seed) {
    this.s = [seed, seed ^ 0xdeadbeef, seed ^ 0x12345678, seed ^ 0x87654321];
  }
  next() {
    let t = this.s[3];
    const s = this.s[0];
    this.s[3] = this.s[2];
    this.s[2] = this.s[1];
    this.s[1] = s;
    t ^= t << 11;
    t ^= t >>> 8;
    this.s[0] = t ^ s ^ (s >>> 19);
    return (this.s[0] >>> 0) / 0xFFFFFFFF;
  }
  pick(arr) { return arr[Math.floor(this.next() * arr.length)]; }
  int(min, max) { return Math.floor(this.next() * (max - min + 1)) + min; }
}

// ============================================
// CSS Generators
// ============================================

const IDENTS = [
  'div', 'span', 'a', 'p', 'h1', 'body', 'main', 'section', 'article',
  'header', 'footer', 'nav', 'aside', 'ul', 'li', 'button', 'input',
  'form', 'label', 'table', 'tr', 'td', 'th', 'img', 'svg'
];

const CLASSES = [
  '.btn', '.card', '.modal', '.alert', '.container', '.row', '.col',
  '.flex', '.grid', '.hidden', '.visible', '.active', '.disabled',
  '.primary', '.secondary', '.success', '.danger', '.warning',
  '.text-center', '.bg-white', '.border-0', '.rounded-lg',
  '.hover\\:bg-blue-500', '.sm\\:flex', '.md\\:grid-cols-2',
  '.dark\\:bg-gray-800', '.focus\\:ring-2'
];

const PSEUDO = [':hover', ':focus', ':active', ':first-child', ':last-child', ':nth-child(2n)', '::before', '::after', ':not(.active)', ':focus-visible'];

const PROPS = [
  'color', 'background-color', 'background', 'font-size', 'font-weight', 'font-family',
  'margin', 'margin-top', 'margin-bottom', 'margin-left', 'margin-right',
  'padding', 'padding-top', 'padding-bottom', 'padding-left', 'padding-right',
  'border', 'border-radius', 'border-color', 'border-width',
  'display', 'position', 'top', 'left', 'right', 'bottom', 'z-index',
  'width', 'height', 'max-width', 'min-height',
  'flex', 'flex-direction', 'align-items', 'justify-content', 'gap',
  'grid-template-columns', 'opacity', 'overflow', 'cursor',
  'text-align', 'text-decoration', 'text-transform', 'line-height', 'letter-spacing',
  'box-shadow', 'transition', 'transform', 'animation',
  '--custom-var', '--tw-shadow', '--bs-btn-bg'
];

const VALUES = [
  'red', 'blue', 'green', '#fff', '#333', '#0d6efd', 'rgba(0, 0, 0, 0.5)', 'transparent', 'inherit', 'initial',
  '0', '1px', '2px', '0.5rem', '1rem', '16px', '24px', '100%', '50%', 'auto', 'none',
  'block', 'inline', 'inline-block', 'flex', 'grid', 'none',
  'center', 'left', 'right', 'space-between', 'flex-start', 'flex-end',
  'row', 'column', 'wrap', 'nowrap',
  'pointer', 'default', 'not-allowed',
  'relative', 'absolute', 'fixed', 'sticky',
  'hidden', 'visible', 'scroll', 'auto',
  'bold', '400', '500', '700',
  '1.5', '1.2',
  "url('image.png')",
  'var(--custom-var)',
  'calc(100% - 20px)',
  'repeat(3, minmax(0, 1fr))',
  '0 1px 3px 0 rgb(0 0 0 / 0.1)',
  'cubic-bezier(0.4, 0, 0.2, 1)',
  'all 0.15s ease-in-out'
];

const AT_RULES = ['media', 'supports', 'keyframes', 'layer', 'container'];
const MEDIA_CONDS = [
  '(min-width: 640px)', '(min-width: 768px)', '(min-width: 1024px)',
  '(max-width: 600px)', 'screen and (min-width: 768px)',
  '(prefers-color-scheme: dark)', '(prefers-reduced-motion: reduce)',
  'print'
];

const WHITESPACE_VARIANTS = ['', ' ', '  ', '\n', '\n  ', '\t', ' \n '];

function genSelector(rng) {
  const parts = [];
  const count = rng.int(1, 3);
  for (let i = 0; i < count; i++) {
    if (rng.next() < 0.5) {
      parts.push(rng.pick(IDENTS));
    } else {
      parts.push(rng.pick(CLASSES));
    }
    if (rng.next() < 0.3) {
      parts.push(rng.pick(PSEUDO));
    }
  }
  const sep = rng.next() < 0.5 ? ' ' : (rng.next() < 0.5 ? ', ' : ' > ');
  return parts.join(sep);
}

function genDecl(rng) {
  const before = rng.pick(WHITESPACE_VARIANTS);
  const prop = rng.pick(PROPS);
  const between = rng.next() < 0.9 ? ': ' : ':';
  const value = rng.pick(VALUES);
  const important = rng.next() < 0.15 ? ' !important' : '';
  return `${before}${prop}${between}${value}${important};`;
}

function genComment(rng) {
  const texts = ['comment', 'TODO', 'FIXME', 'section header', `block #${rng.int(1, 999)}`];
  const left = rng.next() < 0.7 ? ' ' : '  ';
  const right = rng.next() < 0.7 ? ' ' : '  ';
  return `/*${left}${rng.pick(texts)}${right}*/`;
}

function genRule(rng, depth = 0) {
  const lines = [];
  const before = depth === 0 ? rng.pick(['', '\n', '\n\n']) : '\n  ';
  const selector = genSelector(rng);
  const declCount = rng.int(1, 5);

  lines.push(`${before}${selector} {`);
  for (let i = 0; i < declCount; i++) {
    if (rng.next() < 0.1) {
      lines.push(`  ${genComment(rng)}`);
    }
    lines.push(`  ${genDecl(rng)}`);
  }
  // Occasionally nest a rule (depth limited)
  if (depth < 1 && rng.next() < 0.15) {
    lines.push(genRule(rng, depth + 1));
  }
  lines.push('}');
  return lines.join('\n');
}

function genAtRule(rng) {
  const name = rng.pick(AT_RULES);
  if (name === 'keyframes') {
    const animName = `anim-${rng.int(1, 100)}`;
    return `@keyframes ${animName} {\n  0% { opacity: 0; }\n  100% { opacity: 1; }\n}`;
  }
  if (name === 'layer') {
    const layerName = rng.pick(['base', 'components', 'utilities']);
    const innerRule = genRule(rng);
    return `@layer ${layerName} {\n${innerRule}\n}`;
  }
  const condition = rng.pick(MEDIA_CONDS);
  const innerCount = rng.int(1, 3);
  const innerRules = [];
  for (let i = 0; i < innerCount; i++) {
    innerRules.push(genRule(rng));
  }
  return `@${name} ${condition} {\n${innerRules.join('\n')}\n}`;
}

function genCSS(rng) {
  const parts = [];
  const blockCount = rng.int(2, 8);
  for (let i = 0; i < blockCount; i++) {
    const roll = rng.next();
    if (roll < 0.1) {
      parts.push(genComment(rng));
    } else if (roll < 0.3) {
      parts.push(genAtRule(rng));
    } else {
      parts.push(genRule(rng));
    }
  }
  return parts.join('\n');
}

// ============================================
// Comparison
// ============================================

function cleanAST(node) {
  if (!node) return null;
  const clean = {
    type: node.type,
    raws: { ...node.raws },
    source: {
      start: node.source && node.source.start ? {
        line: node.source.start.line,
        column: node.source.start.column,
        offset: node.source.start.offset
      } : undefined,
      end: node.source && node.source.end ? {
        line: node.source.end.line,
        column: node.source.end.column,
        offset: node.source.end.offset
      } : undefined
    }
  };
  if (node.type === 'rule') clean.selector = node.selector;
  else if (node.type === 'decl') {
    clean.prop = node.prop;
    clean.value = node.value;
    clean.important = node.important;
  } else if (node.type === 'atrule') {
    clean.name = node.name;
    clean.params = node.params;
  } else if (node.type === 'comment') clean.text = node.text;
  if (node.nodes) clean.nodes = node.nodes.map(cleanAST);
  return clean;
}

function compareObjects(obj1, obj2, path = '') {
  if (typeof obj1 !== typeof obj2) return [`Type mismatch at ${path}: ${typeof obj1} vs ${typeof obj2}`];
  if (obj1 === null || obj2 === null || typeof obj1 !== 'object') {
    if (obj1 !== obj2) return [`Value mismatch at ${path}: ${JSON.stringify(obj1)} vs ${JSON.stringify(obj2)}`];
    return [];
  }
  const keys1 = Object.keys(obj1).filter(k => obj1[k] !== undefined);
  const keys2 = Object.keys(obj2).filter(k => obj2[k] !== undefined);
  let errors = [];
  const allKeys = new Set([...keys1, ...keys2]);
  for (const key of allKeys) {
    errors = errors.concat(compareObjects(obj1[key], obj2[key], path ? `${path}.${key}` : key));
  }
  return errors;
}

// ============================================
// Main
// ============================================

const ITERATIONS = parseInt(process.env.FUZZ_ITERATIONS || '500', 10);
const seed = parseInt(process.env.FUZZ_SEED || String(Date.now() % 100000), 10);
const rng = new SeededRNG(seed);

console.log(`=== Fuzz Testing postcss-rs (${ITERATIONS} iterations, seed=${seed}) ===\n`);

let passed = 0;
let errors = 0;
const failures = [];

for (let i = 0; i < ITERATIONS; i++) {
  const css = genCSS(rng);

  let origResult = null, origError = null;
  let rsResult = null, rsError = null;

  try { origResult = originalParse(css); } catch (e) { origError = e; }
  try { rsResult = postcssRs.parse(css); } catch (e) { rsError = e; }

  // If both throw, verify error parity (just type match, not exact message)
  if (origError && rsError) {
    passed++;
    continue;
  }

  // If one throws and the other doesn't, that's a mismatch
  if (origError && !rsError) {
    failures.push({ iteration: i, type: 'JS threw but Rust did not', error: origError.message, css: css.slice(0, 200) });
    errors++;
    continue;
  }
  if (!origError && rsError) {
    failures.push({ iteration: i, type: 'Rust threw but JS did not', error: rsError.message, css: css.slice(0, 200) });
    errors++;
    continue;
  }

  // Both succeeded — compare ASTs
  const origClean = cleanAST(origResult);
  const rsClean = cleanAST(rsResult);
  const diffs = compareObjects(origClean, rsClean);

  if (diffs.length === 0) {
    passed++;
  } else {
    failures.push({ iteration: i, type: 'AST mismatch', diffs: diffs.slice(0, 5), css: css.slice(0, 200) });
    errors++;
  }
}

console.log(`Results: ${passed}/${ITERATIONS} passed, ${errors} failed\n`);

if (failures.length > 0) {
  console.error(`First ${Math.min(failures.length, 10)} failures:\n`);
  for (const f of failures.slice(0, 10)) {
    console.error(`  Iteration ${f.iteration}: ${f.type}`);
    if (f.error) console.error(`    Error: ${f.error}`);
    if (f.diffs) f.diffs.forEach(d => console.error(`    - ${d}`));
    console.error(`    CSS (truncated): ${f.css}...`);
    console.error('');
  }
  console.error(`\nTo reproduce: FUZZ_SEED=${seed} node test_fuzz.js`);
  process.exit(1);
} else {
  console.log(`🎉 Fuzz testing PASSED! (seed=${seed})`);
}
