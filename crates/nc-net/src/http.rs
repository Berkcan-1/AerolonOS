extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use nc_arch::rand::HardwareRng;
use crate::tcp::{build_ethernet_frame, build_ipv4_packet, build_tcp_segment, TCP_ACK, TCP_FIN, TCP_PSH, TCP_RST, TCP_SYN};
use crate::tls13::Tls13;
use crate::NetworkStack;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HttpState {
    Idle,
    WaitingNetwork,
    ResolvingDns,
    ConnectingSyn,
    Handshaking,
    Connected,
    RequestSent,
    ReceivingData,
    Complete,
    Failed,
}

pub struct TcpHttpClient {
    pub state: HttpState,
    pub host: String,
    pub path: String,
    pub target_ip: [u8; 4],
    pub local_port: u16,
    pub remote_port: u16,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub response_data: Vec<u8>,
    pub html_body: String,
    pub status_code: u16,
    pub https: bool,
    dns_tx_id: u16,
    tls: Option<Tls13>,
    request_sent: bool,
    ticks: u32,
    retries: u8,
    redirects: u8,
    port_counter: u16,
    pending: Option<String>,
}

impl TcpHttpClient {
    pub fn new() -> Self {
        TcpHttpClient {
            state: HttpState::Idle,
            host: String::new(),
            path: String::new(),
            target_ip: [0, 0, 0, 0],
            local_port: 49152,
            remote_port: 80,
            local_seq: 1000,
            remote_seq: 0,
            response_data: Vec::new(),
            html_body: String::new(),
            status_code: 0,
            https: false,
            dns_tx_id: 0x1A2B,
            tls: None,
            request_sent: false,
            ticks: 0,
            retries: 0,
            redirects: 0,
            port_counter: 0,
            pending: None,
        }
    }

    pub fn navigate(&mut self, url: &str, net: &mut NetworkStack) {
        self.redirects = 0;
        self.start(url, net);
    }

    fn start(&mut self, url: &str, net: &mut NetworkStack) {
        let clean = url.trim();
        let is_https = clean.starts_with("https://");
        let without_proto = if let Some(stripped) = clean.strip_prefix("https://") {
            stripped
        } else if let Some(stripped) = clean.strip_prefix("http://") {
            stripped
        } else {
            clean
        };
        let (host, path) = if let Some(slash_idx) = without_proto.find('/') {
            (&without_proto[..slash_idx], &without_proto[slash_idx..])
        } else {
            (without_proto, "/")
        };

        self.host = String::from(host);
        self.path = String::from(path);
        self.https = is_https;
        self.remote_port = if is_https { 443 } else { 80 };
        self.response_data.clear();
        self.html_body.clear();
        self.tls = None;
        self.request_sent = false;
        self.ticks = 0;
        self.retries = 0;
        self.status_code = 0;
        self.port_counter = self.port_counter.wrapping_add(1);
        self.local_port = 49152 + (self.port_counter % 4000);
        self.local_seq = HardwareRng::next_u64() as u32;
        self.remote_seq = 0;

        if !net.is_ready() {
            self.pending = Some(String::from(url));
            self.state = HttpState::WaitingNetwork;
            return;
        }
        self.begin(net);
    }

    fn begin(&mut self, net: &mut NetworkStack) {
        self.ticks = 0;
        self.retries = 0;
        if let Some(ip) = parse_ipv4(&self.host) {
            self.target_ip = ip;
            self.state = HttpState::ConnectingSyn;
            self.send_syn(net);
        } else {
            self.state = HttpState::ResolvingDns;
            self.send_dns_query(net);
        }
    }

    fn destination_mac(&self, net: &NetworkStack) -> [u8; 6] {
        if net.gateway_mac != [0xFF; 6] && net.gateway_mac != [0; 6] {
            net.gateway_mac
        } else {
            [0x52, 0x55, 0x0A, 0x00, 0x02, 0x02]
        }
    }

    fn send_frame(&mut self, net: &mut NetworkStack, dst_ip: [u8; 4], proto: u8, payload: &[u8]) {
        let ip_pkt = build_ipv4_packet(net.ip_addr, dst_ip, proto, payload);
        let frame = build_ethernet_frame(net.mac_addr, self.destination_mac(net), 0x0800, &ip_pkt);
        if let Some(ref mut drv) = net.driver {
            let _ = drv.send_packet(&frame);
        }
    }

    fn send_dns_query(&mut self, net: &mut NetworkStack) {
        let query = crate::dns::encode_dns_query(&self.host, self.dns_tx_id);
        let udp_pkt = crate::build_udp_packet(net.ip_addr, net.dns_ip, 5353, 53, &query);
        let dns_ip = net.dns_ip;
        self.send_frame(net, dns_ip, 17, &udp_pkt);
    }

    fn send_syn(&mut self, net: &mut NetworkStack) {
        let seg = build_tcp_segment(net.ip_addr, self.target_ip, self.local_port, self.remote_port, self.local_seq, 0, TCP_SYN, &[]);
        let target = self.target_ip;
        self.send_frame(net, target, 6, &seg);
    }

    fn send_ack(&mut self, net: &mut NetworkStack, flags: u8, payload: &[u8]) {
        let seg = build_tcp_segment(net.ip_addr, self.target_ip, self.local_port, self.remote_port, self.local_seq, self.remote_seq, flags, payload);
        let target = self.target_ip;
        self.send_frame(net, target, 6, &seg);
    }

    fn send_data(&mut self, net: &mut NetworkStack, data: &[u8]) {
        let mut offset = 0;
        while offset < data.len() {
            let end = (offset + 1400).min(data.len());
            let chunk = &data[offset..end];
            self.send_ack(net, TCP_ACK | TCP_PSH, chunk);
            self.local_seq = self.local_seq.wrapping_add(chunk.len() as u32);
            offset = end;
        }
    }

    pub fn poll(&mut self, net: &mut NetworkStack) {
        for _ in 0..16 {
            net.poll();
        }
        if matches!(self.state, HttpState::Idle | HttpState::Complete | HttpState::Failed) {
            net.inbox.clear();
            return;
        }
        while !net.inbox.is_empty() {
            let pkt = net.inbox.remove(0);
            self.process_packet(&pkt, net);
        }
        self.tick(net);
    }

    fn tick(&mut self, net: &mut NetworkStack) {
        self.ticks += 1;
        match self.state {
            HttpState::WaitingNetwork => {
                if net.is_ready() {
                    if let Some(url) = self.pending.take() {
                        self.start(&url, net);
                    }
                } else if self.ticks > 1800 {
                    self.state = HttpState::Failed;
                }
            }
            HttpState::ResolvingDns | HttpState::ConnectingSyn => {
                if self.ticks % 90 == 0 {
                    self.retries += 1;
                    if self.retries > 5 {
                        self.state = HttpState::Failed;
                    } else {
                        if net.gateway_mac == [0xFF; 6] {
                            net.resolve_gateway_mac();
                        }
                        if self.state == HttpState::ResolvingDns {
                            self.send_dns_query(net);
                        } else {
                            self.send_syn(net);
                        }
                    }
                }
            }
            _ => {
                if self.ticks > 900 {
                    if self.response_data.is_empty() {
                        self.state = HttpState::Failed;
                    } else {
                        self.finish(net);
                    }
                }
            }
        }
    }

    fn on_established(&mut self, net: &mut NetworkStack) {
        if self.https {
            let mut tls = Tls13::new(&self.host);
            let hello = tls.client_hello();
            self.tls = Some(tls);
            self.state = HttpState::Handshaking;
            self.send_data(net, &hello);
        } else {
            let req = crate::tcp::build_http_get(&self.host, &self.path);
            self.send_data(net, &req);
            self.request_sent = true;
            self.state = HttpState::RequestSent;
        }
    }

    fn on_payload(&mut self, payload: &[u8], net: &mut NetworkStack) {
        if !self.https {
            self.state = HttpState::ReceivingData;
            self.response_data.extend_from_slice(payload);
            return;
        }
        let mut out = Vec::new();
        let (ready, failed, closed, app) = match self.tls.as_mut() {
            Some(tls) => {
                tls.feed(payload, &mut out);
                (tls.is_ready(), tls.is_failed(), tls.closed, core::mem::take(&mut tls.app_in))
            }
            None => return,
        };
        if !out.is_empty() {
            self.send_data(net, &out);
        }
        if failed {
            self.state = HttpState::Failed;
            return;
        }
        if ready && !self.request_sent {
            let req = crate::tcp::build_http_get(&self.host, &self.path);
            let record = match self.tls.as_mut() {
                Some(tls) => tls.encrypt_app(&req),
                None => Vec::new(),
            };
            self.send_data(net, &record);
            self.request_sent = true;
            self.state = HttpState::RequestSent;
        }
        if !app.is_empty() {
            self.state = HttpState::ReceivingData;
            self.response_data.extend_from_slice(&app);
        }
        if closed {
            self.send_ack(net, TCP_ACK | TCP_FIN, &[]);
            self.local_seq = self.local_seq.wrapping_add(1);
            self.finish(net);
        }
    }

    fn process_packet(&mut self, pkt: &[u8], net: &mut NetworkStack) {
        if pkt.len() < 34 || u16::from_be_bytes([pkt[12], pkt[13]]) != 0x0800 {
            return;
        }
        let ip_len = u16::from_be_bytes([pkt[16], pkt[17]]) as usize;
        let end = (14 + ip_len).min(pkt.len());
        if end < 34 {
            return;
        }
        let proto = pkt[23];

        if proto == 17 {
            if self.state != HttpState::ResolvingDns || end < 42 {
                return;
            }
            if u16::from_be_bytes([pkt[36], pkt[37]]) != 5353 {
                return;
            }
            if let Some(ip) = crate::dns::parse_dns_response(&pkt[42..end]) {
                self.target_ip = ip;
                self.state = HttpState::ConnectingSyn;
                self.ticks = 0;
                self.retries = 0;
                self.send_syn(net);
            }
            return;
        }

        if proto != 6 || end < 54 {
            return;
        }
        let src_port = u16::from_be_bytes([pkt[34], pkt[35]]);
        let dst_port = u16::from_be_bytes([pkt[36], pkt[37]]);
        if src_port != self.remote_port || dst_port != self.local_port {
            return;
        }
        let seq_in = u32::from_be_bytes([pkt[38], pkt[39], pkt[40], pkt[41]]);
        let flags = pkt[47];
        let payload_start = 34 + ((pkt[46] >> 4) as usize) * 4;
        let payload: &[u8] = if end > payload_start { &pkt[payload_start..end] } else { &[] };

        if flags & TCP_RST != 0 {
            self.state = HttpState::Failed;
            return;
        }
        if self.state == HttpState::ConnectingSyn {
            if flags & (TCP_SYN | TCP_ACK) == (TCP_SYN | TCP_ACK) {
                self.remote_seq = seq_in.wrapping_add(1);
                self.local_seq = self.local_seq.wrapping_add(1);
                self.ticks = 0;
                self.retries = 0;
                self.on_established(net);
            }
            return;
        }
        if matches!(self.state, HttpState::Idle | HttpState::Complete | HttpState::Failed | HttpState::ResolvingDns | HttpState::WaitingNetwork) {
            return;
        }

        if !payload.is_empty() {
            if seq_in != self.remote_seq {
                self.send_ack(net, TCP_ACK, &[]);
                return;
            }
            self.remote_seq = seq_in.wrapping_add(payload.len() as u32);
            self.ticks = 0;
            self.send_ack(net, TCP_ACK, &[]);
            self.on_payload(payload, net);
            if matches!(self.state, HttpState::Complete | HttpState::Failed) {
                return;
            }
        }

        if flags & TCP_FIN != 0 && seq_in.wrapping_add(payload.len() as u32) == self.remote_seq {
            self.remote_seq = self.remote_seq.wrapping_add(1);
            self.send_ack(net, TCP_ACK | TCP_FIN, &[]);
            self.local_seq = self.local_seq.wrapping_add(1);
            self.finish(net);
        }
    }

    fn resolve_location(&self, location: &str) -> String {
        let scheme = if self.https { "https://" } else { "http://" };
        if location.starts_with("http://") || location.starts_with("https://") {
            String::from(location)
        } else if let Some(rest) = location.strip_prefix("//") {
            alloc::format!("{}{}", scheme, rest)
        } else if location.starts_with('/') {
            alloc::format!("{}{}{}", scheme, self.host, location)
        } else {
            alloc::format!("{}{}/{}", scheme, self.host, location)
        }
    }

    fn finish(&mut self, net: &mut NetworkStack) {
        let (status, body) = parse_http_payload(&self.response_data);
        self.status_code = status;
        if matches!(status, 301 | 302 | 303 | 307 | 308) && self.redirects < 5 {
            if let Some(location) = header_value(&self.response_data, "location") {
                let url = self.resolve_location(&location);
                self.redirects += 1;
                self.start(&url, net);
                return;
            }
        }
        if !body.is_empty() {
            self.html_body = body;
        } else {
            self.html_body = String::from_utf8_lossy(&self.response_data).into_owned();
        }
        self.state = HttpState::Complete;
    }

    pub fn decode_chunked_body(&self, data: &[u8]) -> Vec<u8> {
        decode_chunked_body(data)
    }
}

pub fn header_value(raw: &[u8], name: &str) -> Option<String> {
    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n")?;
    let text = core::str::from_utf8(&raw[..header_end]).ok()?;
    for line in text.lines().skip(1) {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case(name) {
                return Some(String::from(value.trim()));
            }
        }
    }
    None
}

pub fn parse_http_payload(raw_response: &[u8]) -> (u16, String) {
    let header_end = match raw_response.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(pos) => pos,
        None => return (0, String::new()),
    };

    let header_str = core::str::from_utf8(&raw_response[..header_end]).unwrap_or("");
    let body_bytes = &raw_response[header_end + 4..];

    let status_code = header_str
        .lines()
        .next()
        .and_then(|line| {
            let mut parts = line.split_whitespace();
            parts.next()?;
            parts.next()?.parse::<u16>().ok()
        })
        .unwrap_or(0);

    let is_chunked = header_str.to_ascii_lowercase().contains("transfer-encoding: chunked");

    let clean_body = if is_chunked {
        decode_chunked_body(body_bytes)
    } else {
        body_bytes.to_vec()
    };

    let html_string = String::from_utf8_lossy(&clean_body).into_owned();
    (status_code, html_string)
}

pub fn decode_chunked_body(mut data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    loop {
        let crlf = match data.windows(2).position(|w| w == b"\r\n") {
            Some(pos) => pos,
            None => break,
        };

        let hex_str = core::str::from_utf8(&data[..crlf]).unwrap_or("0");
        let chunk_size = usize::from_str_radix(hex_str.trim(), 16).unwrap_or(0);

        if chunk_size == 0 {
            break;
        }

        let chunk_start = crlf + 2;
        let chunk_end = chunk_start + chunk_size;

        if chunk_end > data.len() {
            result.extend_from_slice(&data[chunk_start..]);
            break;
        }

        result.extend_from_slice(&data[chunk_start..chunk_end]);
        data = if chunk_end + 2 <= data.len() {
            &data[chunk_end + 2..]
        } else {
            &[]
        };
    }
    result
}

pub fn parse_ipv4(s: &str) -> Option<[u8; 4]> {
    let mut parts = [0u8; 4];
    let mut count = 0;
    for segment in s.split('.') {
        if count >= 4 { return None; }
        parts[count] = segment.parse::<u8>().ok()?;
        count += 1;
    }
    if count == 4 { Some(parts) } else { None }
}
