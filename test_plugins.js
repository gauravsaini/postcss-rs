const postcss = require('postcss');
const postcssRs = require('./bridge.js');

// 1. Define a sample PostCSS plugin (simulating real plugin transformations)
const myPlugin = () => {
  return {
    postcssPlugin: 'test-plugin',
    Once(root) {
      root.walkRules(rule => {
        if (rule.selector === '.btn') {
          rule.append({ prop: 'cursor', value: 'pointer' });
        }
      });
      root.walkDecls('color', decl => {
        if (decl.value === 'red') {
          decl.value = 'blue';
        }
      });
    }
  };
};
myPlugin.postcss = true;

// 2. Test CSS input
const inputCss = `
.btn {
  color: red;
  font-size: 14px;
}
.card {
  color: red;
}
`;

console.log('=== PostCSS Plugin Compatibility Test ===\n');

async function runTest() {
  try {
    // Process CSS using PostCSS runner with custom parser = postcssRs.parse
    const result = await postcss([myPlugin()]).process(inputCss, {
      from: 'input.css',
      parser: postcssRs.parse
    });

    console.log('Transformed CSS Output:\n');
    console.log(result.css);

    const hasCursorPointer = result.css.includes('cursor: pointer');
    const hasColorBlue = result.css.includes('color: blue');

    if (hasCursorPointer && hasColorBlue) {
      console.log('✅ SUCCESS: PostCSS plugins work out-of-the-box with postcss-rs parser!');
    } else {
      console.error('❌ FAILURE: Transformation missing in plugin output.');
    }
  } catch (err) {
    console.error('💥 ERROR during plugin processing:', err);
  }
}

runTest();
