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
}