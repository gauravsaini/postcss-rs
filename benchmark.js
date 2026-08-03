/**
 * benchmark.js — Rigorous benchmark for postcss-rs vs JS PostCSS
 *
 * Features:
 *   - 50 iterations with 5 warmup runs (configurable via BENCH_ITERATIONS)
 *   - GC control when run with --expose-gc
 *   - Statistical reporting: mean, median (p50), p99, min, max, std-dev, CV%
 *   - Multi-input: synthetic, Bootstrap-like, Tailwind-like CSS
 *   - Four benchmark modes including AST walk verification
 *
 * Usage:
 *   node --expose-gc benchmark.js       # recommended (GC control)
 *   node benchmark.js                   # works without --expose-gc (with warning)
 */

const fs = require('fs');
const path = require('path');
const postcssRs = require('./bridge.js');
const postcssOriginalParse = require('postcss/lib/parse');

// ============================================
// Configuration
// ============================================
const ITERATIONS = parseInt(process.env.BENCH_ITERATIONS || '50', 10);
const WARMUP = 5;

const hasGC = typeof global.gc === 'function';
if (!hasGC) {
  console.warn('⚠️  Run with --expose-gc for accurate results: node --expose-gc benchmark.js\n');
}
function forceGC() {
  if (hasGC) global.gc();
}

// ============================================
// Statistical Helpers
// ============================================
function computeStats(times) {
  const sorted = [...times].sort((a, b) => a - b);
  const n = sorted.length;
  const sum = sorted.reduce((a, b) => a + b, 0);
  const mean = sum / n;
  const median = n % 2 === 0 ? (sorted[n / 2 - 1] + sorted[n / 2]) / 2 : sorted[Math.floor(n / 2)];
  const p99 = sorted[Math.min(Math.ceil(n * 0.99) - 1, n - 1)];
  const min = sorted[0];
  const max = sorted[n - 1];
  const variance = sorted.reduce((acc, v) => acc + (v - mean) ** 2, 0) / n;
  const stdDev = Math.sqrt(variance);
  const cv = mean > 0 ? (stdDev / mean) * 100 : 0;

  return { mean, median, p99, min, max, stdDev, cv };
}

function fmtStats(stats) {
  return `mean=${stats.mean.toFixed(2)}ms  median=${stats.median.toFixed(2)}ms  p99=${stats.p99.toFixed(2)}ms  min=${stats.min.toFixed(2)}ms  max=${stats.max.toFixed(2)}ms  σ=${stats.stdDev.toFixed(2)}ms  CV=${stats.cv.toFixed(1)}%`;
}

// ============================================
// AST Walk (proves the AST is usable)
// ============================================
function walkAST(node) {
  let counts = { rules: 0, decls: 0, atrules: 0, comments: 0, total: 1 };
  if (node.type === 'rule') counts.rules++;
  else if (node.type === 'decl') counts.decls++;
  else if (node.type === 'atrule') counts.atrules++;
  else if (node.type === 'comment') counts.comments++;

  if (node.nodes) {
    for (const child of node.nodes) {
      const childCounts = walkAST(child);
      counts.rules += childCounts.rules;
      counts.decls += childCounts.decls;
      counts.atrules += childCounts.atrules;
      counts.comments += childCounts.comments;
      counts.total += childCounts.total;
    }
  }
  return counts;
}

// ============================================
// Benchmark Runner
// ============================================
function runBenchmark(name, fn) {
  // Warmup
  for (let i = 0; i < WARMUP; i++) fn();

  forceGC();

  const times = [];
  for (let i = 0; i < ITERATIONS; i++) {
    const start = performance.now();
    fn();
    const end = performance.now();
    times.push(end - start);
  }
  return computeStats(times);
}

function benchmarkInput(label, css) {
  const sizeKB = (Buffer.byteLength(css) / 1024).toFixed(1);
  const lineCount = css.split('\n').length;
  console.log(`\n${'='.repeat(72)}`);
  console.log(`📄 ${label} (${sizeKB} KB, ${lineCount.toLocaleString()} lines)`);
  console.log('='.repeat(72));

  // 1. Original JS PostCSS
  forceGC();
  const jsStats = runBenchmark('JS PostCSS', () => postcssOriginalParse(css));
  console.log(`\n  JS PostCSS (baseline):\n    ${fmtStats(jsStats)}`);

  // Verify AST on first parse
  const jsRoot = postcssOriginalParse(css);
  const jsCounts = walkAST(jsRoot);

  // 2. postcss-rs Drop-in (Rust parse + JS AST reconstruction)
  forceGC();
  const rsDropInStats = runBenchmark('postcss-rs Drop-in', () => postcssRs.parse(css));
  console.log(`\n  postcss-rs Drop-in:\n    ${fmtStats(rsDropInStats)}`);

  // Verify AST parity
  const rsRoot = postcssRs.parse(css);
  const rsCounts = walkAST(rsRoot);

  if (jsCounts.total !== rsCounts.total) {
    console.error(`    ⚠️  AST node count mismatch! JS=${jsCounts.total}, Rust=${rsCounts.total}`);
  } else {
    console.log(`    ✓ AST verified: ${rsCounts.total} nodes (${rsCounts.rules} rules, ${rsCounts.decls} decls, ${rsCounts.atrules} at-rules, ${rsCounts.comments} comments)`);
  }

  // 3. Rust Parse + Buffer Only (NOT a usable AST)
  forceGC();
  const rsBufferStats = runBenchmark('Rust Parse + Buffer', () => postcssRs.nativeParseCss(css));
  console.log(`\n  Rust Parse + Buffer Only (not a usable AST):\n    ${fmtStats(rsBufferStats)}`);

  // 4. Parse + Full AST Walk (end-to-end "parse & use" benchmark)
  forceGC();
  const jsWalkStats = runBenchmark('JS PostCSS + Walk', () => {
    const root = postcssOriginalParse(css);
    walkAST(root);
  });

  forceGC();
  const rsWalkStats = runBenchmark('postcss-rs + Walk', () => {
    const root = postcssRs.parse(css);
    walkAST(root);
  });

  console.log(`\n  JS PostCSS + AST Walk:\n    ${fmtStats(jsWalkStats)}`);
  console.log(`\n  postcss-rs + AST Walk:\n    ${fmtStats(rsWalkStats)}`);

  // Summary for this input
  const dropinSpeedup = jsStats.median / rsDropInStats.median;
  const bufferSpeedup = jsStats.median / rsBufferStats.median;
  const walkSpeedup = jsWalkStats.median / rsWalkStats.median;

  console.log(`\n  ── Speedup Summary (median-based) ──`);
  console.log(`  Drop-in replacement:   ${dropinSpeedup.toFixed(2)}× faster`);
  console.log(`  Parse + buffer only:   ${bufferSpeedup.toFixed(2)}× faster`);
  console.log(`  Parse + full AST walk: ${walkSpeedup.toFixed(2)}× faster`);

  return { label, jsStats, rsDropInStats, rsBufferStats, jsWalkStats, rsWalkStats, jsCounts };
}

// ============================================
// Generate Synthetic CSS
// ============================================
function generateSyntheticCSS() {
  const parts = [];
  for (let i = 0; i < 8000; i++) {
    parts.push(`/* Comment block number ${i} */`);
    parts.push(`.class-selector-${i} {`);
    parts.push(`  color: #${(i % 999).toString().padStart(3, '0')};`);
    parts.push(`  font-size: ${10 + (i % 20)}px;`);
    parts.push(`  margin-${(i % 2 === 0 ? 'top' : 'bottom')}: ${i % 50}px;`);
    parts.push(`  padding: 10px 15px !important;`);
    parts.push(`  background-image: url('http://example.com/assets/img_${i}.png');`);
    parts.push(`}`);
    if (i % 50 === 0) {
      parts.push(`@media screen and (min-width: ${300 + (i % 1000)}px) {`);
      parts.push(`  .responsive-${i} { display: none; }`);
      parts.push(`}`);
    }
  }
  return parts.join('\n');
}

// ============================================
// Main
// ============================================
console.log(`postcss-rs Benchmark Suite`);
console.log(`Iterations: ${ITERATIONS} | Warmup: ${WARMUP} | GC Control: ${hasGC ? 'YES' : 'NO'}`);

const inputs = [];

// 1. Synthetic
inputs.push({ label: 'Synthetic (generated)', css: generateSyntheticCSS() });

// 2. Bootstrap-like fixture
const bootstrapPath = path.join(__dirname, 'fixtures', 'bootstrap.css');
if (fs.existsSync(bootstrapPath)) {
  inputs.push({ label: 'Bootstrap-like (real-world)', css: fs.readFileSync(bootstrapPath, 'utf8') });
} else {
  console.warn('⚠️  fixtures/bootstrap.css not found, skipping');
}

// 3. Tailwind-like fixture
const tailwindPath = path.join(__dirname, 'fixtures', 'tailwind.css');
if (fs.existsSync(tailwindPath)) {
  inputs.push({ label: 'Tailwind-like (real-world)', css: fs.readFileSync(tailwindPath, 'utf8') });
} else {
  console.warn('⚠️  fixtures/tailwind.css not found, skipping');
}

const allResults = [];
for (const { label, css } of inputs) {
  allResults.push(benchmarkInput(label, css));
}

// ============================================
// Grand Summary
// ============================================
console.log(`\n${'='.repeat(72)}`);
console.log(`📊 GRAND SUMMARY (all inputs, median-based)`);
console.log('='.repeat(72));

const headerFmt = (s, w) => s.padEnd(w);
const numFmt = (n, w) => n.toFixed(2).padStart(w);

console.log(`\n${headerFmt('Input', 30)} ${headerFmt('JS (ms)', 10)} ${headerFmt('Drop-in (ms)', 14)} ${headerFmt('Speedup', 10)} ${headerFmt('Buffer (ms)', 13)} ${headerFmt('Speedup', 10)}`);
console.log('-'.repeat(90));

for (const r of allResults) {
  const dropinX = (r.jsStats.median / r.rsDropInStats.median).toFixed(2) + '×';
  const bufferX = (r.jsStats.median / r.rsBufferStats.median).toFixed(2) + '×';
  console.log(
    `${headerFmt(r.label.slice(0, 29), 30)} ${numFmt(r.jsStats.median, 10)} ${numFmt(r.rsDropInStats.median, 14)} ${headerFmt(dropinX, 10)} ${numFmt(r.rsBufferStats.median, 13)} ${headerFmt(bufferX, 10)}`
  );
}

console.log(`\n✅ Benchmark complete.`);
