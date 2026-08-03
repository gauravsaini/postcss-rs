/**
 * test_fixtures.js — AST parity verification against real-world CSS fixtures
 *
 * Parses each fixture file with both JS PostCSS and postcss-rs, then deep-compares
 * the cleaned ASTs to verify structural equivalence.
 */

const fs = require('fs');
const path = require('path');
const originalParse = require('postcss/lib/parse');
const postcssRs = require('./bridge.js');

const FIXTURE_DIR = path.join(__dirname, 'fixtures');

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
  } else if (node.type === 'comment') {
    clean.text = node.text;
  }

  if (node.nodes) {
    clean.nodes = node.nodes.map(cleanAST);
  }

  return clean;
}

function compareObjects(obj1, obj2, path = '') {
  if (typeof obj1 !== typeof obj2) {
    return [`Type mismatch at ${path}: ${typeof obj1} vs ${typeof obj2}`];
  }
  if (obj1 === null || obj2 === null || typeof obj1 !== 'object') {
    if (obj1 !== obj2) {
      return [`Value mismatch at ${path}: ${JSON.stringify(obj1)} vs ${JSON.stringify(obj2)}`];
    }
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

function countNodes(node) {
  let count = 1;
  if (node.nodes) {
    for (const child of node.nodes) {
      count += countNodes(child);
    }
  }
  return count;
}

console.log('=== Real-World CSS Fixture Parity Tests ===\n');

const fixtures = fs.readdirSync(FIXTURE_DIR).filter(f => f.endsWith('.css'));

if (fixtures.length === 0) {
  console.error('No fixture files found in fixtures/ directory');
  process.exit(1);
}

let failed = false;
for (const fixture of fixtures) {
  const filePath = path.join(FIXTURE_DIR, fixture);
  const css = fs.readFileSync(filePath, 'utf8');
  const sizeKB = (Buffer.byteLength(css) / 1024).toFixed(1);

  console.log(`Testing fixture: ${fixture} (${sizeKB} KB)...`);

  try {
    const origRoot = originalParse(css);
    const rsRoot = postcssRs.parse(css);

    const origClean = cleanAST(origRoot);
    const rsClean = cleanAST(rsRoot);

    const origNodeCount = countNodes(origRoot);
    const rsNodeCount = countNodes(rsRoot);

    if (origNodeCount !== rsNodeCount) {
      console.error(`  ❌ Node count mismatch: JS=${origNodeCount}, Rust=${rsNodeCount}`);
      failed = true;
      continue;
    }

    const diffs = compareObjects(origClean, rsClean);
    if (diffs.length === 0) {
      console.log(`  ✅ PASS: ${origNodeCount} nodes match exactly.`);
    } else {
      console.error(`  ❌ FAIL: ${diffs.length} mismatches found (showing first 10):`);
      diffs.slice(0, 10).forEach(d => console.error(`    - ${d}`));
      if (diffs.length > 10) {
        console.error(`    ... and ${diffs.length - 10} more`);
      }
      failed = true;
    }
  } catch (err) {
    console.error(`  💥 CRASH: ${err.message}`);
    failed = true;
  }
  console.log('');
}

if (failed) {
  console.error('❌ Fixture tests FAILED.');
  process.exit(1);
} else {
  console.log('🎉 All fixture parity tests PASSED!');
}
