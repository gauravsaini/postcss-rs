const originalParse = require('../postcss/lib/parse.js');
const postcssRs = require('./bridge.js');

const testCases = [
  {
    name: 'Basic CSS rule and declaration',
    css: `a {
  color: red;
}`
  },
  {
    name: 'At-rule and comments',
    css: `@media (min-width: 600px) {
  /* inline comment */
  body {
    background: url('img.png');
  }
}`
  },
  {
    name: 'Custom properties (CSS variables)',
    css: `:root {
  --main-bg-color: brown !important;
}`
  },
  {
    name: 'Trailing and free semicolons',
    css: `a { color: red };
p { margin: 0; };`
  },
  {
    name: 'Carriage returns',
    css: ".test {\r\n  font-size: 14px;\r\n}"
  }
];

function cleanAST(node) {
  // Strip node fields that are not relevant to AST structure comparison,
  // or contains circular references (like parent).
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

  if (node.type === 'rule') {
    clean.selector = node.selector;
  } else if (node.type === 'decl') {
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

let failed = false;
console.log('=== PostCSS Rust Parser Validation ===\n');

for (const tc of testCases) {
  console.log(`Running test: "${tc.name}"...`);
  try {
    const originalAST = cleanAST(originalParse(tc.css));
    const rsAST = cleanAST(postcssRs.parse(tc.css));

    const diffs = compareObjects(originalAST, rsAST);
    if (diffs.length === 0) {
      console.log(`✅ PASS: "${tc.name}" matches exactly.`);
    } else {
      console.error(`❌ FAIL: Mismatches found in "${tc.name}":`);
      diffs.forEach(err => console.error(`  - ${err}`));
      failed = true;
    }
  } catch (err) {
    console.error(`💥 CRASH: Error parsing "${tc.name}":`, err.stack);
    failed = true;
  }
  console.log('--------------------------------------');
}

if (failed) {
  console.error('\n❌ Validation FAILED.');
  process.exit(1);
} else {
  console.log('\n🎉 Validation PASSED! All AST structures match exactly.');
}
