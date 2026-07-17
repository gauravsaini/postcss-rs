const postcssRs = require('./bridge.js');

const css = `
/* Header styling */
header {
  color: #333;
  margin: 10px 20px;
}

@media screen and (min-width: 900px) {
  article {
    padding: 1rem !important;
  }
}
`;

try {
  console.log("Imported postcssRs:", postcssRs);
  console.log("Parsing CSS using postcss-rs...");
  const root = postcssRs.parse(css);
  console.log("Parsing successful!");
  
  console.log("Parsed AST structure:");
  root.walk(node => {
    console.log(`- Type: ${node.type}, Prop/Selector/Name: ${node.prop || node.selector || node.name || '(none)'}`);
    if (node.type === 'decl') {
      console.log(`  Value: ${node.value}, Important: ${node.important}`);
    }
  });

  console.log("\nStringified output from AST:");
  console.log(root.toString());
  
} catch (e) {
  console.error("Test failed:", e);
}
