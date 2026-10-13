#![no_std]
extern crate alloc;

pub mod dom;
pub mod html;
pub mod layout;

use alloc::string::String;
use layout::{LayoutBox, LayoutEngine};
use nc_app::{App, AppInfo, Context, Icon, Input, KeyCode, Update};
use nc_gfx::{Canvas, Rect};
use nc_net::http::{HttpState, TcpHttpClient};
use nc_ui::{
    draw_button, draw_input_box, draw_panel, draw_text_color, text_style,
    PANEL_BORDER, PANEL_FILL, TEXT_DIM, TEXT_MUTED, WHITE,
};

pub struct BrowserApp {
    info: AppInfo,
    url_input: String,
    current_url: String,
    net_client: TcpHttpClient,
    layout_engine: Option<LayoutEngine>,
    scroll_y: i32,
    hovered_link: Option<String>,
    hover_go: bool,
    hover_back: bool,
}

impl BrowserApp {
    pub fn new() -> BrowserApp {
        let default_url = String::from("http://info.cern.ch/hypertext/WWW/TheProject.html");
        let initial_html = "<h1>NetizenCore Web Engine</h1><p>Bagimsiz web tarayici motoru calisiyor.</p>";
        let dom = html::HtmlParser::parse(initial_html);
        let layout = LayoutEngine::build(&dom, 600);

        BrowserApp {
            info: AppInfo::new("nc.browser", "BROWSER", "1.0.0", Icon::Generic, 48, 30),
            url_input: default_url.clone(),
            current_url: default_url,
            net_client: TcpHttpClient::new(),
            layout_engine: Some(layout),
            scroll_y: 0,
            hovered_link: None,
            hover_go: false,
            hover_back: false,
        }
    }

    fn load_url(&mut self, url: &str) {
        self.current_url = String::from(url);
        self.url_input = String::from(url);
        self.scroll_y = 0;
        self.net_client.navigate(url, nc_net::global());
    }
}

impl App for BrowserApp {
    fn info(&self) -> &AppInfo {
        &self.info
    }

    fn update(&mut self, ctx: &mut Context, input: &Input) -> Update {
        let unit = ctx.unit;
        let mut redraw = false;

        self.net_client.poll(nc_net::global());

        if self.net_client.state == HttpState::Complete && !self.net_client.html_body.is_empty() {
            let dom = html::HtmlParser::parse(&self.net_client.html_body);
            let content_w = input.area.width - unit * 2;
            self.layout_engine = Some(LayoutEngine::build(&dom, content_w));
            self.net_client.html_body.clear();
            redraw = true;
        }

        if let Some(key) = input.key {
            match key {
                KeyCode::Char(c) => {
                    self.url_input.push(c);
                    redraw = true;
                }
                KeyCode::Backspace => {
                    self.url_input.pop();
                    redraw = true;
                }
                KeyCode::Enter => {
                    let target = self.url_input.clone();
                    self.load_url(&target);
                    redraw = true;
                }
                KeyCode::Up => {
                    self.scroll_y = (self.scroll_y - 24).max(0);
                    redraw = true;
                }
                KeyCode::Down => {
                    if let Some(ref l) = self.layout_engine {
                        let max_scroll = (l.total_height - (input.area.height - unit * 8)).max(0);
                        self.scroll_y = (self.scroll_y + 24).min(max_scroll);
                    }
                    redraw = true;
                }
                _ => {}
            }
        }

        let nav_y = input.area.y + unit;
        let back_btn = Rect::new(input.area.x + unit, nav_y, unit * 3, unit * 2);
        let go_btn = Rect::new(input.area.x + input.area.width - unit * 4, nav_y, unit * 3, unit * 2);
        let viewport_y = input.area.y + unit * 4;
        let viewport = Rect::new(input.area.x + unit, viewport_y, input.area.width - unit * 2, input.area.height - unit * 6);

        let mut found_hovered_link = None;
        let mut clicked_nav_target = None;

        if let Some(pointer) = input.pointer {
            let h_go = inside(go_btn, pointer.x, pointer.y);
            let h_back = inside(back_btn, pointer.x, pointer.y);
            if h_go != self.hover_go || h_back != self.hover_back {
                self.hover_go = h_go;
                self.hover_back = h_back;
                redraw = true;
            }

            if pointer.clicked && h_go {
                let target = self.url_input.clone();
                self.load_url(&target);
                redraw = true;
            }

            if inside(viewport, pointer.x, pointer.y) {
                if let Some(ref l) = self.layout_engine {
                    let rel_x = pointer.x - viewport.x;
                    let rel_y = pointer.y - viewport.y + self.scroll_y;

                    'search: for b in &l.boxes {
                        if let Some(ref url) = b.link_url {
                            if inside(b.rect, rel_x, rel_y) {
                                found_hovered_link = Some(url.clone());
                                if pointer.clicked {
                                    clicked_nav_target = Some(url.clone());
                                }
                                break 'search;
                            }
                        }
                        for cb in &b.children {
                            if let Some(ref url) = cb.link_url {
                                if inside(cb.rect, rel_x, rel_y) {
                                    found_hovered_link = Some(url.clone());
                                    if pointer.clicked {
                                        clicked_nav_target = Some(url.clone());
                                    }
                                    break 'search;
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.hovered_link != found_hovered_link {
            self.hovered_link = found_hovered_link;
            redraw = true;
        }

        if let Some(target) = clicked_nav_target {
            self.load_url(&target);
            redraw = true;
        }

        Update {
            redraw: redraw || !matches!(self.net_client.state, HttpState::Idle | HttpState::Complete | HttpState::Failed),
            close: false,
            install_app: None,
        }
    }

    fn draw(&self, canvas: &mut Canvas<'_>, area: Rect, ctx: &Context) {
        let unit = ctx.unit;
        let cap_style = text_style(unit * 55 / 100, 30);

        draw_panel(canvas, area, unit / 2, PANEL_FILL, 255, PANEL_BORDER, 255);

        let nav_y = area.y + unit;
        let back_btn = Rect::new(area.x + unit, nav_y, unit * 3, unit * 2);
        let go_btn = Rect::new(area.x + area.width - unit * 4, nav_y, unit * 3, unit * 2);
        let url_bar = Rect::new(area.x + unit * 5, nav_y, area.width - unit * 10, unit * 2);

        draw_button(canvas, back_btn, "<", self.hover_back, false);
        draw_input_box(canvas, url_bar, &self.url_input, true);
        draw_button(canvas, go_btn, "GO", self.hover_go, true);

        let view_y = area.y + unit * 4;
        let viewport = Rect::new(area.x + unit, view_y, area.width - unit * 2, area.height - unit * 7);
        draw_panel(canvas, viewport, 2, WHITE, 255, PANEL_BORDER, 255);

        if let Some(ref l) = self.layout_engine {
            for b in &l.boxes {
                paint_box(canvas, b, viewport, self.scroll_y);
            }
        }

        let status_y = area.y + area.height - unit * 2;
        let status_text = match self.net_client.state {
            HttpState::Idle => "READY",
            HttpState::WaitingNetwork => "WAITING FOR NETWORK (DHCP)...",
            HttpState::Handshaking => "TLS HANDSHAKE...",
            HttpState::ResolvingDns => "RESOLVING DNS...",
            HttpState::ConnectingSyn => "CONNECTING (TCP SYN)...",
            HttpState::Connected => "CONNECTED. SENDING REQUEST...",
            HttpState::RequestSent => "WAITING FOR RESPONSE...",
            HttpState::ReceivingData => "RECEIVING HTML PAYLOAD...",
            HttpState::Complete => "DONE",
            HttpState::Failed => "CONNECTION ERROR",
        };

        if let Some(ref h_url) = self.hovered_link {
            let msg = alloc::format!("LINK -> {}", h_url);
            draw_text_color(canvas, &msg, area.x + unit * 2, status_y, &cap_style, ctx.settings.accent_color());
        } else {
            draw_text_color(canvas, status_text, area.x + unit * 2, status_y, &cap_style, TEXT_MUTED);
        }

        let ram_info = alloc::format!("SCROLL: {} PX", self.scroll_y);
        draw_text_color(canvas, &ram_info, area.x + area.width - unit * 12, status_y, &cap_style, TEXT_DIM);
    }
}

fn paint_box(canvas: &mut Canvas<'_>, b: &LayoutBox, viewport: Rect, scroll_y: i32) {
    let screen_y = viewport.y + b.rect.y - scroll_y;
    let screen_x = viewport.x + b.rect.x;

    if screen_y + b.rect.height >= viewport.y && screen_y < viewport.y + viewport.height {
        if let Some(bg) = b.style.bg_color {
            canvas.fill_rect(screen_x, screen_y, b.rect.width, b.rect.height, bg);
        }

        if let Some(ref text) = b.text {
            let font = text_style(b.style.font_size, 30);
            draw_text_color(canvas, text, screen_x, screen_y, &font, b.style.color);
            if b.style.is_link {
                canvas.fill_rect(screen_x, screen_y + b.rect.height - 2, b.rect.width.min(200), 1, b.style.color);
            }
        }
    }

    for child in &b.children {
        paint_box(canvas, child, viewport, scroll_y);
    }
}

fn inside(area: Rect, x: i32, y: i32) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}
