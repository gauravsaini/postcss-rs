const { parseCssToBuffer } = require('./index.js');

function parse(css, opts, classes) {
  const { Root, Rule, Declaration, AtRule, Comment, Input } = classes || {
    Root: require('../postcss/lib/root'),
    Rule: require('../postcss/lib/rule'),
    Declaration: require('../postcss/lib/declaration'),
    AtRule: require('../postcss/lib/at-rule'),
    Comment: require('../postcss/lib/comment'),
    Input: require('../postcss/lib/input')
  };
  const cssStr = css.toString();
  const input = new Input(cssStr, opts);
  let metadata, bigString;
  try {
    const res = parseCssToBuffer(input.css);
    metadata = res.metadata;
    bigString = res.bigString;
  } catch (err) {
    if (err.message.startsWith("Unknown word:")) {
      const parts = err.message.split(":");
      const startOffset = parseInt(parts[1], 10);
      const endOffset = parseInt(parts[2], 10);
      const word = input.css.slice(startOffset, endOffset);
      throw input.error(
        'Unknown word ' + word,
        { offset: startOffset },
        { offset: endOffset }
      );
    }
    if (err.message.startsWith("At-rule without name:")) {
      const parts = err.message.split(":");
      const startOffset = parseInt(parts[1], 10);
      const endOffset = parseInt(parts[2], 10);
      throw input.error(
        'At-rule without name',
        { offset: startOffset },
        { offset: endOffset }
      );
    }
    if (err.message.startsWith("Unexpected }:")) {
      const parts = err.message.split(":");
      const startOffset = parseInt(parts[1], 10);
      const endOffset = parseInt(parts[2], 10);
      throw input.error(
        'Unexpected }',
        { offset: startOffset },
        { offset: endOffset }
      );
    }
    if (err.message.startsWith("Unclosed string:")) {
      const parts = err.message.split(":");
      const offset = parseInt(parts[1], 10);
      throw input.error(
        'Unclosed string',
        offset
      );
    }
    if (err.message.startsWith("Unclosed comment:")) {
      const parts = err.message.split(":");
      const offset = parseInt(parts[1], 10);
      throw input.error(
        'Unclosed comment',
        offset
      );
    }
    if (err.message.startsWith("Unclosed block:")) {
      const parts = err.message.split(":");
      const offset = parseInt(parts[1], 10);
      throw input.error(
        'Unclosed block',
        offset
      );
    }
    if (err.message.startsWith("Unclosed bracket:")) {
      const parts = err.message.split(":");
      const offset = parseInt(parts[1], 10);
      throw input.error(
        'Unclosed bracket',
        offset
      );
    }
    if (err.message.startsWith("Double colon:")) {
      const parts = err.message.split(":");
      const startOffset = parseInt(parts[1], 10);
      const endOffset = parseInt(parts[2], 10);
      throw input.error(
        'Double colon',
        { offset: startOffset },
        { offset: endOffset }
      );
    }
    if (err.message.startsWith("Missed semicolon:")) {
      const parts = err.message.split(":");
      const offset = parseInt(parts[1], 10);
      throw input.error(
        'Missed semicolon',
        offset
      );
    }
    throw err;
  }

  const totalNodes = metadata.length / 23;
  const jsNodes = new Array(totalNodes);

  // Helper to extract a string dynamically
  function getString(nodeIdx, slotIdx) {
    const idx = nodeIdx * 23 + 11 + slotIdx * 2;
    const offset = metadata[idx];
    const length = metadata[idx + 1];
    if (length === 0) return '';
    return bigString.substring(offset, offset + length);
  }

  // Create JS node instances with lazy getters/properties
  for (let i = 0; i < totalNodes; i++) {
    const offset = i * 23;
    const nodeType = metadata[offset];
    
    let jsNode;
    if (nodeType === 0) { // Root
      jsNode = new Root();
      jsNode.raws = {
        after: getString(i, 4)
      };
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
      }
    } else if (nodeType === 1) { // Rule
      jsNode = new Rule();
      jsNode.selector = getString(i, 0);
      jsNode.raws = {
        before: getString(i, 2),
        between: getString(i, 3),
        after: getString(i, 4)
      };
      const ownSemicolon = getString(i, 1);
      if (ownSemicolon) {
        jsNode.raws.ownSemicolon = ownSemicolon;
      }
      const selectorRaw = getString(i, 5);
      if (selectorRaw) {
        jsNode.raws.selector = {
          value: jsNode.selector,
          raw: selectorRaw
        };
      }
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
      }
    } else if (nodeType === 2) { // Decl
      jsNode = new Declaration();
      jsNode.prop = getString(i, 0);
      jsNode.value = getString(i, 1);
      if (metadata[offset + 8] === 1) {
        jsNode.important = true;
      }
      jsNode.raws = {
        before: getString(i, 2),
        between: getString(i, 3)
      };
      const valueRaw = getString(i, 4);
      if (valueRaw) {
        jsNode.raws.value = {
          value: jsNode.value,
          raw: valueRaw
        };
      }
      const importantRaw = getString(i, 5);
      if (importantRaw && importantRaw !== ' !important') {
        jsNode.raws.important = importantRaw;
      }
    } else if (nodeType === 3) { // AtRule
      jsNode = new AtRule();
      jsNode.name = getString(i, 0);
      jsNode.params = getString(i, 1);
      jsNode.raws = {
        before: getString(i, 2),
        between: getString(i, 3),
        afterName: getString(i, 5)
      };
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
        jsNode.raws.after = getString(i, 4);
      }
    } else if (nodeType === 4) { // Comment
      jsNode = new Comment();
      jsNode.text = getString(i, 0);
      jsNode.raws = {
        before: getString(i, 2),
        left: getString(i, 3),
        right: getString(i, 4)
      };
    }

    // Source mapping
    const endOffset = metadata[offset + 3];
    const endLine = metadata[offset + 6];
    jsNode.source = {
      input,
      start: {
        line: metadata[offset + 4],
        column: metadata[offset + 5],
        offset: metadata[offset + 2]
      }
    };
    if (endLine > 0) {
      jsNode.source.end = {
        line: endLine,
        column: metadata[offset + 7],
        offset: endOffset
      };
    }

    jsNodes[i] = jsNode;
  }

  // Link parents and children
  for (let i = 1; i < totalNodes; i++) {
    const parentId = metadata[i * 23 + 1];
    if (parentId >= 0) {
      const parentNode = jsNodes[parentId];
      const childNode = jsNodes[i];
      childNode.parent = parentNode;
      parentNode.nodes.push(childNode);
    }
  }

  // Set semicolon only for rules/atrules/root that actually have children
  for (let i = 0; i < totalNodes; i++) {
    const node = jsNodes[i];
    const offset = i * 23;
    const nodeType = metadata[offset];
    if (nodeType === 0 || nodeType === 1 || nodeType === 3) {
      if (node.nodes && node.nodes.length > 0) {
        node.raws.semicolon = metadata[offset + 9] === 1;
      }
    }
  }

  return jsNodes[0];
}

module.exports = {
  parse,
  nativeParseCss: (css) => parseCssToBuffer(css.toString()).metadata
};