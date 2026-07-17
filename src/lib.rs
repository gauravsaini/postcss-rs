mod ast_builder;
pub use ast_builder::{build_raw_node, RawNode};

use serde::Serialize;
use napi_derive::napi;
use napi::bindgen_prelude::Int32Array;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Space,
    Word,
    AtWord,
    String,
    Comment,
    Brackets,
    Char(char),
}

#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub token_type: TokenType,
    pub content: &'a str,
    pub start: usize,
    pub end: usize,
}

pub fn tokenize(css: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        let code = bytes[pos];
        match code {
            b'\n' | b' ' | b'\t' | b'\r' | b'\x0c' => {
                let start = pos;
                pos += 1;
                while pos < len {
                    match bytes[pos] {
                        b' ' | b'\n' | b'\t' | b'\r' | b'\x0c' => pos += 1,
                        _ => break,
                    }
                }
                tokens.push(Token {
                    token_type: TokenType::Space,
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
            b'[' | b']' | b'{' | b'}' | b':' | b';' | b')' => {
                let start = pos;
                pos += 1;
                tokens.push(Token {
                    token_type: TokenType::Char(code as char),
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
            b'(' => {
                let mut is_url = false;
                if let Some(prev) = tokens.last() {
                    if prev.token_type == TokenType::Word && prev.content.to_lowercase() == "url" {
                        is_url = true;
                    }
                }

                let start = pos;
                pos += 1;
                if is_url {
                    let mut escaped = false;
                    while pos < len {
                        if bytes[pos] == b')' && !escaped {
                            pos += 1;
                            break;
                        }
                        if bytes[pos] == b'\\' {
                            escaped = !escaped;
                        } else {
                            escaped = false;
                        }
                        pos += 1;
                    }
                    tokens.push(Token {
                        token_type: TokenType::Brackets,
                        content: &css[start..pos],
                        start,
                        end: pos,
                    });
                } else {
                    let mut depth = 1;
                    let mut escaped = false;
                    let mut found = false;
                    let mut next = pos;
                    while next < len {
                        let c = bytes[next];
                        if c == b'(' && !escaped {
                            depth += 1;
                        } else if c == b')' && !escaped {
                            depth -= 1;
                            if depth == 0 {
                                found = true;
                                next += 1;
                                break;
                            }
                        } else if c == b'\'' || c == b'"' {
                            let quote = c;
                            next += 1;
                            while next < len {
                                if bytes[next] == quote && !escaped {
                                    next += 1;
                                    break;
                                }
                                if bytes[next] == b'\\' {
                                    escaped = !escaped;
                                } else {
                                    escaped = false;
                                }
                                next += 1;
                            }
                            continue;
                        }
                        if c == b'\\' {
                            escaped = !escaped;
                        } else {
                            escaped = false;
                        }
                        next += 1;
                    }
                    if found {
                        pos = next;
                        tokens.push(Token {
                            token_type: TokenType::Brackets,
                            content: &css[start..pos],
                            start,
                            end: pos,
                        });
                    } else {
                        tokens.push(Token {
                            token_type: TokenType::Char('('),
                            content: "(",
                            start,
                            end: pos,
                        });
                    }
                }
            }
            b'\'' | b'"' => {
                let start = pos;
                let quote = code;
                pos += 1;
                let mut escaped = false;
                while pos < len {
                    if bytes[pos] == quote && !escaped {
                        pos += 1;
                        break;
                    }
                    if bytes[pos] == b'\\' {
                        escaped = !escaped;
                    } else {
                        escaped = false;
                    }
                    pos += 1;
                }
                tokens.push(Token {
                    token_type: TokenType::String,
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
            b'@' => {
                let start = pos;
                pos += 1;
                while pos < len {
                    match bytes[pos] {
                        b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'"' | b'#' | b'\'' | b'(' | b')' | b'/' | b';' | b'[' | b'\\' | b']' | b'{' | b'}' => {
                            break;
                        }
                        _ => pos += 1,
                    }
                }
                tokens.push(Token {
                    token_type: TokenType::AtWord,
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
            b'\\' => {
                let start = pos;
                let mut escape = true;
                while pos + 1 < len && bytes[pos + 1] == b'\\' {
                    pos += 1;
                    escape = !escape;
                }
                if pos + 1 < len {
                    let next_code = bytes[pos + 1];
                    if escape && next_code != b'/' && next_code != b' ' && next_code != b'\n' && next_code != b'\t' && next_code != b'\r' && next_code != b'\x0c' {
                        pos += 1;
                        let c = bytes[pos] as char;
                        if c.is_ascii_hexdigit() {
                            while pos + 1 < len && (bytes[pos + 1] as char).is_ascii_hexdigit() {
                                pos += 1;
                            }
                            if pos + 1 < len && bytes[pos + 1] == b' ' {
                                pos += 1;
                            }
                        }
                    }
                }
                pos += 1;
                tokens.push(Token {
                    token_type: TokenType::Word,
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
            b'/' => {
                if pos + 1 < len && bytes[pos + 1] == b'*' {
                    let start = pos;
                    pos += 2;
                    while pos + 1 < len {
                        if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
                            pos += 2;
                            break;
                        }
                        pos += 1;
                    }
                    tokens.push(Token {
                        token_type: TokenType::Comment,
                        content: &css[start..pos],
                        start,
                        end: pos,
                    });
                } else {
                    let start = pos;
                    pos += 1;
                    while pos < len {
                        match bytes[pos] {
                            b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'!' | b'"' | b'#' | b'\'' | b'(' | b')' | b':' | b';' | b'@' | b'[' | b'\\' | b']' | b'{' | b'}' => {
                                break;
                            }
                            b'/' => {
                                if pos + 1 < len && bytes[pos + 1] == b'*' {
                                    break;
                                }
                                pos += 1;
                            }
                            _ => pos += 1,
                        }
                    }
                    tokens.push(Token {
                        token_type: TokenType::Word,
                        content: &css[start..pos],
                        start,
                        end: pos,
                    });
                }
            }
            _ => {
                let start = pos;
                pos += 1;
                while pos < len {
                    match bytes[pos] {
                        b'\t' | b'\n' | b'\x0c' | b'\r' | b' ' | b'!' | b'"' | b'#' | b'\'' | b'(' | b')' | b':' | b';' | b'@' | b'[' | b'\\' | b']' | b'{' | b'}' => {
                            break;
                        }
                        b'/' => {
                            if pos + 1 < len && bytes[pos + 1] == b'*' {
                                break;
                            }
                            pos += 1;
                        }
                        _ => pos += 1,
                    }
                }
                tokens.push(Token {
                    token_type: TokenType::Word,
                    content: &css[start..pos],
                    start,
                    end: pos,
                });
            }
        }
    }

    tokens
}

#[derive(Debug, Serialize, Clone)]
pub struct SourcePos {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct SourceInfo {
    pub start: SourcePos,
    pub end: Option<SourcePos>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum PostCssNodeData {
    #[serde(rename = "root")]
    Root {
        nodes: Vec<usize>,
        #[serde(rename = "raws_after")]
        raws_after: Option<String>,
        #[serde(rename = "raws_semicolon")]
        raws_semicolon: bool,
    },
    #[serde(rename = "rule")]
    Rule {
        selector: String,
        #[serde(rename = "raws_selector")]
        raws_selector: Option<String>,
        nodes: Vec<usize>,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_between")]
        raws_between: String,
        #[serde(rename = "raws_after")]
        raws_after: Option<String>,
        #[serde(rename = "raws_semicolon")]
        raws_semicolon: bool,
        #[serde(rename = "semicolon")]
        semicolon: bool,
        #[serde(rename = "after")]
        after: Option<String>,
    },
    #[serde(rename = "decl")]
    Decl {
        prop: String,
        value: String,
        #[serde(rename = "raws_value")]
        raws_value: Option<String>,
        important: bool,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_between")]
        raws_between: String,
        #[serde(rename = "raws_important")]
        raws_important: Option<String>,
        #[serde(rename = "raws_semicolon")]
        raws_semicolon: bool,
    },
    #[serde(rename = "atrule")]
    AtRule {
        name: String,
        params: String,
        nodes: Option<Vec<usize>>,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_between")]
        raws_between: String,
        #[serde(rename = "raws_after")]
        raws_after: Option<String>,
        #[serde(rename = "raws_after_name")]
        raws_after_name: String,
        #[serde(rename = "raws_semicolon")]
        raws_semicolon: bool,
    },
    #[serde(rename = "comment")]
    Comment {
        text: String,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_left")]
        raws_left: String,
        #[serde(rename = "raws_right")]
        raws_right: String,
    },
}

#[derive(Debug, Serialize, Clone)]
pub struct PostCssNode {
    pub id: usize,
    pub parent: Option<usize>,
    pub data: PostCssNodeData,
    pub source: SourceInfo,
}

pub struct LineColMap {
    line_starts: Vec<usize>,
}

impl LineColMap {
    pub fn new(s: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in s.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    pub fn get(&self, offset: usize) -> (u32, u32) {
        match self.line_starts.binary_search(&offset) {
            Ok(idx) => ((idx + 1) as u32, 1),
            Err(idx) => {
                let line = idx as u32;
                let start = self.line_starts[idx - 1];
                let col = (offset - start + 1) as u32;
                (line, col)
            }
        }
    }
}

pub struct PostCssParser<'a> {
    css: &'a str,
    tokens: Vec<Token<'a>>,
    token_idx: usize,
    nodes: Vec<PostCssNode>,
    spaces: String,
    semicolon: bool,
    current: usize,
    // Track semicolon at end of decl for semicolon raws
    decl_semicolon: bool,
}

impl<'a> PostCssParser<'a> {
    pub fn new(css: &'a str, map: &LineColMap) -> Self {
        let tokens = tokenize(css);
        let mut parser = Self {
            css,
            tokens,
            token_idx: 0,
            nodes: Vec::new(),
            spaces: String::new(),
            semicolon: false,
            current: 0,
            decl_semicolon: false,
        };

        // Create Root Node
        let root = PostCssNode {
            id: 0,
            parent: None,
            data: PostCssNodeData::Root {
                nodes: Vec::new(),
                raws_after: None,
                raws_semicolon: false,
            },
            source: SourceInfo {
                start: parser.get_pos(0, map),
                end: None,
            },
        };
        parser.nodes.push(root);
        parser
    }

    fn get_pos(&self, offset: usize, map: &LineColMap) -> SourcePos {
        let (line, column) = map.get(offset);
        SourcePos { line, column, offset }
    }

    fn end_of_file(&self) -> bool {
        self.token_idx >= self.tokens.len()
    }

    fn next_token(&mut self) -> Option<&Token<'a>> {
        if self.token_idx < self.tokens.len() {
            let tok = &self.tokens[self.token_idx];
            self.token_idx += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn peek_token(&self) -> Option<&Token<'a>> {
        if self.token_idx < self.tokens.len() {
            Some(&self.tokens[self.token_idx])
        } else {
            None
        }
    }

    fn back(&mut self) {
        if self.token_idx > 0 {
            self.token_idx -= 1;
        }
    }

    fn add_node(&mut self, data: PostCssNodeData, start_offset: usize, map: &LineColMap) -> usize {
        let new_id = self.nodes.len();
        let parent_id = self.current;
        
        let node = PostCssNode {
            id: new_id,
            parent: Some(parent_id),
            data,
            source: SourceInfo {
                start: self.get_pos(start_offset, map),
                end: None,
            }
        };
        self.nodes.push(node);

        match &mut self.nodes[parent_id].data {
            PostCssNodeData::Root { nodes, .. } => nodes.push(new_id),
            PostCssNodeData::Rule { nodes, .. } => nodes.push(new_id),
            PostCssNodeData::AtRule { nodes, .. } => {
                if let Some(nodes_vec) = nodes {
                    nodes_vec.push(new_id);
                } else {
                    *nodes = Some(vec![new_id]);
                }
            }
            _ => {}
        }

        new_id
    }

    fn consume_spaces_and_comments(&mut self) -> String {
        let mut result = String::new();
        while !self.end_of_file() {
            let token = self.next_token().unwrap();
            if token.token_type == TokenType::Space || token.token_type == TokenType::Comment {
                result.push_str(token.content);
            } else {
                self.back();
                break;
            }
        }
        result
    }

    fn comment(&mut self, token: &Token<'a>, map: &LineColMap) {
        let before = std::mem::take(&mut self.spaces);
        let text_raw = &token.content[2..token.content.len() - 2];

        let (text, left, right) = if text_raw.trim().is_empty() {
            ("".to_string(), text_raw.to_string(), "".to_string())
        } else {
            let len = text_raw.len();
            let first_non_ws = text_raw.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            let last_non_ws = text_raw.rfind(|c: char| !c.is_whitespace()).unwrap_or(len - 1);
            (
                text_raw[first_non_ws..=last_non_ws].to_string(),
                text_raw[..first_non_ws].to_string(),
                text_raw[last_non_ws + 1..].to_string()
            )
        };

        let data = PostCssNodeData::Comment {
            text,
            raws_before: before,
            raws_left: left,
            raws_right: right,
        };
        let node_id = self.add_node(data, token.start, map);
        self.nodes[node_id].source.end = Some(self.get_pos(token.end, map));
    }

    fn free_semicolon(&mut self, token: &Token<'a>) {
        self.semicolon = true;
        self.spaces.push_str(token.content);
    }

    fn end(&mut self, token: &Token<'a>, map: &LineColMap) {
        let current_id = self.current;
        let semicolon_val = self.semicolon;
        self.semicolon = false;

        let spaces_taken = std::mem::take(&mut self.spaces);

        let parent_id = match &mut self.nodes[current_id].data {
            PostCssNodeData::Root { raws_after, raws_semicolon, .. } => {
                raws_after.get_or_insert(String::new()).push_str(&spaces_taken);
                *raws_semicolon = semicolon_val;
                None
            }
            PostCssNodeData::Rule { raws_after, raws_semicolon, .. } => {
                raws_after.get_or_insert(String::new()).push_str(&spaces_taken);
                *raws_semicolon = semicolon_val;
                self.nodes[current_id].source.end = Some(self.get_pos(token.end, map));
                self.nodes[current_id].parent
            }
            PostCssNodeData::AtRule { raws_after, raws_semicolon, .. } => {
                raws_after.get_or_insert(String::new()).push_str(&spaces_taken);
                *raws_semicolon = semicolon_val;
                self.nodes[current_id].source.end = Some(self.get_pos(token.end, map));
                self.nodes[current_id].parent
            }
            _ => None,
        };

        if let Some(p_id) = parent_id {
            self.current = p_id;
        }
    }

    fn atrule(&mut self, start_token: &Token<'a>, map: &LineColMap) {
        let name = start_token.content[1..].to_string();
        let before = std::mem::take(&mut self.spaces);

        let mut params_tokens = Vec::new();
        let mut brackets = Vec::new();
        let mut open = false;
        let mut last = false;

        while !self.end_of_file() {
            let token = self.next_token().unwrap().clone();
            let t_type = &token.token_type;

            match t_type {
                TokenType::Char('(') | TokenType::Char('[') => {
                    brackets.push(if *t_type == TokenType::Char('(') { ')' } else { ']' });
                }
                TokenType::Char('{') if !brackets.is_empty() => {
                    brackets.push('}');
                }
                TokenType::Char(c) if !brackets.is_empty() && Some(*c) == brackets.last().copied() => {
                    brackets.pop();
                }
                _ => {}
            }

            if brackets.is_empty() {
                match t_type {
                    TokenType::Char(';') => {
                        self.semicolon = true;
                        break;
                    }
                    TokenType::Char('{') => {
                        open = true;
                        break;
                    }
                    TokenType::Char('}') => {
                        self.back();
                        break;
                    }
                    _ => {
                        params_tokens.push(token);
                    }
                }
            } else {
                params_tokens.push(token);
            }
        }

        if self.end_of_file() && brackets.is_empty() {
            last = true;
        }

        // Extract afterName (spaces after at-rule name)
        let after_name = spaces_and_comments_from_start(&mut params_tokens);
        let between = spaces_and_comments_from_end(&mut params_tokens);
        let params_str: String = params_tokens.iter().map(|t| t.content).collect();

        let data = PostCssNodeData::AtRule {
            name,
            params: params_str,
            nodes: if open { Some(Vec::new()) } else { None },
            raws_before: before,
            raws_between: between,
            raws_after: None,
            raws_after_name: after_name,
            raws_semicolon: false,
        };

        let node_id = self.add_node(data, start_token.start, map);

        if !open {
            let end_pos = if last && !params_tokens.is_empty() {
                let last_tok = params_tokens.last().unwrap();
                self.get_pos(last_tok.end, map)
            } else {
                self.get_pos(start_token.end, map)
            };
            self.nodes[node_id].source.end = Some(end_pos);
        } else {
            self.current = node_id;
        }
    }

    fn other(&mut self, start_token: Token<'a>, map: &LineColMap) {
        let custom_property = start_token.content.starts_with("--");
        let mut tokens = vec![start_token];
        let mut colon = false;
        let mut brackets = Vec::new();
        let mut end = false;

        while !self.end_of_file() {
            let token = self.next_token().unwrap().clone();
            let t_type = &token.token_type;
            tokens.push(token.clone());

            match t_type {
                TokenType::Char('(') | TokenType::Char('[') => {
                    brackets.push(if *t_type == TokenType::Char('(') { ')' } else { ']' });
                }
                TokenType::Char('{') if custom_property && colon => {
                    brackets.push('}');
                }
                TokenType::Char(c) if !brackets.is_empty() && Some(*c) == brackets.last().copied() => {
                    brackets.pop();
                    // If we just closed the custom property block, check for semicolon
                    if brackets.is_empty() && custom_property && colon {
                        eprintln!("DEBUG: Custom prop block closed, calling decl()");
                        // DON'T remove the closing } from tokens - it's part of the value for custom props
                        // Check if next token is semicolon
                        if !self.end_of_file() {
                            let next = self.next_token().unwrap().clone();
                            if next.token_type == TokenType::Char(';') {
                                tokens.push(next);
                                self.semicolon = true;
                            } else {
                                self.back();
                            }
                        }
                        self.decl(tokens, custom_property, map);
                        return;
                    }
                }
                _ => {}
            }

            if brackets.is_empty() {
                match t_type {
                    TokenType::Char(';') => {
                        if colon {
                            self.decl(tokens, custom_property, map);
                            return;
                        } else {
                            break;
                        }
                    }
                    TokenType::Char('{') => {
                        if custom_property && colon {
                            // Start of custom property block value - continue collecting
                            brackets.push('}');
                            // DO NOT return - continue loop to collect tokens inside braces
                        } else {
                            self.rule(tokens, map);
                            return;
                        }
                    }
                    TokenType::Char('}') => {
                        tokens.pop();
                        self.back();
                        // If we're inside a rule (current node is Rule) and brackets are empty,
                        // this } ends the rule, not the declaration
                        let is_in_rule = matches!(&self.nodes[self.current].data, PostCssNodeData::Rule { .. });
                        eprintln!("DEBUG: Found }} with brackets.empty, is_in_rule={}, colon={}, current={}", is_in_rule, colon, self.current);
                        if is_in_rule && colon {
                            // We have a declaration to finalize before the rule ends
                            self.decl(tokens, custom_property, map);
                            return;
                        } else if is_in_rule {
                            // No declaration in progress, just end the rule
                            end = true;
                            return;
                        }
                        end = true;
                        break;
                    }
                    TokenType::Char(':') => {
                        if !colon {
                            colon = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if self.end_of_file() {
            end = true;
        }

        if end && colon {
            if !custom_property {
                while let Some(last) = tokens.last() {
                    if last.token_type == TokenType::Space || last.token_type == TokenType::Comment {
                        tokens.pop();
                        self.back();
                    } else {
                        break;
                    }
                }
            }
            self.decl(tokens, custom_property, map);
        } else {
            self.rule(tokens, map);
        }
    }

    fn decl(&mut self, mut tokens: Vec<Token<'a>>, custom_property: bool, map: &LineColMap) {
        let before = std::mem::take(&mut self.spaces);
        let mut important = false;
        let mut important_raw = None;
        let _decl_semicolon = false; // reset for each decl

        // Save the outer semicolon state
        let outer_semicolon = self.semicolon;

        if let Some(last) = tokens.last() {
            if last.token_type == TokenType::Char(';') {
                self.semicolon = true;
                tokens.pop();
            }
        }

        // Find property name (first word token)
        let mut start_idx = 0;
        while start_idx < tokens.len() && tokens[start_idx].token_type != TokenType::Word {
            start_idx += 1;
        }

        if start_idx >= tokens.len() {
            return;
        }

        // Everything before property name is "before" raw
        let before_extra: String = tokens[0..start_idx].iter().map(|t| t.content).collect();
        let before_total = before + &before_extra;

        // Find property name end (before colon)
        let mut prop_end_idx = start_idx;
        while prop_end_idx < tokens.len() {
            let t_type = &tokens[prop_end_idx].token_type;
            if *t_type == TokenType::Char(':') || *t_type == TokenType::Space || *t_type == TokenType::Comment {
                break;
            }
            prop_end_idx += 1;
        }

        let prop = tokens[start_idx..prop_end_idx].iter().map(|t| t.content).collect::<String>();

        // Between prop and value (after prop name, before colon)
        let between_start_idx = prop_end_idx;
        let mut between_end_idx = prop_end_idx;
        while between_end_idx < tokens.len() {
            let t_type = &tokens[between_end_idx].token_type;
            between_end_idx += 1;
            if *t_type == TokenType::Char(':') {
                break;
            }
        }

        let mut between: String = tokens[between_start_idx..between_end_idx]
            .iter()
            .map(|t| t.content)
            .collect();

        // Value tokens
        let mut val_tokens = tokens[between_end_idx..].to_vec();

        // Strip leading spaces/comments from value (part of between)
        let mut first_spaces = String::new();
        let mut val_start = 0;
        while val_start < val_tokens.len() {
            let t = &val_tokens[val_start];
            if t.token_type == TokenType::Space || t.token_type == TokenType::Comment {
                first_spaces.push_str(t.content);
                val_start += 1;
            } else {
                break;
            }
        }

        let has_word = val_tokens[val_start..].iter().any(|t| t.token_type != TokenType::Space && t.token_type != TokenType::Comment);
        if has_word {
            between.push_str(&first_spaces);
            val_tokens.drain(0..val_start);
        }

        // Check for !important
        if let Some(pos) = val_tokens.iter().rposition(|t| t.content.to_lowercase() == "!important") {
            important = true;
            let mut imp_raw_tokens = val_tokens[..pos].to_vec();
            let imp_raw_spaces = spaces_from_end(&mut imp_raw_tokens);
            important_raw = Some(imp_raw_spaces + "!important");
            val_tokens.truncate(pos);
        } else if let Some(pos) = val_tokens.iter().rposition(|t| t.content.to_lowercase() == "important") {
            let mut found_excl = false;
            let mut search_idx = pos;
            let mut str_collected = String::new();
            while search_idx > 0 {
                search_idx -= 1;
                let t = &val_tokens[search_idx];
                str_collected = t.content.to_string() + &str_collected;
                if t.content == "!" {
                    found_excl = true;
                    break;
                }
            }
            if found_excl {
                important = true;
                important_raw = Some(str_collected + "important");
                val_tokens.truncate(search_idx);
            }
        }

        // Build raw value (preserving all tokens including comments)
        let raw_value: String = val_tokens.iter().map(|t| t.content).collect();

        let data = PostCssNodeData::Decl {
            prop,
            value: raw_value.trim().to_string(),
            raws_value: if raw_value != raw_value.trim() { Some(raw_value) } else { None },
            important,
            raws_before: before_total,
            raws_between: between,
            raws_important: important_raw,
            raws_semicolon: self.semicolon,
        };

        let node_id = self.add_node(data, tokens[start_idx].start, map);
        let end_offset = if let Some(last) = tokens.last() { last.end } else { tokens[start_idx].end };
        self.nodes[node_id].source.end = Some(self.get_pos(end_offset, map));
        // Restore outer semicolon state for parent rule/root
        self.semicolon = outer_semicolon;
    }

    fn rule(&mut self, mut tokens: Vec<Token<'a>>, map: &LineColMap) {
        if let Some(last) = tokens.last() {
            if last.token_type == TokenType::Char('{') {
                tokens.pop();
            }
        }

        let before = std::mem::take(&mut self.spaces);
        let between = spaces_and_comments_from_end(&mut tokens);
        
        // Build selector preserving all tokens (including comments)
        let selector_raw: String = tokens.iter().map(|t| t.content).collect();
        let selector: String = tokens.iter().filter(|t| t.token_type != TokenType::Comment).map(|t| t.content).collect();
        
        // Store raw selector if it has comments
        let raws_selector = if selector_raw != selector { Some(selector_raw) } else { None };

        let start_offset = tokens.first().map(|t| t.start).unwrap_or(0);

        let data = PostCssNodeData::Rule {
            selector,
            raws_selector,
            nodes: Vec::new(),
            raws_before: before,
            raws_between: between,
            raws_after: None,
            raws_semicolon: false,
            semicolon: false,
            after: None,
        };

        let node_id = self.add_node(data, start_offset, map);
        self.current = node_id;
    }

    pub fn parse(&mut self, map: &LineColMap) {
        while !self.end_of_file() {
            let token = self.next_token().unwrap().clone();
            match &token.token_type {
                TokenType::Space => {
                    self.spaces.push_str(token.content);
                }
                TokenType::Char(';') => {
                    self.free_semicolon(&token);
                }
                TokenType::Char('}') => {
                    self.end(&token, map);
                }
                TokenType::Comment => {
                    self.comment(&token, map);
                }
                TokenType::AtWord => {
                    self.atrule(&token, map);
                }
                TokenType::Char('{') => {
                    self.rule(vec![token], map);
                }
                _ => {
                    self.other(token, map);
                }
            }
        }

        let semicolon_val = self.semicolon;
        let spaces_taken = std::mem::take(&mut self.spaces);

        match &mut self.nodes[0].data {
            PostCssNodeData::Root { raws_after, raws_semicolon, .. } => {
                *raws_after = Some(spaces_taken);
                *raws_semicolon = semicolon_val;
            }
            _ => {}
        }
        self.nodes[0].source.end = Some(self.get_pos(self.css.len(), map));
    }
}

fn spaces_and_comments_from_end(tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    while let Some(last) = tokens.last() {
        if last.token_type == TokenType::Space || last.token_type == TokenType::Comment {
            spaces = last.content.to_string() + &spaces;
            tokens.pop();
        } else {
            break;
        }
    }
    spaces
}

fn spaces_and_comments_from_start(tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    let mut idx = 0;
    while idx < tokens.len() {
        let t = &tokens[idx];
        if t.token_type == TokenType::Space || t.token_type == TokenType::Comment {
            spaces.push_str(t.content);
            idx += 1;
        } else {
            break;
        }
    }
    if idx > 0 {
        tokens.drain(0..idx);
    }
    spaces
}

fn spaces_from_end(tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    while let Some(last) = tokens.last() {
        if last.token_type == TokenType::Space {
            spaces = last.content.to_string() + &spaces;
            tokens.pop();
        } else {
            break;
        }
    }
    spaces
}

#[napi]
pub fn parse_css(css: String) -> String {
    let map = LineColMap::new(&css);
    let mut parser = PostCssParser::new(&css, &map);
    parser.parse(&map);
    serde_json::to_string(&parser.nodes).unwrap()
}

#[napi(object)]
pub struct AstBuffer {
    pub metadata: Int32Array,
    pub big_string: String,
}

pub struct AstBufferBuilder {
    pub metadata: Vec<i32>,
    pub big_string: String,
}

impl AstBufferBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Vec::new(),
            big_string: String::new(),
        }
    }

    pub fn push_string(&mut self, s: &str) -> (i32, i32) {
        if s.is_empty() {
            return (0, 0);
        }
        let offset = self.big_string.len() as i32;
        let length = s.len() as i32;
        self.big_string.push_str(s);
        (offset, length)
    }

    pub fn add_node(
        &mut self,
        node_type: i32,
        parent_id: i32,
        start_offset: i32,
        end_offset: i32,
        start_line: i32,
        start_column: i32,
        end_line: i32,
        end_column: i32,
        important: i32,
        semicolon: i32,
        has_nodes: i32,
        slots: &[(i32, i32); 6],
    ) {
        self.metadata.push(node_type);
        self.metadata.push(parent_id);
        self.metadata.push(start_offset);
        self.metadata.push(end_offset);
        self.metadata.push(start_line);
        self.metadata.push(start_column);
        self.metadata.push(end_line);
        self.metadata.push(end_column);
        self.metadata.push(important);
        self.metadata.push(semicolon);
        self.metadata.push(has_nodes);
        for i in 0..6 {
            self.metadata.push(slots[i].0);
            self.metadata.push(slots[i].1);
        }
    }
}

pub fn serialize_to_buffer(nodes: &[PostCssNode]) -> AstBuffer {
    let mut builder = AstBufferBuilder::new();

    for node in nodes {
        let node_type = match &node.data {
            PostCssNodeData::Root { .. } => 0,
            PostCssNodeData::Rule { .. } => 1,
            PostCssNodeData::Decl { .. } => 2,
            PostCssNodeData::AtRule { .. } => 3,
            PostCssNodeData::Comment { .. } => 4,
        };

        let parent_id = match node.parent {
            Some(pid) => pid as i32,
            None => -1,
        };

        let start_offset = node.source.start.offset as i32;
        let end_offset = match &node.source.end {
            Some(ep) => ep.offset as i32,
            None => 0,
        };
        let start_line = node.source.start.line as i32;
        let start_column = node.source.start.column as i32;
        let (end_line, end_column) = match &node.source.end {
            Some(ep) => (ep.line as i32, ep.column as i32),
            None => (0, 0),
        };

        let mut important = 0;
        let mut semicolon = 0;
        let mut has_nodes = 0;

        let mut slots = [(0, 0); 6];

        match &node.data {
            PostCssNodeData::Root { raws_after, raws_semicolon, nodes } => {
                slots[4] = builder.push_string(raws_after.as_deref().unwrap_or(""));
                if *raws_semicolon { semicolon = 1; }
                if !nodes.is_empty() { has_nodes = 1; }
            }
            PostCssNodeData::Rule { selector, raws_selector, raws_before, raws_between, raws_after, raws_semicolon, nodes, .. } => {
                slots[0] = builder.push_string(selector);
                slots[1] = builder.push_string(raws_selector.as_deref().unwrap_or(""));
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_between);
                slots[4] = builder.push_string(raws_after.as_deref().unwrap_or(""));
                if *raws_semicolon { semicolon = 1; }
                if !nodes.is_empty() { has_nodes = 1; }
            }
            PostCssNodeData::Decl { prop, value, raws_value, important: imp, raws_before, raws_between, raws_important, raws_semicolon } => {
                slots[0] = builder.push_string(prop);
                slots[1] = builder.push_string(value);
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_between);
                slots[4] = builder.push_string(raws_value.as_deref().unwrap_or(value));
                slots[5] = builder.push_string(raws_important.as_deref().unwrap_or(""));
                if *imp { important = 1; }
                if *raws_semicolon { semicolon = 1; }
            }
            PostCssNodeData::AtRule { name, params, raws_before, raws_between, raws_after, raws_after_name, raws_semicolon, nodes } => {
                slots[0] = builder.push_string(name);
                slots[1] = builder.push_string(params);
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_between);
                slots[4] = builder.push_string(raws_after.as_deref().unwrap_or(""));
                slots[5] = builder.push_string(raws_after_name);
                if *raws_semicolon { semicolon = 1; }
                if nodes.is_some() { has_nodes = 1; }
            }
            PostCssNodeData::Comment { text, raws_before, raws_left, raws_right } => {
                slots[0] = builder.push_string(text);
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_left);
                slots[4] = builder.push_string(raws_right);
            }
        }

        builder.add_node(
            node_type,
            parent_id,
            start_offset,
            end_offset,
            start_line,
            start_column,
            end_line,
            end_column,
            important,
            semicolon,
            has_nodes,
            &slots,
        );
    #[napi]
    pub fn parse_css(css: String) -> String {
        let map = LineColMap::new(&css);
        let mut parser = PostCssParser::new(&css, &map);
        parser.parse(&map);
        serde_json::to_string(&parser.nodes).unwrap()
    }

    #[napi]
    pub fn parse_css_to_buffer(css: String) -> AstBuffer {
        let map = LineColMap::new(&css);
        let mut parser = PostCssParser::new(&css, &map);
        parser.parse(&map);
        serialize_to_buffer(&parser.nodes)
    }

    #[napi]
    pub fn parse_css_napi(env: Env, css: String) -> Result<Object> {
        use napi::{Object, Result};
        let map = LineColMap::new(&css);
        let mut parser = PostCssParser::new(&css, &map);
        parser.parse(&map);

        // Convert parser nodes to RawNode format for napi AST building
        let raw_nodes = convert_to_raw_nodes(
            &parser.nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
            &parser.nodes,
        );
        build_raw_node(env, &raw_nodes[0])
        }

        fn convert_to_raw_nodes(node_ids: &[usize], all_nodes: &[PostCssNode]) -> Vec<RawNode> {
    node_ids.iter().map(|id| convert_node(&all_nodes[*id], all_nodes)).collect()
}

fn convert_node(node: &PostCssNode, all_nodes: &[PostCssNode]) -> RawNode {
    let r#type = match &node.data {
        PostCssNodeData::Root { .. } => "root",
        PostCssNodeData::Rule { .. } => "rule",
        PostCssNodeData::Decl { .. } => "decl",
        PostCssNodeData::AtRule { .. } => "atrule",
        PostCssNodeData::Comment { .. } => "comment",
    }.to_string();

    let source = SourceInfo {
        start: SourcePos {
            line: node.source.start.line as u32,
            column: node.source.start.column as u32,
            offset: node.source.start.offset as usize,
        },
        end: node.source.end.as_ref().map(|ep| SourcePos {
            line: ep.line as u32,
            column: ep.column as u32,
            offset: ep.offset as usize,
        }),
    };

    let nodes = if let PostCssNodeData::Root { nodes, .. } = &node.data {
        Some(convert_to_raw_nodes(nodes, all_nodes))
    } else if let PostCssNodeData::Rule { nodes, .. } = &node.data {
        Some(convert_to_raw_nodes(nodes, all_nodes))
    } else if let PostCssNodeData::AtRule { nodes, .. } = &node.data {
        nodes.as_ref().map(|n| convert_to_raw_nodes(n, all_nodes))
    } else {
        None
    };

    let mut raw = RawNode {
        r#type,
        source,
        nodes,
        // Root
        raws_after: None,
        raws_semicolon: None,
        // Rule
        selector: None,
        raws_selector: None,
        raws_before: None,
        raws_between: None,
        // Decl
        prop: None,
        value: None,
        important: None,
        raws_value: None,
        raws_important: None,
        // AtRule
        name: None,
        params: None,
        raws_after_name: None,
        // Comment
        text: None,
        raws_left: None,
        raws_right: None,
    };

    match &node.data {
        PostCssNodeData::Root { raws_after, raws_semicolon, .. } => {
            raw.raws_after = raws_after.clone();
            raw.raws_semicolon = Some(*raws_semicolon);
        }
        PostCssNodeData::Rule { selector, raws_selector, raws_before, raws_between, raws_after, raws_semicolon, .. } => {
            raw.selector = Some(selector.clone());
            raw.raws_selector = raws_selector.clone();
            raw.raws_before = Some(raws_before.clone());
            raw.raws_between = Some(raws_between.clone());
            raw.raws_after = raws_after.clone();
            raw.raws_semicolon = Some(*raws_semicolon);
        }
        PostCssNodeData::Decl { prop, value, raws_value, important, raws_before, raws_between, raws_important, raws_semicolon } => {
            raw.prop = Some(prop.clone());
            raw.value = Some(value.clone());
            raw.raws_value = raws_value.clone();
            raw.important = Some(*important);
            raw.raws_before = Some(raws_before.clone());
            raw.raws_between = Some(raws_between.clone());
            raw.raws_important = raws_important.clone();
            raw.raws_semicolon = Some(*raws_semicolon);
        }
        PostCssNodeData::AtRule { name, params, raws_before, raws_between, raws_after, raws_after_name, raws_semicolon, .. } => {
            raw.name = Some(name.clone());
            raw.params = Some(params.clone());
            raw.raws_before = Some(raws_before.clone());
            raw.raws_between = Some(raws_between.clone());
            raw.raws_after = raws_after.clone();
            raw.raws_after_name = Some(raws_after_name.clone());
            raw.raws_semicolon = Some(*raws_semicolon);
        }
        PostCssNodeData::Comment { text, raws_before, raws_left, raws_right } => {
            raw.text = Some(text.clone());
            raw.raws_before = Some(raws_before.clone());
            raw.raws_left = Some(raws_left.clone());
            raw.raws_right = Some(raws_right.clone());
        }
    }

    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn profile_parse() {
        let mut css_parts = Vec::new();
        for i in 0..8000 {
            css_parts.push(format!("/* Comment block number {} */", i));
            css_parts.push(format!(".class-selector-{} {{", i));
            css_parts.push(format!("  color: #{};", &(i % 999).to_string()));
            css_parts.push(format!("  font-size: {}px;", 10 + (i % 20)));
            css_parts.push(format!("  margin-{}: {}px;", if i % 2 == 0 { "top" } else { "bottom" }, i % 50));
            css_parts.push(format!("  padding: 10px 15px !important;"));
            css_parts.push(format!("  background-image: url('http://example.com/assets/img_{}.png');", i));
            css_parts.push(format!("}}"));
            if i % 50 == 0 {
                css_parts.push(format!("@media screen and (min-width: {}px) {{", 300 + (i % 1000)));
                css_parts.push(format!("  .responsive-{} {{ display: none; }}", i));
                css_parts.push(format!("}}"));
            }
        }
        let css = css_parts.join("\n");
        println!("CSS Length: {}", css.len());

        let start = Instant::now();
        let map = LineColMap::new(&css);
        println!("LineColMap creation: {} ms", start.elapsed().as_millis());

        let start = Instant::now();
        let tokens = tokenize(&css);
        println!("Tokenize: {} ms ({} tokens)", start.elapsed().as_millis(), tokens.len());

        let start = Instant::now();
        let mut parser = PostCssParser::new(&css, &map);
        parser.parse(&map);
        println!("Parse: {} ms ({} nodes)", start.elapsed().as_millis(), parser.nodes.len());

        let start = Instant::now();
        let json = serde_json::to_string(&parser.nodes).unwrap();
        println!("Serialize: {} ms (json len: {})", start.elapsed().as_millis(), json.len());
    }
}