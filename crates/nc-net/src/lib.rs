#![no_std]
extern crate alloc;

pub mod dhcp;
pub mod dns;
pub mod http;
pub mod tcp;
pub mod tls;
pub mod tls13;
pub mod tls_engine;

use alloc::vec;
use alloc::vec::Vec;
use nc_arch::{inl, outl};

pub use dhcp::{DhcpAction, DhcpClient, DhcpState};
pub use dns::{encode_dns_query, parse_dns_response};
pub use http::{HttpState, TcpHttpClient};
pub use tcp::{build_ethernet_frame, build_ipv4_packet, build_tcp_segment, TcpSocket, TcpState, TCP_ACK, TCP_FIN, TCP_PSH, TCP_SYN};
pub use tls::{build_tls_client_hello, parse_tls_record};

pub const E1000_VENDOR_ID: u16 = 0x8086;
pub const E1000_DEVICE_ID: u16 = 0x100E;

pub fn build_udp_packet(
    _src_ip: [u8; 4],
    _dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(8 + payload.len());
    pkt.extend_from_slice(&src_port.to_be_bytes());
    pkt.extend_from_slice(&dst_port.to_be_bytes());
    let len = (8 + payload.len()) as u16;
    pkt.extend_from_slice(&len.to_be_bytes());
    pkt.extend_from_slice(&[0, 0]);
    pkt.extend_from_slice(payload);
    pkt
}

fn pci_read_config_32(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC);
    unsafe {
        outl(0xCF8, address);
        inl(0xCFC)
    }
}

fn pci_write_config_32(bus: u8, dev: u8, func: u8, offset: u8, val: u32) {
    let address = 0x8000_0000u32
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC);
    unsafe {
        outl(0xCF8, address);
        outl(0xCFC, val);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PciDeviceInfo {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bar0: u32,
    pub irq: u8,
}

pub fn scan_pci_e1000() -> Option<PciDeviceInfo> {
    for bus in 0..=255 {
        for dev in 0..32 {
            for func in 0..8 {
                let vendor_dev = pci_read_config_32(bus, dev, func, 0x00);
                let vendor_id = (vendor_dev & 0xFFFF) as u16;
                let device_id = ((vendor_dev >> 16) & 0xFFFF) as u16;

                if vendor_id == E1000_VENDOR_ID && device_id == E1000_DEVICE_ID {
                    let bar0 = pci_read_config_32(bus, dev, func, 0x10);
                    let irq_info = pci_read_config_32(bus, dev, func, 0x3C);
                    let irq = (irq_info & 0xFF) as u8;

                    let cmd = pci_read_config_32(bus, dev, func, 0x04);
                    pci_write_config_32(bus, dev, func, 0x04, cmd | 0x07);

                    return Some(PciDeviceInfo {
                        bus,
                        dev,
                        func,
                        vendor_id,
                        device_id,
                        bar0: bar0 & 0xFFFF_FFF0,
                        irq,
                    });
                }
            }
        }
    }
    None
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct RxDesc {
    pub addr: u64,
    pub length: u16,
    pub checksum: u16,
    pub status: u8,
    pub errors: u8,
    pub special: u16,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct TxDesc {
    pub addr: u64,
    pub length: u16,
    pub cso: u8,
    pub cmd: u8,
    pub status: u8,
    pub css: u8,
    pub special: u16,
}

pub struct E1000Driver {
    pub pci: PciDeviceInfo,
    pub mmio_base: usize,
    pub mac_addr: [u8; 6],
    pub rx_descs: Vec<RxDesc>,
    pub tx_descs: Vec<TxDesc>,
    pub rx_buffers: Vec<Vec<u8>>,
    pub tx_buffers: Vec<Vec<u8>>,
    pub rx_cur: usize,
    pub tx_cur: usize,
}

impl E1000Driver {
    pub fn new(pci: PciDeviceInfo) -> Self {
        let mmio_base = pci.bar0 as usize;
        let mut mac_addr = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        if mmio_base != 0 {
            let mac_low = unsafe { core::ptr::read_volatile((mmio_base + 0x5400) as *const u32) };
            let mac_high = unsafe { core::ptr::read_volatile((mmio_base + 0x5404) as *const u32) };
            if mac_low != 0 || mac_high != 0 {
                mac_addr[0] = (mac_low & 0xFF) as u8;
                mac_addr[1] = ((mac_low >> 8) & 0xFF) as u8;
                mac_addr[2] = ((mac_low >> 16) & 0xFF) as u8;
                mac_addr[3] = ((mac_low >> 24) & 0xFF) as u8;
                mac_addr[4] = (mac_high & 0xFF) as u8;
                mac_addr[5] = ((mac_high >> 8) & 0xFF) as u8;
            }
        }
        let num_rx = 64;
        let num_tx = 16;
        let mut rx_descs = Vec::with_capacity(num_rx);
        let mut tx_descs = Vec::with_capacity(num_tx);
        let mut rx_buffers = Vec::with_capacity(num_rx);
        let mut tx_buffers = Vec::with_capacity(num_tx);
        for _ in 0..num_rx {
            let mut buf = Vec::with_capacity(2048);
            buf.resize(2048, 0u8);
            let ptr = buf.as_ptr() as u64;
            rx_descs.push(RxDesc {
                addr: ptr,
                length: 0,
                checksum: 0,
                status: 0,
                errors: 0,
                special: 0,
            });
            rx_buffers.push(buf);
        }
        for _ in 0..num_tx {
            let mut buf = Vec::with_capacity(2048);
            buf.resize(2048, 0u8);
            let ptr = buf.as_ptr() as u64;
            tx_descs.push(TxDesc {
                addr: ptr,
                length: 0,
                cso: 0,
                cmd: 0,
                status: 1,
                css: 0,
                special: 0,
            });
            tx_buffers.push(buf);
        }

        if mmio_base != 0 {
            unsafe {
                let ctrl = core::ptr::read_volatile(mmio_base as *const u32);
                core::ptr::write_volatile(mmio_base as *mut u32, ctrl | (1 << 6) | (1 << 5));
                core::ptr::write_volatile((mmio_base + 0x00D8) as *mut u32, 0xFFFF_FFFF);
                let rx_base_ptr = rx_descs.as_ptr() as u32;
                core::ptr::write_volatile((mmio_base + 0x2800) as *mut u32, rx_base_ptr);
                core::ptr::write_volatile((mmio_base + 0x2804) as *mut u32, 0);
                core::ptr::write_volatile((mmio_base + 0x2808) as *mut u32, (num_rx * 16) as u32);
                core::ptr::write_volatile((mmio_base + 0x2810) as *mut u32, 0);
                core::ptr::write_volatile((mmio_base + 0x2818) as *mut u32, (num_rx - 1) as u32);
                core::ptr::write_volatile((mmio_base + 0x0100) as *mut u32, (1 << 1) | (1 << 15) | (1 << 26));

                let tx_base_ptr = tx_descs.as_ptr() as u32;
                core::ptr::write_volatile((mmio_base + 0x3800) as *mut u32, tx_base_ptr);
                core::ptr::write_volatile((mmio_base + 0x3804) as *mut u32, 0);
                core::ptr::write_volatile((mmio_base + 0x3808) as *mut u32, (num_tx * 16) as u32);
                core::ptr::write_volatile((mmio_base + 0x3810) as *mut u32, 0);
                core::ptr::write_volatile((mmio_base + 0x3818) as *mut u32, 0);
                core::ptr::write_volatile((mmio_base + 0x0400) as *mut u32, (1 << 1) | (1 << 3) | (0x0F << 4));
            }
        }

        E1000Driver {
            pci,
            mmio_base,
            mac_addr,
            rx_descs,
            tx_descs,
            rx_buffers,
            tx_buffers,
            rx_cur: 0,
            tx_cur: 0,
        }
    }

    pub fn send_packet(&mut self, data: &[u8]) -> Result<(), ()> {
        if data.len() > 2048 {
            return Err(());
        }
        let idx = self.tx_cur;
        self.tx_buffers[idx][..data.len()].copy_from_slice(data);
        self.tx_descs[idx].length = data.len() as u16;
        self.tx_descs[idx].cmd = (1 << 0) | (1 << 1) | (1 << 3);
        self.tx_descs[idx].status = 0;
        self.tx_cur = (self.tx_cur + 1) % self.tx_descs.len();
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        if self.mmio_base != 0 {
            unsafe {
                core::ptr::write_volatile((self.mmio_base + 0x3818) as *mut u32, self.tx_cur as u32);
            }
        }
        Ok(())
    }

    pub fn poll_rx(&mut self) -> Option<Vec<u8>> {
        let idx = self.rx_cur;
        let status = unsafe { core::ptr::read_volatile(&self.rx_descs[idx].status) };
        if (status & 1) != 0 {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            let len = (unsafe { core::ptr::read_volatile(&self.rx_descs[idx].length) } as usize).min(2048);
            let packet = self.rx_buffers[idx][..len].to_vec();
            self.rx_descs[idx].status = 0;
            self.rx_cur = (self.rx_cur + 1) % self.rx_descs.len();
            if self.mmio_base != 0 {
                unsafe {
                    core::ptr::write_volatile((self.mmio_base + 0x2818) as *mut u32, idx as u32);
                }
            }
            Some(packet)
        } else {
            None
        }
    }
}

pub struct NetworkStack {
    pub driver: Option<E1000Driver>,
    pub ip_addr: [u8; 4],
    pub gateway_ip: [u8; 4],
    pub gateway_mac: [u8; 6],
    pub dns_ip: [u8; 4],
    pub mac_addr: [u8; 6],
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub link_online: bool,
    pub dhcp: DhcpClient,
    pub inbox: Vec<Vec<u8>>,
    pub dhcp_wait: u32,
}

impl NetworkStack {
    pub fn new() -> Self {
        let mut driver = None;
        let mut mac_addr = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
        let mut link_online = false;

        if let Some(pci) = scan_pci_e1000() {
            let drv = E1000Driver::new(pci);
            mac_addr = drv.mac_addr;
            driver = Some(drv);
            link_online = true;
        }

        let mut dhcp = DhcpClient::new();
        let initial_tx = 0u64;

        let mut pending_discover: Option<Vec<u8>> = None;
        if link_online {
            pending_discover = Some(dhcp.build_discover(mac_addr));
        }

        let mut stack = NetworkStack {
            driver,
            ip_addr: [0, 0, 0, 0],
            gateway_ip: [0, 0, 0, 0],
            gateway_mac: [0xFF; 6],
            dns_ip: [1, 1, 1, 1],
            mac_addr,
            rx_packets: 0,
            tx_packets: initial_tx,
            link_online,
            dhcp,
            inbox: Vec::new(),
            dhcp_wait: 0,
        };

        if let Some(pkt) = pending_discover {
            if let Some(ref mut drv) = stack.driver {
                let _ = drv.send_packet(&pkt);
                stack.tx_packets += 1;
            }
        }

        stack
    }

    pub fn resolve_gateway_mac(&mut self) {
        if self.gateway_ip == [0, 0, 0, 0] {
            return;
        }
        let mut arp_pkt = [0u8; 42];
        arp_pkt[0..6].copy_from_slice(&[0xFF; 6]);
        arp_pkt[6..12].copy_from_slice(&self.mac_addr);
        arp_pkt[12..14].copy_from_slice(&[0x08, 0x06]);
        arp_pkt[14..16].copy_from_slice(&[0x00, 0x01]);
        arp_pkt[16..18].copy_from_slice(&[0x08, 0x00]);
        arp_pkt[18] = 6;
        arp_pkt[19] = 4;
        arp_pkt[20..22].copy_from_slice(&[0x00, 0x01]);
        arp_pkt[22..28].copy_from_slice(&self.mac_addr);
        arp_pkt[28..32].copy_from_slice(&self.ip_addr);
        arp_pkt[32..38].copy_from_slice(&[0x00; 6]);
        arp_pkt[38..42].copy_from_slice(&self.gateway_ip);

        if let Some(ref mut drv) = self.driver {
            let _ = drv.send_packet(&arp_pkt);
            self.tx_packets += 1;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ip_addr != [0, 0, 0, 0]
    }

    fn retry_dhcp(&mut self) {
        if !self.link_online || self.dhcp.state == DhcpState::Bound {
            return;
        }
        self.dhcp_wait += 1;
        if self.dhcp_wait >= 300 {
            self.dhcp_wait = 0;
            let pkt = self.dhcp.build_discover(self.mac_addr);
            if let Some(ref mut drv) = self.driver {
                let _ = drv.send_packet(&pkt);
                self.tx_packets += 1;
            }
        }
    }

    pub fn poll(&mut self) {
        self.retry_dhcp();
        let rx_opt = if let Some(ref mut drv) = self.driver {
            drv.poll_rx()
        } else {
            None
        };

        let rx = match rx_opt {
            Some(pkt) => pkt,
            None => return,
        };

        self.rx_packets += 1;
        if rx.len() < 14 {
            return;
        }

        let eth_type = u16::from_be_bytes([rx[12], rx[13]]);

        if eth_type == 0x0806 && rx.len() >= 42 {
            let opcode = u16::from_be_bytes([rx[20], rx[21]]);
            if opcode == 1 {
                let target_ip = [rx[38], rx[39], rx[40], rx[41]];
                if self.ip_addr != [0, 0, 0, 0] && target_ip == self.ip_addr {
                    let mut arp_reply = vec![0u8; 42];
                    arp_reply[0..6].copy_from_slice(&rx[6..12]);
                    arp_reply[6..12].copy_from_slice(&self.mac_addr);
                    arp_reply[12..14].copy_from_slice(&[0x08, 0x06]);
                    arp_reply[14..16].copy_from_slice(&[0x00, 0x01]);
                    arp_reply[16..18].copy_from_slice(&[0x08, 0x00]);
                    arp_reply[18] = 6;
                    arp_reply[19] = 4;
                    arp_reply[20..22].copy_from_slice(&[0x00, 0x02]);
                    arp_reply[22..28].copy_from_slice(&self.mac_addr);
                    arp_reply[28..32].copy_from_slice(&self.ip_addr);
                    arp_reply[32..38].copy_from_slice(&rx[22..28]);
                    arp_reply[38..42].copy_from_slice(&rx[28..32]);

                    if let Some(ref mut drv) = self.driver {
                        let _ = drv.send_packet(&arp_reply);
                        self.tx_packets += 1;
                    }
                }
            } else if opcode == 2 {
                let sender_ip = [rx[28], rx[29], rx[30], rx[31]];
                if sender_ip == self.gateway_ip && self.gateway_ip != [0, 0, 0, 0] {
                    self.gateway_mac.copy_from_slice(&rx[22..28]);
                }
            }
            return;
        }

        if eth_type == 0x0800 && rx.len() >= 34 {
            let proto = rx[23];

            if proto == 17 && self.dhcp.state != DhcpState::Bound {
                if let Some(action) = self.dhcp.handle_packet(&rx, self.mac_addr) {
                    match action {
                        DhcpAction::SendRequest(req_pkt) => {
                            if let Some(ref mut drv) = self.driver {
                                let _ = drv.send_packet(&req_pkt);
                                self.tx_packets += 1;
                            }
                        }
                        DhcpAction::Bound { ip, gateway, dns } => {
                            self.ip_addr = ip;
                            if gateway != [0, 0, 0, 0] {
                                self.gateway_ip = gateway;
                            }
                            if dns != [0, 0, 0, 0] {
                                self.dns_ip = dns;
                            }
                            self.resolve_gateway_mac();
                        }
                    }
                    return;
                }
            }

            if proto == 1 && rx.len() >= 42 {
                let icmp_type = rx[34];
                if icmp_type == 8 {
                    let mut ping_reply = rx.clone();
                    ping_reply[0..6].copy_from_slice(&rx[6..12]);
                    ping_reply[6..12].copy_from_slice(&self.mac_addr);
                    ping_reply[26..30].copy_from_slice(&rx[30..34]);
                    ping_reply[30..34].copy_from_slice(&rx[26..30]);
                    ping_reply[34] = 0;
                    ping_reply[36..38].copy_from_slice(&[0x00, 0x00]);
                    if let Some(ref mut drv) = self.driver {
                        let _ = drv.send_packet(&ping_reply);
                        self.tx_packets += 1;
                    }
                }
            }
            if proto == 6 || proto == 17 {
                if self.inbox.len() >= 64 {
                    self.inbox.remove(0);
                }
                self.inbox.push(rx);
            }
        }
    }
}

static mut GLOBAL_STACK: Option<NetworkStack> = None;

pub fn global() -> &'static mut NetworkStack {
    unsafe {
        let slot = &mut *core::ptr::addr_of_mut!(GLOBAL_STACK);
        if slot.is_none() {
            *slot = Some(NetworkStack::new());
        }
        slot.as_mut().unwrap()
    }
}
