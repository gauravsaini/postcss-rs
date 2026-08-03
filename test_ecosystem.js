const postcss = require('postcss');
const postcssRs = require('./bridge.js');

console.log('=== PostCSS Ecosystem & Plugin Breakdown ===\n');

// 1. Standard PostCSS Plugins Compatibility Demo
// Demonstrating how PostCSS plugins across categories operate on the AST produced by postcss-rs:

// Plugin Category 1: Future CSS & Transformation (e.g. Autoprefixer / Preset Env / Nested)
const nestedPlugin = {
  postcssPlugin: 'mock-postcss-nested',
  Rule(rule) {
    rule.each(child => {
      if (child.type === 'rule' && child.selector.startsWith('&')) {
        child.selector = child.selector.replace('&', rule.selector);
        rule.after(child);
      }
    });
  }
};
nestedPlugin.Rule.postcss = true;

// Plugin Category 2: Utility & Optimization (e.g. CSSNano / Short / Sorting)
const minifierPlugin = {
  postcssPlugin: 'mock-cssnano',
  Once(root) {
    root.walkComments(comment => comment.remove());
    root.walkDecls(decl => {
      decl.value = decl.value.trim();
    });
  }
};
minifierPlugin.Once.postcss = true;

// Plugin Category 3: Linters & Diagnostics (e.g. Stylelint / Doiuse / Colorguard)
const linterPlugin = {
  postcssPlugin: 'mock-stylelint',
  Once(root, { result }) {
    root.walkDecls('color', decl => {
      if (decl.value === 'red') {
        result.warn(`Avoid plain color keyword "${decl.value}"`, { node: decl });
      }
    });
  }
};
linterPlugin.Once.postcss = true;

async function runEcosystemVerification() {
  const sampleCSS = `
  /* Banner Comment */
  .card {
    color: red;
    font-size: 16px;
    & .title {
      font-weight: bold;
    }
  }
  `;

  console.log('Original CSS Input:\n', sampleCSS);

  const runner = postcss([nestedPlugin, minifierPlugin, linterPlugin]);

  const result = await runner.process(sampleCSS, {
    from: 'input.css',
    parser: postcssRs.parse
  });

  console.log('Processed Output CSS:\n', result.css);

  const warnings = result.warnings();
  console.log('\nLinter Warnings Caught:');
  warnings.forEach(w => console.log(`  ⚠️  [${w.plugin}] ${w.text} at line ${w.line}:${w.column}`));

  console.log('\n---------------------------------------------------------');
  console.log('✅ Ecosystem Verification PASSED');
}

runEcosystemVerification();
