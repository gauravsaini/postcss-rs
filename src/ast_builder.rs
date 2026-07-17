use napi_derive::napi;
use napi::{Env, Result, Object};
use crate::{SourcePos, SourceInfo};

#[napi(object)]
pub struct RawNode {
    #[napi(ts_type = "'root' | 'rule' | 'decl' | 'atrule' | 'comment'")]
    pub r#type: String,
    pub source: SourceInfo,
    pub nodes: Option<Vec<RawNode>>,

    // Root
    pub raws_after: Option<String>,
    pub raws_semicolon: Option<bool>,

    // Rule
    pub selector: Option<String>,
    pub raws_selector: Option<String>,
    pub raws_before: Option<String>,
    pub raws_between: Option<String>,

    // Decl
    pub prop: Option<String>,
    pub value: Option<String>,
    pub important: Option<bool>,
    pub raws_value: Option<String>,
    pub raws_important: Option<String>,

    // AtRule
    pub name: Option<String>,
    pub params: Option<String>,
    pub raws_after_name: Option<String>,

    // Comment
    pub text: Option<String>,
    pub raws_left: Option<String>,
    pub raws_right: Option<String>,
}

#[napi]
pub fn build_raw_node(env: Env, node: &RawNode) -> Result<Object> {
    let mut obj = env.create_object()?;

    obj.set("type", env.create_string(&node.r#type)?.into_unknown())?;
    obj.set("source", build_source_info(env, &node.source)?)?;

    if let Some(nodes) = &node.nodes {
        let arr = env.create_array(nodes.len())?;
        for (i, child) in nodes.iter().enumerate() {
            let child_obj = build_raw_node(env, child)?;
            arr.set(i as u32, child_obj)?;
        }
        obj.set("nodes", arr)?;
    }

    // Helper to conditionally set raws
    let set_raw = |obj: &mut Object, key: &str, value: Option<String>| -> Result<()> {
        if let Some(v) = value {
            if !v.is_empty() {
                obj.set(key, env.create_string(&v)?.into_unknown())?;
            }
        }
        Ok(())
    };

    let set_raw_bool = |obj: &mut Object, key: &str, value: Option<bool>| -> Result<()> {
        if let Some(true) = value {
            obj.set(key, env.get_boolean(true)?.into_unknown())?;
        }
        Ok(());
    };

    // Common raws
    set_raw(&mut obj, "after", node.raws_after.clone())?;
    set_raw_bool(&mut obj, "semicolon", node.raws_semicolon)?;

    match node.r#type.as_str() {
        "root" => {
            // Root only has after, semicolon
        }
        "rule" => {
            if let Some(s) = &node.selector {
                obj.set("selector", env.create_string(s)?.into_unknown())?;
            }
            set_raw(&mut obj, "selector", node.raws_selector.clone())?;
            set_raw(&mut obj, "before", node.raws_before.clone())?;
            set_raw(&mut obj, "between", node.raws_between.clone())?;
        }
        "decl" => {
            if let Some(s) = &node.prop {
                obj.set("prop", env.create_string(s)?.into_unknown())?;
            }
            if let Some(s) = &node.value {
                obj.set("value", env.create_string(s)?.into_unknown())?;
            }
            if let Some(imp) = node.important {
                obj.set("important", env.get_boolean(imp)?.into_unknown())?;
            }
            set_raw(&mut obj, "before", node.raws_before.clone())?;
            set_raw(&mut obj, "between", node.raws_between.clone())?;
            set_raw(&mut obj, "value", node.raws_value.clone())?;
            set_raw_bool(
                &mut obj,
                "important",
                node.raws_important.as_ref().map(|s| !s.is_empty()),
            )?;
        }
        "atrule" => {
            if let Some(s) = &node.name {
                obj.set("name", env.create_string(s)?.into_unknown())?;
            }
            if let Some(s) = &node.params {
                obj.set("params", env.create_string(s)?.into_unknown())?;
            }
            set_raw(&mut obj, "before", node.raws_before.clone())?;
            set_raw(&mut obj, "between", node.raws_between.clone())?;
            set_raw(&mut obj, "afterName", node.raws_after_name.clone())?;
        }
        "comment" => {
            if let Some(s) = &node.text {
                obj.set("text", env.create_string(s)?.into_unknown())?;
            }
            set_raw(&mut obj, "before", node.raws_before.clone())?;
            set_raw(&mut obj, "left", node.raws_left.clone())?;
            set_raw(&mut obj, "right", node.raws_right.clone())?;
        }
        _ => {}
    }

    // Add toJSON method
    let to_json = env.create_function_from_closure("toJSON", 0, |ctx| {
        let this = ctx.this?;
        Ok(this)
    })?;
    obj.set("toJSON", to_json)?;

    Ok(obj)
}

fn build_source_info(env: Env, source: &SourceInfo) -> Result<Object> {
    let mut obj = env.create_object()?;
    obj.set("start", build_pos(env, &source.start)?)?;
    if let Some(end) = &source.end {
        obj.set("end", build_pos(env, end)?)?;
    }
    Ok(obj)
}

fn build_pos(env: Env, pos: &SourcePos) -> Result<Object> {
    let mut obj = env.create_object()?;
    obj.set("line", env.create_int32(pos.line as i32)?)?;
    obj.set("column", env.create_int32(pos.column as i32)?)?;
    obj.set("offset", env.create_int64(pos.offset as i64)?)?;
    Ok(obj)
}