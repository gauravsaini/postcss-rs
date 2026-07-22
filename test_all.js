const originalParse = require('postcss/lib/parse');
const postcssRs = require('./bridge.js');

const complexTestCases = [
  // Nesting (PostCSS 8 CSS Nesting syntax)
  { name: 'Direct CSS Nesting', css: 'a {\n  color: red;\n  & b {\n    color: blue;\n  }\n}' },
  { name: 'Implicit CSS Nesting', css: 'a {\n  color: red;\n  b {\n    color: blue;\n  }\n}' },
  
  // Attribute selectors with colons and semicolons inside quotes
  { name: 'Attribute selector with colons & semicolons', css: 'div[data-style="color: red; margin: 0"] { display: block }' },

  // Comments in intricate positions
  { name: 'Comment at end of decl inside rule', css: 'a { color: red /* comment at end */; }' },
  { name: 'Comment after colon before value', css: 'a { color: /* comment */ red; }' },
  { name: 'Multiple consecutive comments', css: '/* 1 */ /* 2 */ a { /* 3 */ /* 4 */ color: red; }' },

  // CSS variables with empty values or whitespace
  { name: 'CSS variable with empty value', css: ':root { --empty: ; }' },
  { name: 'CSS variable with spaces and comments', css: ':root { --var:  /* comment */ ; }' },

  // Deeply nested at-rules
  { name: 'Nested at-rules', css: '@media screen {\n  @supports (display: flex) {\n    a { color: red }\n  }\n}' },
  { name: 'Layer and container at-rules', css: '@layer base {\n  @container (min-width: 500px) {\n    p { font-size: 1rem }\n  }\n}' },

  // Declaration edge cases
  { name: 'Decl value with data URI (slashes, colons, semicolons, commas)', css: 'a { background: url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="); }' },
  { name: 'Decl with calc() and parentheses', css: 'a { width: calc((100% - 20px) / 2); }' }
];

const complexErrorCases = [
  { name: 'Word without colon in block', css: 'a { color }' },
  { name: 'Unclosed at-rule paren', css: '@media (min-width: 600px {' },
  { name: 'Unclosed string in decl value', css: 'a { content: "foo; }' },
  { name: 'Double colon in value', css: 'a { color: red:: blue }' }
];

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

console.log('--- Complex AST Edge Cases ---\n');
for (const tc of complexTestCases) {
  try {
    const origAST = cleanAST(originalParse(tc.css));
    const rsAST = cleanAST(postcssRs.parse(tc.css));
    const diffs = compareObjects(origAST, rsAST);
    if (diffs.length === 0) {
      console.log(`✅ PASS: ${tc.name}`);
    } else {
      console.error(`❌ FAIL: ${tc.name}`);
      diffs.forEach(d => console.error(`   - ${d}`));
    }
  } catch (err) {
    console.error(`💥 CRASH: ${tc.name}`, err.message);
  }
}

console.log('\n--- Complex Syntax Error Cases ---\n');
for (const tc of complexErrorCases) {
  let origError = null;
  let rsError = null;

  try { originalParse(tc.css); } catch (e) { origError = e; }
  try { postcssRs.parse(tc.css); } catch (e) { rsError = e; }

  if (!origError) {
    console.log(`⚠️ SKIP: ${tc.name} (Original PostCSS did not throw)`);
    continue;
  }

  if (!rsError) {
    console.error(`❌ FAIL: ${tc.name} (PostCSS-RS did NOT throw error! Expected: "${origError.message}")`);
    continue;
  }

  const msgMatch = origError.reason === rsError.reason || origError.message.includes(rsError.reason || '');
  const lineMatch = origError.line === rsError.line;
  const colMatch = origError.column === rsError.column;

  if (msgMatch && lineMatch && colMatch) {
    console.log(`✅ PASS: ${tc.name} -> "${rsError.reason || rsError.message}" at L${rsError.line}:C${rsError.column}`);
  } else {
    console.error(`❌ FAIL: ${tc.name}`);
    console.error(`   Original: "${origError.message}" (L${origError.line}:C${origError.column}, offset ${origError.offset})`);
    console.error(`   PostCSS-RS: "${rsError.message}" (L${rsError.line}:C${rsError.column}, offset ${rsError.offset})`);
  }
}
