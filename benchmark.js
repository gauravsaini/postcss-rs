const postcssRs = require('./bridge.js');
const postcssOriginalParse = require('../postcss/lib/parse.js');

console.log("Generating large mock CSS document...");

// Let's generate a large CSS file (approx 1-2 MB) containing comments, rules, decls, and at-rules
let cssParts = [];
for (let i = 0; i < 8000; i++) {
  cssParts.push(`/* Comment block number ${i} */`);
  cssParts.push(`.class-selector-${i} {`);
  cssParts.push(`  color: #${(i % 999).toString().padStart(3, '0')};`);
  cssParts.push(`  font-size: ${10 + (i % 20)}px;`);
  cssParts.push(`  margin-${(i % 2 === 0 ? 'top' : 'bottom')}: ${i % 50}px;`);
  cssParts.push(`  padding: 10px 15px !important;`);
  cssParts.push(`  background-image: url('http://example.com/assets/img_${i}.png');`);
  cssParts.push(`}`);
  if (i % 50 === 0) {
    cssParts.push(`@media screen and (min-width: ${300 + (i % 1000)}px) {`);
    cssParts.push(`  .responsive-${i} { display: none; }`);
    cssParts.push(`}`);
  }
}
const css = cssParts.join('\n');
const cssSizeMB = (Buffer.byteLength(css) / (1024 * 1024)).toFixed(2);
console.log(`Mock CSS size: ${cssSizeMB} MB (${cssParts.length} lines of CSS code)`);

const ITERATIONS = 10;

// Warmup
console.log("\nWarming up parsers...");
for (let i = 0; i < 2; i++) {
  postcssOriginalParse(css);
  postcssRs.parse(css);
  postcssRs.nativeParseCss(css);
}

// Benchmark 1: Original JS PostCSS
console.log("--- Benchmarking Original PostCSS (JS) ---");
let originalTimes = [];
for (let i = 0; i < ITERATIONS; i++) {
  const start = performance.now();
  const root = postcssOriginalParse(css);
  const end = performance.now();
  originalTimes.push(end - start);
  // Verify AST integrity on iteration 0
  if (i === 0) {
    console.log(`  Initial original AST check: Root has ${root.nodes.length} top-level nodes.`);
  }
}
const originalAvg = originalTimes.reduce((a, b) => a + b, 0) / ITERATIONS;
console.log(`Original Avg Parse: ${originalAvg.toFixed(2)} ms`);

// Benchmark 2: PostCSS Rust with JS AST instantiation (drop-in)
console.log("\n--- Benchmarking Rust POC with JS AST Instantiation (drop-in) ---");
let rustJsTimes = [];
for (let i = 0; i < ITERATIONS; i++) {
  const start = performance.now();
  const root = postcssRs.parse(css);
  const end = performance.now();
  rustJsTimes.push(end - start);
  // Verify AST integrity on iteration 0
  if (i === 0) {
    console.log(`  Initial Rust-backed AST check: Root has ${root.nodes.length} top-level nodes.`);
    // Verify first rule and decl
    const firstRule = root.nodes.find(n => n.type === 'rule');
    console.log(`  First selector: "${firstRule.selector}", first decl: "${firstRule.nodes[0].prop}: ${firstRule.nodes[0].value}"`);
  }
}
const rustJsAvg = rustJsTimes.reduce((a, b) => a + b, 0) / ITERATIONS;
console.log(`Rust POC + JS Instantiation Avg Parse: ${rustJsAvg.toFixed(2)} ms`);

// Benchmark 3: Native Rust parsing (raw JSON return)
console.log("\n--- Benchmarking Raw Rust Tokenizer/Parser (Native JSON) ---");
let rustNativeTimes = [];
for (let i = 0; i < ITERATIONS; i++) {
  const start = performance.now();
  const json = postcssRs.nativeParseCss(css);
  const end = performance.now();
  rustNativeTimes.push(end - start);
}
const rustNativeAvg = rustNativeTimes.reduce((a, b) => a + b, 0) / ITERATIONS;
console.log(`Raw Rust Parsing Avg Parse: ${rustNativeAvg.toFixed(2)} ms`);

// Summary
console.log("\n=== PERFORMANCE COMPARISON (Speedup Factor) ===");
console.log(`1. Raw Tokenizer/Parser Speedup (Original JS vs Raw Rust):`);
console.log(`   - Original JS PostCSS:   ${originalAvg.toFixed(2)} ms`);
console.log(`   - Raw Rust Parser:       ${rustNativeAvg.toFixed(2)} ms`);
console.log(`   - Speedup Factor:        **${(originalAvg / rustNativeAvg).toFixed(1)}x faster**`);

console.log(`\n2. Drop-in Replacement Speedup (Original JS vs Rust POC + JS Instantiation):`);
console.log(`   - Original JS PostCSS:   ${originalAvg.toFixed(2)} ms`);
console.log(`   - Rust POC (Drop-in):    ${rustJsAvg.toFixed(2)} ms`);
console.log(`   - Speedup Factor:        **${(originalAvg / rustJsAvg).toFixed(1)}x faster**`);
