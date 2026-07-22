use serde::Serialize;
use napi_derive::napi;
use napi::bindgen_prelude::Int32Array;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

static IS_WORD_STOP: [bool; 256] = {
    let mut table = [false; 256];
    table[b'\t' as usize] = true;
    table[b'\n' as usize] = true;
    table[b'\x0c' as usize] = true;
    table[b'\r' as usize] = true;
    table[b' ' as usize] = true;
    table[b'!' as usize] = true;
    table[b'"' as usize] = true;
    table[b'#' as usize] = true;
    table[b'\'' as usize] = true;
    table[b'(' as usize] = true;
    table[b')' as usize] = true;
    table[b':' as usize] = true;
    table[b';' as usize] = true;
    table[b'@' as usize] = true;
    table[b'[' as usize] = true;
    table[b'\\' as usize] = true;
    table[b']' as usize] = true;
    table[b'{' as usize] = true;
    table[b'}' as usize] = true;
    table
};

static IS_AT_STOP: [bool; 256] = {
    let mut table = [false; 256];
    table[b'\t' as usize] = true;
    table[b'\n' as usize] = true;
    table[b'\x0c' as usize] = true;
    table[b'\r' as usize] = true;
    table[b' ' as usize] = true;
    table[b'"' as usize] = true;
    table[b'#' as usize] = true;
    table[b'\'' as usize] = true;
    table[b'(' as usize] = true;
    table[b')' as usize] = true;
    table[b'/' as usize] = true;
    table[b';' as usize] = true;
    table[b'[' as usize] = true;
    table[b'\\' as usize] = true;
    table[b']' as usize] = true;
    table[b'{' as usize] = true;
    table[b'}' as usize] = true;
    table
};

pub fn tokenize<'a>(css: &'a str) -> Result<Vec<Token<'a>>, String> {
    let mut tokens = Vec::with_capacity(css.len() / 5);
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
                    if prev.token_type == TokenType::Word && prev.content.eq_ignore_ascii_case("url") {
                        is_url = true;
                    }
                }

                let start = pos;
                pos += 1;
                if is_url {
                    let mut escaped = false;
                    let mut in_quote = None;
                    while pos < len {
                        let c = bytes[pos];
                        if let Some(q) = in_quote {
                            if c == q && !escaped {
                                in_quote = None;
                            }
                        } else if (c == b'\'' || c == b'"') && !escaped {
                            in_quote = Some(c);
                        } else if c == b')' && !escaped {
                            pos += 1;
                            break;
                        }
                        if c == b'\\' {
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
                        let bracket_content = &css[start..next];
                        let mut is_bad = false;
                        if bracket_content.len() > 1 {
                            for c in bracket_content[1..].bytes() {
                                if c == b'\n' || c == b'\r' || c == b'"' || c == b'\'' || c == b'(' || c == b'\\' || c == b'/' {
                                    is_bad = true;
                                    break;
                                }
                            }
                        }
                        if is_bad {
                            tokens.push(Token {
                                token_type: TokenType::Char('('),
                                content: "(",
                                start,
                                end: start + 1,
                            });
                            pos = start + 1;
                        } else {
                            pos = next;
                            tokens.push(Token {
                                token_type: TokenType::Brackets,
                                content: bracket_content,
                                start,
                                end: pos,
                            });
                        }
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
                let mut closed = false;
                while pos < len {
                    if bytes[pos] == quote && !escaped {
                        pos += 1;
                        closed = true;
                        break;
                    }
                    if bytes[pos] == b'\\' {
                        escaped = !escaped;
                    } else {
                        escaped = false;
                    }
                    pos += 1;
                }
                if !closed {
                    return Err(format!("Unclosed string:{}", start));
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
                while pos < len && !IS_AT_STOP[bytes[pos] as usize] {
                    pos += 1;
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
                    let mut closed = false;
                    while pos + 1 < len {
                        if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
                            pos += 2;
                            closed = true;
                            break;
                        }
                        pos += 1;
                    }
                    if !closed {
                        return Err(format!("Unclosed comment:{}", start));
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
                        if IS_WORD_STOP[bytes[pos] as usize] {
                            break;
                        }
                        if bytes[pos] == b'/' && pos + 1 < len && bytes[pos + 1] == b'*' {
                            break;
                        }
                        pos += 1;
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
                    if IS_WORD_STOP[bytes[pos] as usize] {
                        break;
                    }
                    if bytes[pos] == b'/' && pos + 1 < len && bytes[pos + 1] == b'*' {
                        break;
                    }
                    pos += 1;
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

    Ok(tokens)
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
        nodes: Vec<usize>,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_between")]
        raws_between: String,
        #[serde(rename = "raws_after")]
        raws_after: Option<String>,
        #[serde(rename = "raws_semicolon")]
        raws_semicolon: bool,
        #[serde(rename = "raws_selector")]
        raws_selector: Option<String>,
        #[serde(rename = "raws_own_semicolon")]
        raws_own_semicolon: Option<String>,
    },
    #[serde(rename = "decl")]
    Decl {
        prop: String,
        value: String,
        important: bool,
        #[serde(rename = "raws_before")]
        raws_before: String,
        #[serde(rename = "raws_between")]
        raws_between: String,
        #[serde(rename = "raws_important")]
        raws_important: Option<String>,
        #[serde(rename = "raws_value")]
        raws_value: Option<String>,
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
    last_hint: std::cell::Cell<usize>,
}

impl LineColMap {
    pub fn new(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut line_starts = vec![0];
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            line_starts,
            last_hint: std::cell::Cell::new(0),
        }
    }

    pub fn get(&self, offset: usize) -> (u32, u32) {
        let len = self.line_starts.len();
        let mut hint = self.last_hint.get();
        if hint >= len {
            hint = len.saturating_sub(1);
        }
        while hint + 1 < len && self.line_starts[hint + 1] <= offset {
            hint += 1;
        }
        while hint > 0 && self.line_starts[hint] > offset {
            hint -= 1;
        }
        self.last_hint.set(hint);
        let line = (hint + 1) as u32;
        let start = self.line_starts[hint];
        let col = (offset - start + 1) as u32;
        (line, col)
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
}

impl<'a> PostCssParser<'a> {
    pub fn new(css: &'a str, map: &LineColMap) -> Result<Self, String> {
        let tokens = tokenize(css)?;
        let mut parser = Self {
            css,
            tokens,
            token_idx: 0,
            nodes: Vec::with_capacity(css.len() / 25),
            spaces: String::new(),
            semicolon: false,
            current: 0,
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
        Ok(parser)
    }

    fn get_pos(&self, offset: usize, map: &LineColMap) -> SourcePos {
        let (line, column) = map.get(offset);
        SourcePos { line, column, offset }
    }

    fn get_end_pos(&self, offset: usize, map: &LineColMap) -> SourcePos {
        if offset > 0 {
            let (line, column) = map.get(offset - 1);
            SourcePos { line, column, offset }
        } else {
            SourcePos { line: 1, column: 1, offset: 0 }
        }
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

    fn comment(&mut self, token: &Token<'a>, map: &LineColMap) {
        let before = std::mem::take(&mut self.spaces);
        let mut text_raw = token.content;
        if text_raw.starts_with("/*") {
            text_raw = &text_raw[2..];
        }
        if text_raw.ends_with("*/") {
            text_raw = &text_raw[..text_raw.len() - 2];
        }
        
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
        self.nodes[node_id].source.end = Some(self.get_end_pos(token.end, map));
    }

    fn free_semicolon(&mut self, token: &Token<'a>, map: &LineColMap) {
        self.spaces.push_str(token.content);
        let current_id = self.current;
        let last_node_id = match &self.nodes[current_id].data {
            PostCssNodeData::Root { nodes, .. } => nodes.last().copied(),
            PostCssNodeData::Rule { nodes, .. } => nodes.last().copied(),
            PostCssNodeData::AtRule { nodes: Some(nodes), .. } => nodes.last().copied(),
            _ => None,
        };
        if let Some(last_id) = last_node_id {
            let mut is_rule_without_own_semi = false;
            match &self.nodes[last_id].data {
                PostCssNodeData::Rule { raws_own_semicolon, .. } => {
                    if raws_own_semicolon.is_none() {
                        is_rule_without_own_semi = true;
                    }
                }
                _ => {}
            }
            if is_rule_without_own_semi {
                let spaces_taken = std::mem::take(&mut self.spaces);
                let final_offset = token.start + spaces_taken.len();
                match &mut self.nodes[last_id].data {
                    PostCssNodeData::Rule { raws_own_semicolon, .. } => {
                        *raws_own_semicolon = Some(spaces_taken);
                    }
                    _ => {}
                }
                let mut end_pos = self.get_end_pos(token.end, map);
                end_pos.offset = final_offset;
                self.nodes[last_id].source.end = Some(end_pos);
            }
        }
    }

    fn end(&mut self, token: &Token<'a>, map: &LineColMap) -> Result<(), String> {
        let current_id = self.current;
        if current_id == 0 {
            return Err(format!("Unexpected }}:{}:{}", token.start, token.end));
        }
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
                self.nodes[current_id].source.end = Some(self.get_end_pos(token.end, map));
                self.nodes[current_id].parent
            }
            PostCssNodeData::AtRule { raws_after, raws_semicolon, .. } => {
                raws_after.get_or_insert(String::new()).push_str(&spaces_taken);
                *raws_semicolon = semicolon_val;
                self.nodes[current_id].source.end = Some(self.get_end_pos(token.end, map));
                self.nodes[current_id].parent
            }
            _ => None,
        };

        if let Some(p_id) = parent_id {
            self.current = p_id;
        }
        Ok(())
    }

    fn atrule(&mut self, start_token: &Token<'a>, map: &LineColMap) -> Result<(), String> {
        let name = start_token.content[1..].to_string();
        if name.is_empty() {
            return Err(format!("At-rule without name:{}:{}", start_token.start, start_token.end));
        }
        let before = std::mem::take(&mut self.spaces);
        
        let mut params_tokens = Vec::new();
        let mut brackets = Vec::new();
        let mut open = false;

        let mut end_offset = 0;

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
                        end_offset = token.end;
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



        let between = spaces_and_comments_from_end(&mut params_tokens);
        let after_name = spaces_and_comments_from_start(&mut params_tokens);
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
            let mut final_end_offset = None;
            if end_offset > 0 {
                final_end_offset = Some(end_offset);
            } else if !params_tokens.is_empty() {
                final_end_offset = Some(params_tokens.last().unwrap().end);
            }
            if let Some(offset_val) = final_end_offset {
                self.nodes[node_id].source.end = Some(self.get_end_pos(offset_val, map));
            }
        } else {
            self.current = node_id;
        }
        Ok(())
    }

    fn other(&mut self, start_token: Token<'a>, map: &LineColMap) -> Result<(), String> {
        let custom_property = start_token.content.starts_with("--");
        let mut brackets = Vec::new();
        let mut bracket_offsets = Vec::new();
        match &start_token.token_type {
            TokenType::Char('(') | TokenType::Char('[') => {
                brackets.push(if start_token.token_type == TokenType::Char('(') { ')' } else { ']' });
                bracket_offsets.push(start_token.start);
            }
            _ => {}
        }
        let mut tokens = vec![start_token];
        let mut colon = false;
        let mut end = false;

        while !self.end_of_file() {
            let token = self.next_token().unwrap().clone();
            let t_type = &token.token_type;
            tokens.push(token.clone());

            let mut popped = false;
            match t_type {
                TokenType::Char('(') | TokenType::Char('[') => {
                    brackets.push(if *t_type == TokenType::Char('(') { ')' } else { ']' });
                    bracket_offsets.push(token.start);
                }
                TokenType::Char('{') if custom_property && colon => {
                    brackets.push('}');
                    bracket_offsets.push(token.start);
                }
                TokenType::Char(c) if !brackets.is_empty() && Some(*c) == brackets.last().copied() => {
                    brackets.pop();
                    bracket_offsets.pop();
                    popped = true;
                }
                _ => {}
            }

            if brackets.is_empty() && !popped {
                match t_type {
                    TokenType::Char(';') => {
                        if colon {
                            self.decl(tokens, custom_property, map)?;
                            return Ok(());
                        } else {
                            break;
                        }
                    }
                    TokenType::Char('{') => {
                        self.rule(tokens, map);
                        return Ok(());
                    }
                    TokenType::Char('}') => {
                        tokens.pop();
                        self.back();
                        end = true;
                        break;
                    }
                    TokenType::Char(':') => {
                        colon = true;
                    }
                    _ => {}
                }
            }
        }

        if self.end_of_file() {
            end = true;
        }

        if !brackets.is_empty() {
            let offset = bracket_offsets.last().copied().unwrap_or(tokens[0].start);
            return Err(format!("Unclosed bracket:{}:{}", offset, offset + 1));
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
            self.decl(tokens, custom_property, map)?;
            Ok(())
        } else {
            let start_offset = tokens[0].start;
            let end_offset = tokens[0].end;
            Err(format!("Unknown word:{}:{}", start_offset, end_offset))
        }
    }

    fn decl(&mut self, mut tokens: Vec<Token<'a>>, custom_property: bool, map: &LineColMap) -> Result<(), String> {
        let before = std::mem::take(&mut self.spaces);
        let mut important = false;
        let mut important_raw = None;

        let mut end_offset = 0;

        if let Some(last) = tokens.last() {
            if last.token_type == TokenType::Char(';') {
                self.semicolon = true;
                end_offset = last.end;
                tokens.pop();
            } else {
                self.semicolon = false;
            }
        } else {
            self.semicolon = false;
        }

        let mut start_idx = 0;
        while start_idx < tokens.len() && tokens[start_idx].token_type != TokenType::Word {
            start_idx += 1;
        }

        if start_idx >= tokens.len() {
            return Ok(());
        }

        let mut prop_end_idx = start_idx;
        while prop_end_idx < tokens.len() {
            let t_type = &tokens[prop_end_idx].token_type;
            if *t_type == TokenType::Char(':') || *t_type == TokenType::Space || *t_type == TokenType::Comment {
                break;
            }
            prop_end_idx += 1;
        }

        let mut prop = tokens[start_idx..prop_end_idx].iter().map(|t| t.content).collect::<String>();
        let mut before_extra: String = tokens[0..start_idx].iter().map(|t| t.content).collect();
        if prop.starts_with('_') || prop.starts_with('*') {
            before_extra.push(prop.remove(0));
        }
        let before_total = before + &before_extra;

        let between_start_idx = prop_end_idx;
        let mut between_end_idx = prop_end_idx;
        while between_end_idx < tokens.len() {
            let token = &tokens[between_end_idx];
            if token.token_type == TokenType::Char(':') {
                between_end_idx += 1;
                break;
            }
            if token.token_type == TokenType::Word && token.content.chars().any(|c| c.is_alphanumeric() || c == '_') {
                return Err(format!("Unknown word:{}:{}", token.start, token.end));
            }
            between_end_idx += 1;
        }

        let mut between = tokens[between_start_idx..between_end_idx].iter().map(|t| t.content).collect::<String>();

        let val_tokens = tokens[between_end_idx..].to_vec();

        let mut first_spaces = Vec::new();
        let mut val_start = 0;
        while val_start < val_tokens.len() {
            let t = &val_tokens[val_start];
            if t.token_type == TokenType::Space || t.token_type == TokenType::Comment {
                first_spaces.push(t.clone());
                val_start += 1;
            } else {
                break;
            }
        }
        let mut remaining_tokens = val_tokens[val_start..].to_vec();

        let mut important_idx = None;
        for (i, t) in remaining_tokens.iter().enumerate().rev() {
            if t.token_type != TokenType::Space && t.token_type != TokenType::Comment {
                if t.content.to_lowercase() == "!important" || t.content.to_lowercase() == "important" {
                    important_idx = Some(i);
                }
                break;
            }
        }

        if let Some(pos) = important_idx {
            if remaining_tokens[pos].content.to_lowercase() == "!important" {
                important = true;
                let imp_token_content = remaining_tokens[pos].content;
                let after_imp: String = remaining_tokens[pos + 1..].iter().map(|t| t.content).collect();
                remaining_tokens.truncate(pos);
                let before_imp = spaces_from_end(&mut remaining_tokens);
                let imp_raw = before_imp + imp_token_content + &after_imp;
                if imp_raw != " !important" {
                    important_raw = Some(imp_raw);
                }
            } else {
                let mut cache = remaining_tokens.clone();
                let mut str_collected = String::new();
                let mut j = pos;
                while j > 0 {
                    let current_type = &cache[j].token_type;
                    let trimmed = str_collected.trim_start();
                    if trimmed.starts_with('!') && *current_type != TokenType::Space {
                        break;
                    }
                    let popped = cache.pop().unwrap();
                    str_collected = popped.content.to_string() + &str_collected;
                    j -= 1;
                }
                if str_collected.trim_start().starts_with('!') {
                    important = true;
                    let after_imp: String = remaining_tokens[pos + 1..].iter().map(|t| t.content).collect();
                    let imp_raw = str_collected + &after_imp;
                    if imp_raw != " !important" {
                        important_raw = Some(imp_raw);
                    }
                    remaining_tokens = cache;
                }
            }
        }

        let has_word = remaining_tokens.iter().any(|t| t.token_type != TokenType::Space && t.token_type != TokenType::Comment);
        let final_tokens = if has_word {
            let first_spaces_str: String = first_spaces.iter().map(|t| t.content).collect();
            between.push_str(&first_spaces_str);
            remaining_tokens
        } else {
            let mut ft = first_spaces;
            ft.extend(remaining_tokens);
            ft
        };

        let (value, value_raw) = clean_value(&final_tokens, custom_property);

        if !custom_property && value.contains(':') {
            let mut colon_idx = None;
            let mut brackets = 0;
            let mut prev_tok: Option<&Token<'a>> = None;
            for (i, t) in val_tokens.iter().enumerate() {
                if t.token_type == TokenType::Char('(') {
                    brackets += 1;
                } else if t.token_type == TokenType::Char(')') {
                    if brackets > 0 {
                        brackets -= 1;
                    }
                } else if brackets == 0 && t.token_type == TokenType::Char(':') {
                    match prev_tok {
                        None => {
                            return Err(format!("Double colon:{}:{}", t.start, t.start + 1));
                        }
                        Some(p) => {
                            if p.token_type == TokenType::Word && p.content == "progid" {
                                // Ignore
                            } else {
                                colon_idx = Some(i);
                                break;
                            }
                        }
                    }
                }
                prev_tok = Some(t);
            }

            if let Some(c_idx) = colon_idx {
                let mut word_token = &val_tokens[c_idx];
                let mut founded = 0;
                for j in (0..c_idx).rev() {
                    if val_tokens[j].token_type != TokenType::Space {
                        founded += 1;
                        if founded == 2 {
                            word_token = &val_tokens[j];
                            break;
                        }
                    }
                }
                let offset = if word_token.token_type == TokenType::Word {
                    word_token.end
                } else {
                    word_token.start
                };
                return Err(format!("Missed semicolon:{}", offset));
            }
        }

        let data = PostCssNodeData::Decl {
            prop,
            value,
            important,
            raws_before: before_total,
            raws_between: between,
            raws_important: important_raw,
            raws_value: value_raw,
        };

        let node_id = self.add_node(data, tokens[start_idx].start, map);
        if end_offset == 0 {
            let mut found_non_space = false;
            for t in val_tokens.iter().rev() {
                if t.token_type != TokenType::Space {
                    end_offset = t.end;
                    found_non_space = true;
                    break;
                }
            }
            if !found_non_space {
                if between_end_idx > 0 {
                    end_offset = tokens[between_end_idx - 1].end;
                } else {
                    end_offset = tokens[start_idx].end;
                }
            }
        }
        self.nodes[node_id].source.end = Some(self.get_end_pos(end_offset, map));
        Ok(())
    }

    fn rule(&mut self, mut tokens: Vec<Token<'a>>, map: &LineColMap) {
        if let Some(last) = tokens.last() {
            if last.token_type == TokenType::Char('{') {
                tokens.pop();
            }
        }

        let before = std::mem::take(&mut self.spaces);
        let between = spaces_and_comments_from_end(&mut tokens);
        let (selector, selector_raw) = clean_value(&tokens, false);

        let start_offset = tokens.first().map(|t| t.start).unwrap_or(0);

        let data = PostCssNodeData::Rule {
            selector,
            nodes: Vec::new(),
            raws_before: before,
            raws_between: between,
            raws_after: None,
            raws_semicolon: false,
            raws_selector: selector_raw,
            raws_own_semicolon: None,
        };

        let node_id = self.add_node(data, start_offset, map);
        self.current = node_id;
    }

    pub fn parse(&mut self, map: &LineColMap) -> Result<(), String> {
        while !self.end_of_file() {
            let token = self.next_token().unwrap().clone();
            match &token.token_type {
                TokenType::Space => {
                    self.spaces.push_str(token.content);
                }
                TokenType::Char(';') => {
                    self.free_semicolon(&token, map);
                }
                TokenType::Char('}') => {
                    self.end(&token, map)?;
                }
                TokenType::Comment => {
                    self.comment(&token, map);
                }
                TokenType::AtWord => {
                    self.atrule(&token, map)?;
                }
                TokenType::Char('{') => {
                    self.rule(vec![token], map);
                }
                _ => {
                    self.other(token, map)?;
                }
            }
        }

        if self.current != 0 {
            let start = &self.nodes[self.current].source.start;
            return Err(format!("Unclosed block:{}:{}", start.offset, start.offset + 1));
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
        Ok(())
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

fn clean_value(tokens: &[Token], custom_property: bool) -> (String, Option<String>) {
    let mut value = String::new();
    let mut clean = true;
    let length = tokens.len();
    
    for (i, token) in tokens.iter().enumerate() {
        match token.token_type {
            TokenType::Space if i == length - 1 && !custom_property => {
                clean = false;
            }
            TokenType::Comment => {
                let prev = if i > 0 { &tokens[i - 1].token_type } else { &TokenType::Space };
                let next = if i + 1 < length { &tokens[i + 1].token_type } else { &TokenType::Space };
                fn is_safe(t: &TokenType) -> bool {
                    match t {
                        TokenType::Space | TokenType::Char(';') | TokenType::Char('{') | TokenType::Char('}') | TokenType::Char(',') => true,
                        _ => false
                    }
                }
                if !is_safe(prev) && !is_safe(next) {
                    if value.ends_with(',') {
                        clean = false;
                    } else {
                        value.push_str(token.content);
                    }
                } else {
                    clean = false;
                }
            }
            _ => {
                value.push_str(token.content);
            }
        }
    }
    
    let raw = if !clean {
        let raw_str: String = tokens.iter().map(|t| t.content).collect();
        Some(raw_str)
    } else {
        None
    };
    
    (value, raw)
}

#[napi]
pub fn parse_css(css: String) -> String {
    let map = LineColMap::new(&css);
    let mut parser = match PostCssParser::new(&css, &map) {
        Ok(p) => p,
        Err(_) => return "[]".to_string(),
    };
    let _ = parser.parse(&map);
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
    pub fn with_capacity(nodes_count: usize, css_len: usize) -> Self {
        Self {
            metadata: Vec::with_capacity(nodes_count * 23),
            big_string: String::with_capacity(css_len),
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
    let mut builder = AstBufferBuilder::with_capacity(nodes.len(), nodes.len() * 32);
    
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
            PostCssNodeData::Root { raws_after, raws_semicolon, .. } => {
                slots[4] = builder.push_string(raws_after.as_deref().unwrap_or(""));
                if *raws_semicolon { semicolon = 1; }
                has_nodes = 1;
            }
            PostCssNodeData::Rule { selector, raws_before, raws_between, raws_after, raws_semicolon, raws_selector, raws_own_semicolon, .. } => {
                slots[0] = builder.push_string(selector);
                slots[1] = builder.push_string(raws_own_semicolon.as_deref().unwrap_or(""));
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_between);
                slots[4] = builder.push_string(raws_after.as_deref().unwrap_or(""));
                slots[5] = builder.push_string(raws_selector.as_deref().unwrap_or(""));
                if *raws_semicolon { semicolon = 1; }
                has_nodes = 1;
            }
            PostCssNodeData::Decl { prop, value, important: imp, raws_before, raws_between, raws_important, raws_value } => {
                slots[0] = builder.push_string(prop);
                slots[1] = builder.push_string(value);
                slots[2] = builder.push_string(raws_before);
                slots[3] = builder.push_string(raws_between);
                slots[4] = builder.push_string(raws_value.as_deref().unwrap_or(""));
                slots[5] = builder.push_string(raws_important.as_deref().unwrap_or(""));
                if *imp { important = 1; }
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
    }
    
    AstBuffer {
        metadata: Int32Array::from(builder.metadata),
        big_string: builder.big_string,
    }
}

#[napi]
pub fn parse_css_to_buffer(css: String) -> Result<AstBuffer, napi::Error> {
    let map = LineColMap::new(&css);
    let mut parser = match PostCssParser::new(&css, &map) {
        Ok(p) => p,
        Err(err) => return Err(napi::Error::new(napi::Status::GenericFailure, err)),
    };
    if let Err(err) = parser.parse(&map) {
        return Err(napi::Error::new(napi::Status::GenericFailure, err));
    }
    Ok(serialize_to_buffer(&parser.nodes))
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
        let tokens = tokenize(&css).unwrap();
        println!("Tokenize: {} ms ({} tokens)", start.elapsed().as_millis(), tokens.len());

        let start = Instant::now();
        let mut parser = PostCssParser::new(&css, &map).unwrap();
        let _ = parser.parse(&map);
        println!("Parse: {} ms ({} nodes)", start.elapsed().as_millis(), parser.nodes.len());

        let start = Instant::now();
        let json = serde_json::to_string(&parser.nodes).unwrap();
        println!("Serialize: {} ms (json len: {})", start.elapsed().as_millis(), json.len());
    }
}
