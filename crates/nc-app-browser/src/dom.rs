extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub enum NodeType {
    Element {
        tag: String,
        attributes: Vec<(String, String)>,
    },
    Text(String),
}

#[derive(Clone, Debug)]
pub struct DomNode {
    pub id: usize,
    pub node_type: NodeType,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

pub struct DomTree {
    pub nodes: Vec<DomNode>,
    pub root: usize,
}

impl DomTree {
    pub fn new() -> Self {
        let root_node = DomNode {
            id: 0,
            node_type: NodeType::Element {
                tag: String::from("document"),
                attributes: Vec::new(),
            },
            children: Vec::new(),
            parent: None,
        };
        DomTree {
            nodes: alloc::vec![root_node],
            root: 0,
        }
    }

    pub fn add_node(&mut self, node_type: NodeType, parent_id: usize) -> usize {
        let id = self.nodes.len();
        let node = DomNode {
            id,
            node_type,
            children: Vec::new(),
            parent: Some(parent_id),
        };
        self.nodes.push(node);
        self.nodes[parent_id].children.push(id);
        id
    }

    pub fn tag(&self, node_id: usize) -> Option<&str> {
        match &self.nodes.get(node_id)?.node_type {
            NodeType::Element { tag, .. } => Some(tag.as_str()),
            NodeType::Text(_) => None,
        }
    }

    pub fn attr(&self, node_id: usize, key: &str) -> Option<&str> {
        match &self.nodes.get(node_id)?.node_type {
            NodeType::Element { attributes, .. } => {
                for (k, v) in attributes {
                    if k.eq_ignore_ascii_case(key) {
                        return Some(v.as_str());
                    }
                }
                None
            }
            NodeType::Text(_) => None,
        }
    }

    pub fn id_attr(&self, node_id: usize) -> Option<&str> {
        self.attr(node_id, "id")
    }

    pub fn has_class(&self, node_id: usize, target_class: &str) -> bool {
        if let Some(classes) = self.attr(node_id, "class") {
            for c in classes.split_whitespace() {
                if c.eq_ignore_ascii_case(target_class) {
                    return true;
                }
            }
        }
        false
    }

    pub fn text_content(&self, node_id: usize) -> String {
        let mut out = String::new();
        self.collect_text_recursive(node_id, &mut out);
        out
    }

    fn collect_text_recursive(&self, node_id: usize, out: &mut String) {
        let Some(node) = self.nodes.get(node_id) else {
            return;
        };
        match &node.node_type {
            NodeType::Text(t) => {
                if !out.is_empty() && !out.ends_with(' ') && !t.starts_with(' ') {
                    out.push(' ');
                }
                out.push_str(t);
            }
            NodeType::Element { tag, .. } => {
                if tag == "script" || tag == "style" {
                    return;
                }
                for &child in &node.children {
                    self.collect_text_recursive(child, out);
                }
            }
        }
    }

    pub fn title(&self) -> Option<String> {
        for node in &self.nodes {
            if let NodeType::Element { ref tag, .. } = node.node_type {
                if tag == "title" {
                    let mut t = String::new();
                    for &c in &node.children {
                        if let NodeType::Text(ref s) = self.nodes[c].node_type {
                            t.push_str(s.trim());
                        }
                    }
                    if !t.is_empty() {
                        return Some(t);
                    }
                }
            }
        }
        None
    }

    pub fn collect_style_texts(&self) -> Vec<String> {
        let mut styles = Vec::new();
        for node in &self.nodes {
            if let NodeType::Element { ref tag, .. } = node.node_type {
                if tag == "style" {
                    let mut buf = String::new();
                    for &c in &node.children {
                        if let NodeType::Text(ref s) = self.nodes[c].node_type {
                            buf.push_str(s);
                            buf.push('\n');
                        }
                    }
                    if !buf.trim().is_empty() {
                        styles.push(buf);
                    }
                }
            }
        }
        styles
    }

    pub fn collect_stylesheet_links(&self) -> Vec<String> {
        let mut links = Vec::new();
        for node in &self.nodes {
            if let NodeType::Element { ref tag, .. } = node.node_type {
                if tag == "link" {
                    let rel = self.attr(node.id, "rel").unwrap_or("");
                    let href = self.attr(node.id, "href").unwrap_or("");
                    if (rel.eq_ignore_ascii_case("stylesheet") || href.ends_with(".css")) && !href.is_empty() {
                        links.push(String::from(href));
                    }
                }
            }
        }
        links
    }

    pub fn collect_image_sources(&self) -> Vec<String> {
        let mut srcs = Vec::new();
        for node in &self.nodes {
            if let NodeType::Element { ref tag, .. } = node.node_type {
                if tag == "img" {
                    let src = self.attr(node.id, "src").or_else(|| self.attr(node.id, "data-src")).unwrap_or("");
                    if !src.is_empty() && !srcs.iter().any(|s| s == src) {
                        srcs.push(String::from(src));
                    }
                }
            }
        }
        srcs
    }
}
