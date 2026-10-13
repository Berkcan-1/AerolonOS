extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::dom::{DomTree, NodeType};

#[derive(Debug)]
enum Token {
    StartTag {
        name: String,
        attributes: Vec<(String, String)>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(String),
}

pub struct HtmlParser {
    chars: Vec<char>,
    pos: usize,
}

impl HtmlParser {
    pub fn parse(html: &str) -> DomTree {
        let mut parser = HtmlParser {
            chars: html.chars().collect(),
            pos: 0,
        };
        let tokens = parser.tokenize();
        parser.build_tree(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.chars.len() {
            if self.peek() == Some('<') {
                self.next(); // '<' karakterini tüket
                if self.peek() == Some('!') {
                    // Yorum veya DOCTYPE'ı atla
                    while let Some(ch) = self.next() {
                        if ch == '>' { break; }
                    }
                    continue;
                }

                let is_closing = if self.peek() == Some('/') {
                    self.next();
                    true
                } else {
                    false
                };

                let mut tag_name = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_whitespace() || ch == '>' || ch == '/' {
                        break;
                    }
                    tag_name.push(ch.to_ascii_lowercase());
                    self.next();
                }

                if is_closing {
                    while let Some(ch) = self.next() {
                        if ch == '>' { break; }
                    }
                    tokens.push(Token::EndTag { name: tag_name });
                } else {
                    let mut attributes = Vec::new();
                    let mut self_closing = false;

                    loop {
                        self.consume_whitespace();
                        match self.peek() {
                            Some('>') => {
                                self.next();
                                break;
                            }
                            Some('/') => {
                                self.next();
                                if self.peek() == Some('>') {
                                    self.next();
                                    self_closing = true;
                                    break;
                                }
                            }
                            Some(_) => {
                                let (k, v) = self.parse_attribute();
                                if !k.is_empty() {
                                    attributes.push((k, v));
                                }
                            }
                            None => break,
                        }
                    }

                    // Void (kendi kendini kapatan) HTML etiketleri
                    if tag_name == "br" || tag_name == "hr" || tag_name == "img" || tag_name == "input" {
                        self_closing = true;
                    }

                    let raw_text = !self_closing && (tag_name == "script" || tag_name == "style" || tag_name == "title");
                    let closing = alloc::format!("</{}", tag_name);
                    tokens.push(Token::StartTag {
                        name: tag_name,
                        attributes,
                        self_closing,
                    });
                    if raw_text {
                        self.skip_raw_text(&closing);
                    }
                }
            } else {
                let mut text = String::new();
                while let Some(ch) = self.peek() {
                    if ch == '<' { break; }
                    text.push(ch);
                    self.next();
                }
                if !text.trim().is_empty() {
                    tokens.push(Token::Text(text));
                }
            }
        }
        tokens
    }

    fn skip_raw_text(&mut self, closing: &str) {
        let needle: Vec<char> = closing.chars().collect();
        while self.pos < self.chars.len() {
            if self.chars[self.pos] == '<' && self.pos + needle.len() <= self.chars.len() {
                let mut matched = true;
                for (i, expected) in needle.iter().enumerate() {
                    if self.chars[self.pos + i].to_ascii_lowercase() != *expected {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    return;
                }
            }
            self.pos += 1;
        }
    }

    fn consume_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.next();
            } else {
                break;
            }
        }
    }

    fn parse_attribute(&mut self) -> (String, String) {
        let mut key = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '=' || ch == '>' || ch == '/' {
                break;
            }
            key.push(ch.to_ascii_lowercase());
            self.next();
        }

        self.consume_whitespace();
        let mut val = String::new();
        if self.peek() == Some('=') {
            self.next();
            self.consume_whitespace();
            if let Some(quote) = self.peek() {
                if quote == '"' || quote == '\'' {
                    self.next();
                    while let Some(ch) = self.next() {
                        if ch == quote { break; }
                        val.push(ch);
                    }
                } else {
                    while let Some(ch) = self.peek() {
                        if ch.is_whitespace() || ch == '>' { break; }
                        val.push(ch);
                        self.next();
                    }
                }
            }
        }
        (key, val)
    }

    fn build_tree(&self, tokens: Vec<Token>) -> DomTree {
        let mut tree = DomTree::new();
        let mut stack = alloc::vec![tree.root];

        for token in tokens {
            match token {
                Token::StartTag { name, attributes, self_closing } => {
                    let parent = *stack.last().unwrap();
                    let node_id = tree.add_node(
                        NodeType::Element {
                            tag: name,
                            attributes,
                        },
                        parent,
                    );
                    if !self_closing {
                        stack.push(node_id);
                    }
                }
                Token::EndTag { name } => {
                    if stack.len() > 1 {
                        if let NodeType::Element { ref tag, .. } = tree.nodes[*stack.last().unwrap()].node_type {
                            if tag == &name {
                                stack.pop();
                            }
                        }
                    }
                }
                Token::Text(text) => {
                    let parent = *stack.last().unwrap();
                    tree.add_node(NodeType::Text(text), parent);
                }
            }
        }
        tree
    }
}
