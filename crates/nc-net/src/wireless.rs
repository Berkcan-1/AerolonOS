extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use nc_arch::{inb, inl, outb, outl};

const COM2_PORT: u16 = 0x02F8;

#[derive(Clone, Debug)]
pub struct WifiNetworkInfo {
    pub ssid: String,
    pub channel: u8,
    pub freq_label: String,
    pub signal_pct: u8,
    pub security: String,
    pub connected: bool,
}

impl WifiNetworkInfo {
    pub fn bars(&self) -> u8 {
        match self.signal_pct {
            0..=24 => 1,
            25..=49 => 2,
            50..=74 => 3,
            _ => 4,
        }
    }

    pub fn is_open(&self) -> bool {
        self.security.is_empty()
            || self.security == "--"
            || self.security.eq_ignore_ascii_case("none")
            || self.security.eq_ignore_ascii_case("acik")
    }
}

#[derive(Clone, Debug)]
pub struct BluetoothDeviceInfo {
    pub mac: String,
    pub name: String,
    pub kind: String,
    pub battery: u8,
    pub connected: bool,
}

#[derive(Clone, Copy)]
pub struct PciRadioHardware {
    pub wifi_pci_found: bool,
    pub wifi_vendor: u16,
    pub wifi_device: u16,
    pub wifi_bus: u8,
    pub wifi_dev: u8,
    pub wifi_irq: u8,
    pub wifi_bar0: u32,
    pub bt_controller_found: bool,
    pub bt_vendor: u16,
    pub bt_device: u16,
}

fn pci_read(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
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

pub fn scan_pci_radio_hardware() -> PciRadioHardware {
    let mut hw = PciRadioHardware {
        wifi_pci_found: false,
        wifi_vendor: 0,
        wifi_device: 0,
        wifi_bus: 0,
        wifi_dev: 0,
        wifi_irq: 0,
        wifi_bar0: 0,
        bt_controller_found: false,
        bt_vendor: 0,
        bt_device: 0,
    };

    for bus in 0..=32u8 {
        for dev in 0..32u8 {
            for func in 0..8u8 {
                let vd = pci_read(bus, dev, func, 0x00);
                let vendor = (vd & 0xFFFF) as u16;
                let device = ((vd >> 16) & 0xFFFF) as u16;
                if vendor == 0xFFFF || vendor == 0x0000 {
                    if func == 0 {
                        break;
                    }
                    continue;
                }
                let class_reg = pci_read(bus, dev, func, 0x08);
                let base_class = ((class_reg >> 24) & 0xFF) as u8;
                let sub_class = ((class_reg >> 16) & 0xFF) as u8;

                if base_class == 0x02 && sub_class == 0x80 && !hw.wifi_pci_found {
                    hw.wifi_pci_found = true;
                    hw.wifi_vendor = vendor;
                    hw.wifi_device = device;
                    hw.wifi_bus = bus;
                    hw.wifi_dev = dev;
                    hw.wifi_bar0 = pci_read(bus, dev, func, 0x10) & 0xFFFF_FFF0;
                    hw.wifi_irq = (pci_read(bus, dev, func, 0x3C) & 0xFF) as u8;
                }

                if (base_class == 0x0D || (base_class == 0x0C && sub_class == 0x03))
                    && !hw.bt_controller_found
                {
                    hw.bt_controller_found = true;
                    hw.bt_vendor = vendor;
                    hw.bt_device = device;
                }
            }
        }
    }
    hw
}

fn pci_wifi_model_name(vendor: u16, device: u16) -> &'static str {
    match (vendor, device) {
        (0x8086, 0x2723) => "Intel Wi-Fi 6 AX200 160MHz",
        (0x8086, 0x06F0) | (0x8086, 0xA0F0) | (0x8086, 0x51F0) | (0x8086, 0x51F1) => {
            "Intel Wi-Fi 6 AX201 / AX211 160MHz"
        }
        (0x8086, 0x2725) => "Intel Wi-Fi 6E AX210 160MHz",
        (0x8086, 0x2526) | (0x8086, 0x9DF0) => "Intel Wireless-AC 9260 / 9560",
        (0x14C3, 0x7961) | (0x14C3, 0x0608) => "MediaTek MT7921 / RZ608 Wi-Fi 6",
        (0x10EC, 0xC821) | (0x10EC, 0xB822) | (0x10EC, 0x8852) => {
            "Realtek 802.11ac / 802.11ax PCIe"
        }
        (0x168C, _) => "Qualcomm Atheros 802.11 Kablosuz",
        (0x14E4, _) => "Broadcom 802.11 Kablosuz",
        _ => "PCI 802.11 Kablosuz Ag Denetleyicisi",
    }
}

pub struct WirelessManager {
    pub pci_hw: PciRadioHardware,
    pub uart_present: bool,
    pub bridge_online: bool,
    pub wifi_adapter_name: String,
    pub bt_adapter_name: String,
    pub wifi_enabled: bool,
    pub bt_enabled: bool,
    pub wifi_scanning: bool,
    pub bt_scanning: bool,
    pub wifi_networks: Vec<WifiNetworkInfo>,
    pub bt_devices: Vec<BluetoothDeviceInfo>,
    pub status_message: String,
    rx_line: Vec<u8>,
    staging_wifi: Vec<WifiNetworkInfo>,
    staging_bt: Vec<BluetoothDeviceInfo>,
    poll_ticks: u32,
}

impl WirelessManager {
    pub fn new() -> Self {
        let pci_hw = scan_pci_radio_hardware();
        let uart_present = Self::init_com2_uart();

        let wifi_name = if pci_hw.wifi_pci_found {
            String::from(pci_wifi_model_name(pci_hw.wifi_vendor, pci_hw.wifi_device))
        } else {
            String::from("Fiziksel 802.11 PCI bagdastirici bulunamadi")
        };

        let bt_name = if pci_hw.bt_controller_found {
            String::from("USB / PCI Bluetooth HCI Denetleyicisi")
        } else {
            String::from("Bluetooth HCI denetleyicisi bulunamadi")
        };

        let mut mgr = WirelessManager {
            pci_hw,
            uart_present,
            bridge_online: false,
            wifi_adapter_name: wifi_name,
            bt_adapter_name: bt_name,
            wifi_enabled: true,
            bt_enabled: true,
            wifi_scanning: false,
            bt_scanning: false,
            wifi_networks: Vec::new(),
            bt_devices: Vec::new(),
            status_message: String::from("Radyo donanimi sorgulaniyor..."),
            rx_line: Vec::with_capacity(256),
            staging_wifi: Vec::new(),
            staging_bt: Vec::new(),
            poll_ticks: 0,
        };

        if uart_present {
            mgr.send_command("CMD:INIT");
            mgr.send_command("CMD:WIFI_SCAN");
            mgr.send_command("CMD:BT_SCAN");
            mgr.wifi_scanning = true;
            mgr.bt_scanning = true;
        } else if !pci_hw.wifi_pci_found {
            mgr.status_message =
                String::from("Bu makinede kablosuz donanim veya radyo koprusu algilanmadi.");
        }

        mgr
    }

    fn init_com2_uart() -> bool {
        unsafe {
            outb(COM2_PORT + 1, 0x00);
            outb(COM2_PORT + 3, 0x80);
            outb(COM2_PORT + 0, 0x01);
            outb(COM2_PORT + 1, 0x00);
            outb(COM2_PORT + 3, 0x03);
            outb(COM2_PORT + 2, 0xC7);
            outb(COM2_PORT + 4, 0x0B);
            outb(COM2_PORT + 7, 0xA5);
            inb(COM2_PORT + 7) == 0xA5
        }
    }

    pub fn send_command(&mut self, cmd: &str) {
        if !self.uart_present {
            return;
        }
        for b in cmd.bytes() {
            Self::write_com2_byte(b);
        }
        Self::write_com2_byte(b'\n');
    }

    fn write_com2_byte(byte: u8) {
        unsafe {
            let mut spins = 0u32;
            while (inb(COM2_PORT + 5) & 0x20) == 0 {
                spins += 1;
                if spins > 50_000 {
                    return;
                }
            }
            outb(COM2_PORT, byte);
        }
    }

    pub fn request_wifi_scan(&mut self) {
        if !self.wifi_enabled {
            self.status_message = String::from("Kablosuz alici kapali.");
            return;
        }
        self.wifi_scanning = true;
        self.status_message = String::from("Cevredeki gercek 802.11 aglari taraniyor...");
        self.send_command("CMD:WIFI_SCAN");
    }

    pub fn request_bt_scan(&mut self) {
        if !self.bt_enabled {
            self.status_message = String::from("Bluetooth denetleyicisi kapali.");
            return;
        }
        self.bt_scanning = true;
        self.status_message = String::from("Bluetooth HCI sorgusu (Inquiry) yurutuluyor...");
        self.send_command("CMD:BT_SCAN");
    }

    pub fn set_wifi_enabled(&mut self, enabled: bool) {
        self.wifi_enabled = enabled;
        if enabled {
            self.send_command("CMD:WIFI_ON");
            self.request_wifi_scan();
        } else {
            self.send_command("CMD:WIFI_OFF");
            self.wifi_networks.clear();
            self.wifi_scanning = false;
            self.status_message = String::from("802.11 radyo vericisi kapatildi.");
        }
    }

    pub fn set_bt_enabled(&mut self, enabled: bool) {
        self.bt_enabled = enabled;
        if enabled {
            self.send_command("CMD:BT_ON");
            self.request_bt_scan();
        } else {
            self.send_command("CMD:BT_OFF");
            self.bt_devices.clear();
            self.bt_scanning = false;
            self.status_message = String::from("Bluetooth HCI radyosu kapatildi.");
        }
    }

    pub fn connect_wifi(&mut self, ssid: &str, password: &str) {
        self.status_message = alloc::format!("'{}' agina baglaniliyor...", ssid);
        let cmd = alloc::format!("CMD:WIFI_CONN|{}|{}", ssid, password);
        self.send_command(&cmd);
    }

    pub fn disconnect_wifi(&mut self, ssid: &str) {
        self.status_message = alloc::format!("'{}' ag baglantisi kesiliyor...", ssid);
        let cmd = alloc::format!("CMD:WIFI_DISC|{}", ssid);
        self.send_command(&cmd);
    }

    pub fn toggle_bt_device(&mut self, idx: usize) {
        if idx >= self.bt_devices.len() {
            return;
        }
        let mac = self.bt_devices[idx].mac.clone();
        let name = self.bt_devices[idx].name.clone();
        let was_conn = self.bt_devices[idx].connected;
        if was_conn {
            self.status_message = alloc::format!("'{}' baglantisi kesiliyor...", name);
            let cmd = alloc::format!("CMD:BT_DISC|{}", mac);
            self.send_command(&cmd);
        } else {
            self.status_message = alloc::format!("'{}' ile HCI baglantisi kuruluyor...", name);
            let cmd = alloc::format!("CMD:BT_CONN|{}", mac);
            self.send_command(&cmd);
        }
    }

    pub fn poll(&mut self) {
        if !self.uart_present {
            return;
        }
        self.poll_ticks = self.poll_ticks.wrapping_add(1);
        let mut budget = 2048usize;
        while budget > 0 {
            budget -= 1;
            let lsr = unsafe { inb(COM2_PORT + 5) };
            if (lsr & 0x01) == 0 {
                break;
            }
            let byte = unsafe { inb(COM2_PORT) };
            if byte == b'\r' {
                continue;
            }
            if byte == b'\n' {
                if !self.rx_line.is_empty() {
                    let line_bytes = core::mem::take(&mut self.rx_line);
                    if let Ok(line_str) = core::str::from_utf8(&line_bytes) {
                        self.process_line(line_str.trim());
                    }
                }
            } else if self.rx_line.len() < 512 {
                self.rx_line.push(byte);
            }
        }
    }

    fn process_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        self.bridge_online = true;

        if let Some(rest) = line.strip_prefix("H|") {
            let mut parts = rest.split('|');
            if let Some(w_hw) = parts.next() {
                if !w_hw.trim().is_empty() {
                    self.wifi_adapter_name = String::from(w_hw.trim());
                }
            }
            if let Some(b_hw) = parts.next() {
                if !b_hw.trim().is_empty() {
                    self.bt_adapter_name = String::from(b_hw.trim());
                }
            }
            if let Some(w_on) = parts.next() {
                self.wifi_enabled = w_on.trim() == "1";
            }
            if let Some(b_on) = parts.next() {
                self.bt_enabled = b_on.trim() == "1";
            }
        } else if line == "W_CLEAR" {
            self.staging_wifi.clear();
        } else if let Some(rest) = line.strip_prefix("W|") {
            let mut parts = rest.split('|');
            let in_use = parts.next().unwrap_or("0").trim() == "1";
            let ssid = parts.next().unwrap_or("").trim();
            let chan = parts
                .next()
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(1);
            let freq = parts.next().unwrap_or("2.4 GHz").trim();
            let signal = parts
                .next()
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(50)
                .min(100);
            let sec = parts.next().unwrap_or("WPA2").trim();

            if !ssid.is_empty() && ssid != "--" {
                if let Some(existing) = self.staging_wifi.iter_mut().find(|n| n.ssid == ssid) {
                    if in_use {
                        existing.connected = true;
                    }
                    if signal > existing.signal_pct {
                        existing.signal_pct = signal;
                    }
                } else if self.staging_wifi.len() < 8 {
                    self.staging_wifi.push(WifiNetworkInfo {
                        ssid: String::from(ssid),
                        channel: chan,
                        freq_label: String::from(freq),
                        signal_pct: signal,
                        security: String::from(if sec.is_empty() { "Acik" } else { sec }),
                        connected: in_use,
                    });
                }
            }
        } else if line == "W_END" {
            self.wifi_networks = self.staging_wifi.clone();
            self.wifi_scanning = false;
            self.status_message = alloc::format!(
                "Tarama tamamlandi: {} gercek kablosuz ag bulundu.",
                self.wifi_networks.len()
            );
        } else if line == "B_CLEAR" {
            self.staging_bt.clear();
        } else if let Some(rest) = line.strip_prefix("B|") {
            let mut parts = rest.split('|');
            let conn = parts.next().unwrap_or("0").trim() == "1";
            let mac = parts.next().unwrap_or("").trim();
            let kind = parts.next().unwrap_or("Bluetooth Cihazi").trim();
            let batt = parts
                .next()
                .and_then(|s| s.trim().parse::<u8>().ok())
                .unwrap_or(0);
            let name = parts.next().unwrap_or("").trim();

            if !mac.is_empty() && !name.is_empty() && self.staging_bt.len() < 8 {
                self.staging_bt.push(BluetoothDeviceInfo {
                    mac: String::from(mac),
                    name: String::from(name),
                    kind: String::from(kind),
                    battery: batt,
                    connected: conn,
                });
            }
        } else if line == "B_END" {
            self.bt_devices = self.staging_bt.clone();
            self.bt_scanning = false;
            self.status_message = alloc::format!(
                "HCI sorgusu bitti: {} gercek Bluetooth cihazi listelendi.",
                self.bt_devices.len()
            );
        } else if let Some(msg) = line.strip_prefix("MSG|") {
            self.status_message = String::from(msg.trim());
        }
    }
}

static mut GLOBAL_WIRELESS: Option<WirelessManager> = None;
static mut WL_MAGIC: u64 = 0;

pub fn global() -> &'static mut WirelessManager {
    unsafe {
        let slot = core::ptr::addr_of_mut!(GLOBAL_WIRELESS);
        let magic = core::ptr::addr_of_mut!(WL_MAGIC);
        if core::ptr::read_volatile(magic) != 0x574C_5F49_4E49_5431 {
            core::ptr::write(slot, None);
            core::ptr::write(slot, Some(WirelessManager::new()));
            core::ptr::write(magic, 0x574C_5F49_4E49_5431);
        }
        (*slot).as_mut().unwrap()
    }
}
