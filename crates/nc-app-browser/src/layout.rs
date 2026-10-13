extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use nc_font::{text_width, Style};
use nc_gfx::Rect;
use nc_ui::{text_style, ACCENT_LINE, TEXT_PRIMARY};
use crate::dom::{DomTree, NodeType};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DisplayType {
    Block,
    Inline,
    None,
}

#[derive(Clone, Debug)]
pub struct ComputedStyle {
    pub display: DisplayType,
    pub font_size: i32,
    pub color: u32,
    pub bg_color: Option<u32>,
    pub margin_bottom: i32,
    pub padding: i32,
    pub is_link: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutBox {
    pub rect: Rect,
    pub text: Option<String>,
    pub style: ComputedStyle,
    pub link_url: Option<String>,
    pub children: Vec<LayoutBox>,
}

pub struct LayoutEngine {
    pub boxes: Vec<LayoutBox>,
    pub total_height: i32,
}

impl LayoutEngine {
    pub fn build(tree: &DomTree, content_width: i32) -> Self {
        let mut engine = LayoutEngine {
            boxes: Vec::new(),
            total_height: 0,
        };
        let mut cursor_y = 10;
        engine.layout_children(tree, tree.root, 10, &mut cursor_y, content_width - 20);
        engine.total_height = cursor_y + 20;
        engine
    }

    fn layout_children(
        &mut self,
        tree: &DomTree,
        parent_id: usize,
        x: i32,
        cursor_y: &mut i32,
        available_width: i32,
    ) {
        for &child_id in &tree.nodes[parent_id].children {
            let node = &tree.nodes[child_id];
            match &node.node_type {
                NodeType::Element { tag, attributes } => {
                    let style = compute_element_style(tag);
                    if style.display == DisplayType::None {
                        continue;
                    }

                    let link_url = if tag == "a" {
                        attributes.iter().find(|(k, _)| k == "href").map(|(_, v)| v.clone())
                    } else {
                        None
                    };

                    let start_y = *cursor_y;
                    let mut inner_y = start_y + style.padding;

                    let mut child_boxes = Vec::new();
                    for &sub_child_id in &node.children {
                        if let NodeType::Text(ref raw_text) = tree.nodes[sub_child_id].node_type {
                            let text_style_font = text_style(style.font_size, 30);
                            let wrapped_lines = wrap_text(raw_text, &text_style_font, available_width - style.padding * 2);
                            let line_height = style.font_size * 14 / 10;

                            for line in wrapped_lines {
                                let line_box = LayoutBox {
                                    rect: Rect::new(x + style.padding, inner_y, available_width - style.padding * 2, line_height),
                                    text: Some(line),
                                    style: style.clone(),
                                    link_url: link_url.clone(),
                                    children: Vec::new(),
                                };
                                inner_y += line_height;
                                child_boxes.push(line_box);
                            }
                        } else {
                            self.layout_children(tree, sub_child_id, x + style.padding, &mut inner_y, available_width - style.padding * 2);
                        }
                    }

                    let block_height = (inner_y - start_y + style.padding).max(style.font_size);
                    let element_box = LayoutBox {
                        rect: Rect::new(x, start_y, available_width, block_height),
                        text: None,
                        style: style.clone(),
                        link_url,
                        children: child_boxes,
                    };

                    *cursor_y = start_y + block_height + style.margin_bottom;
                    self.boxes.push(element_box);
                }
                NodeType::Text(text) => {
                    let default_style = ComputedStyle {
                        display: DisplayType::Block,
                        font_size: 12,
                        color: TEXT_PRIMARY,
                        bg_color: None,
                        margin_bottom: 6,
                        padding: 0,
                        is_link: false,
                    };
                    let font = text_style(default_style.font_size, 30);
                    let lines = wrap_text(text, &font, available_width);
                    let line_h = default_style.font_size * 14 / 10;
                    for l in lines {
                        self.boxes.push(LayoutBox {
                            rect: Rect::new(x, *cursor_y, available_width, line_h),
                            text: Some(l),
                            style: default_style.clone(),
                            link_url: None,
                            children: Vec::new(),
                        });
                        *cursor_y += line_h;
                    }
                    *cursor_y += default_style.margin_bottom;
                }
            }
        }
    }
}

fn compute_element_style(tag: &str) -> ComputedStyle {
    match tag {
        "h1" => ComputedStyle {
            display: DisplayType::Block,
            font_size: 20,
            color: TEXT_PRIMARY,
            bg_color: None,
            margin_bottom: 10,
            padding: 0,
            is_link: false,
        },
        "h2" => ComputedStyle {
            display: DisplayType::Block,
            font_size: 16,
            color: TEXT_PRIMARY,
            bg_color: None,
            margin_bottom: 8,
            padding: 0,
            is_link: false,
        },
        "h3" => ComputedStyle {
            display: DisplayType::Block,
            font_size: 14,
            color: TEXT_PRIMARY,
            bg_color: None,
            margin_bottom: 6,
            padding: 0,
            is_link: false,
        },
        "p" => ComputedStyle {
            display: DisplayType::Block,
            font_size: 12,
            color: TEXT_PRIMARY,
            bg_color: None,
            margin_bottom: 8,
            padding: 0,
            is_link: false,
        },
        "a" => ComputedStyle {
            display: DisplayType::Inline,
            font_size: 12,
            color: ACCENT_LINE,
            bg_color: None,
            margin_bottom: 4,
            padding: 0,
            is_link: true,
        },
        "pre" | "code" => ComputedStyle {
            display: DisplayType::Block,
            font_size: 11,
            color: TEXT_PRIMARY,
            bg_color: Some(nc_gfx::rgb(241, 245, 249)),
            margin_bottom: 8,
            padding: 6,
            is_link: false,
        },
        "head" | "style" | "script" => ComputedStyle {
            display: DisplayType::None,
            font_size: 0,
            color: 0,
            bg_color: None,
            margin_bottom: 0,
            padding: 0,
            is_link: false,
        },
        _ => ComputedStyle {
            display: DisplayType::Block,
            font_size: 12,
            color: TEXT_PRIMARY,
            bg_color: None,
            margin_bottom: 4,
            padding: 0,
            is_link: false,
        },
    }
}

fn wrap_text(text: &str, style: &Style, max_width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        let test_line = if current_line.is_empty() {
            String::from(word)
        } else {
            alloc::format!("{} {}", current_line, word)
        };

        if text_width(&test_line, style) <= max_width {
            current_line = test_line;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = String::from(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}
