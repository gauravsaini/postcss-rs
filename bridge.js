const { parseCssToBuffer } = require('./index.js');

function parse(css, opts, classes) {
  const { Root, Rule, Declaration, AtRule, Comment, Input } = classes || {
    Root: require('postcss/lib/root'),
    Rule: require('postcss/lib/rule'),
    Declaration: require('postcss/lib/declaration'),
    AtRule: require('postcss/lib/at-rule'),
    Comment: require('postcss/lib/comment'),
    Input: require('postcss/lib/input')
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

  const RootProto = Root.prototype;
  const RuleProto = Rule.prototype;
  const DeclProto = Declaration.prototype;
  const AtRuleProto = AtRule.prototype;
  const CommentProto = Comment.prototype;

  function getStr(off, len) {
    return len === 0 ? '' : bigString.substring(off, off + len);
  }

  for (let i = 0; i < totalNodes; i++) {
    const offset = i * 23;
    const nodeType = metadata[offset];
    
    let jsNode;
    if (nodeType === 0) { // Root
      jsNode = Object.create(RootProto);
      jsNode.type = 'root';
      jsNode.raws = {
        after: getStr(metadata[offset + 19], metadata[offset + 20])
      };
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
      }
    } else if (nodeType === 1) { // Rule
      jsNode = Object.create(RuleProto);
      jsNode.type = 'rule';
      jsNode.selector = getStr(metadata[offset + 11], metadata[offset + 12]);
      jsNode.raws = {
        before: getStr(metadata[offset + 15], metadata[offset + 16]),
        between: getStr(metadata[offset + 17], metadata[offset + 18]),
        after: getStr(metadata[offset + 19], metadata[offset + 20])
      };
      const ownSemiLen = metadata[offset + 14];
      if (ownSemiLen > 0) {
        jsNode.raws.ownSemicolon = bigString.substring(metadata[offset + 13], metadata[offset + 13] + ownSemiLen);
      }
      const selRawLen = metadata[offset + 22];
      if (selRawLen > 0) {
        jsNode.raws.selector = {
          value: jsNode.selector,
          raw: bigString.substring(metadata[offset + 21], metadata[offset + 21] + selRawLen)
        };
      }
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
      }
    } else if (nodeType === 2) { // Decl
      jsNode = Object.create(DeclProto);
      jsNode.type = 'decl';
      jsNode.prop = getStr(metadata[offset + 11], metadata[offset + 12]);
      jsNode.value = getStr(metadata[offset + 13], metadata[offset + 14]);
      if (metadata[offset + 8] === 1) {
        jsNode.important = true;
      }
      jsNode.raws = {
        before: getStr(metadata[offset + 15], metadata[offset + 16]),
        between: getStr(metadata[offset + 17], metadata[offset + 18])
      };
      const valRawLen = metadata[offset + 20];
      if (valRawLen > 0) {
        jsNode.raws.value = {
          value: jsNode.value,
          raw: bigString.substring(metadata[offset + 19], metadata[offset + 19] + valRawLen)
        };
      }
      const impRawLen = metadata[offset + 22];
      if (impRawLen > 0) {
        const importantRaw = bigString.substring(metadata[offset + 21], metadata[offset + 21] + impRawLen);
        if (importantRaw !== ' !important') {
          jsNode.raws.important = importantRaw;
        }
      }
    } else if (nodeType === 3) { // AtRule
      jsNode = Object.create(AtRuleProto);
      jsNode.type = 'atrule';
      jsNode.name = getStr(metadata[offset + 11], metadata[offset + 12]);
      jsNode.params = getStr(metadata[offset + 13], metadata[offset + 14]);
      jsNode.raws = {
        before: getStr(metadata[offset + 15], metadata[offset + 16]),
        between: getStr(metadata[offset + 17], metadata[offset + 18]),
        afterName: getStr(metadata[offset + 21], metadata[offset + 22])
      };
      if (metadata[offset + 10] === 1) {
        jsNode.nodes = [];
        jsNode.raws.after = getStr(metadata[offset + 19], metadata[offset + 20]);
      }
    } else if (nodeType === 4) { // Comment
      jsNode = Object.create(CommentProto);
      jsNode.type = 'comment';
      jsNode.text = getStr(metadata[offset + 11], metadata[offset + 12]);
      jsNode.raws = {
        before: getStr(metadata[offset + 15], metadata[offset + 16]),
        left: getStr(metadata[offset + 17], metadata[offset + 18]),
        right: getStr(metadata[offset + 19], metadata[offset + 20])
      };
    }

    // Source mapping
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
        offset: metadata[offset + 3]
      };
    }

    jsNodes[i] = jsNode;

    // Link parent & semicolon
    const parentId = metadata[offset + 1];
    if (parentId >= 0) {
      const parentNode = jsNodes[parentId];
      jsNode.parent = parentNode;
      parentNode.nodes.push(jsNode);
      if (metadata[offset + 9] === 1 && (nodeType === 1 || nodeType === 3)) {
        // Trailing semicolon setting
      }
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