const { parseCss, parseCssToBuffer } = require('./index.linux-x64-gnu.node');

function parse(css, opts) {
  const cssStr = css.toString();
  const r = parseCss(cssStr);
  const nodes = JSON.parse(r);
  
  // Convert to PostCSS-compatible object with toJSON() method
  function buildNode(node) {
    const nodeType = node.data.type;
    let result = {
      type: nodeType,
      source: node.source,
      raws: {}
    };
  
    function addRaw(obj, key, value) {
      if (value !== '' && value !== false && value !== undefined && value !== null) {
        obj[key] = value;
      }
    }
  
    if (nodeType === 'root') {
      result.nodes = (node.data.nodes || []).map(childId => buildNode(nodes[childId]));
      addRaw(result.raws, 'after', node.data.raws_after);
      addRaw(result.raws, 'semicolon', node.data.raws_semicolon);
    } else if (nodeType === 'rule') {
      result.selector = node.data.selector;
      if (node.data.raws_selector) {
        result.raws.selector = node.data.raws_selector;
      }
      result.nodes = (node.data.nodes || []).map(childId => buildNode(nodes[childId]));
      addRaw(result.raws, 'before', node.data.raws_before);
      addRaw(result.raws, 'between', node.data.raws_between);
      addRaw(result.raws, 'after', node.data.raws_after);
      addRaw(result.raws, 'semicolon', node.data.raws_semicolon);
    } else if (nodeType === 'decl') {
      result.prop = node.data.prop;
      result.value = node.data.value;
      result.important = node.data.important || false;
      addRaw(result.raws, 'before', node.data.raws_before);
      addRaw(result.raws, 'between', node.data.raws_between);
      addRaw(result.raws, 'after', node.data.raws_after);
      addRaw(result.raws, 'semicolon', node.data.raws_semicolon);
      if (node.data.raws_value && node.data.raws_value !== node.data.value) {
        result.raws.value = node.data.raws_value;
      }
      if (node.data.raws_important) {
        result.raws.important = node.data.raws_important;
      }
    } else if (nodeType === 'atrule') {
      result.name = node.data.name;
      result.params = node.data.params;
      addRaw(result.raws, 'before', node.data.raws_before);
      addRaw(result.raws, 'between', node.data.raws_between);
      addRaw(result.raws, 'after', node.data.raws_after);
      addRaw(result.raws, 'afterName', node.data.raws_after_name);
      addRaw(result.raws, 'semicolon', node.data.raws_semicolon);
      if (node.data.nodes) {
        result.nodes = node.data.nodes.map(childId => buildNode(nodes[childId]));
      }
    } else if (nodeType === 'comment') {
      result.text = node.data.text;
      addRaw(result.raws, 'before', node.data.raws_before);
      addRaw(result.raws, 'left', node.data.raws_left);
      addRaw(result.raws, 'right', node.data.raws_right);
    }
  
    // Add toJSON method
    result.toJSON = function() {
      const json = { ...result };
      delete json.toJSON;
      if (json.nodes) {
        json.nodes = json.nodes.map(n => n.toJSON());
      }
      return json;
    };
  
    return result;
  }
  
  const root = buildNode(nodes[0]);
  return root;
}

module.exports = {
  parse,
  nativeParseCss: (css) => parseCssToBuffer(css.toString()).metadata
};