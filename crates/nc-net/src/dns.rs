// crates/nc-net/src/dns.rs
extern crate alloc;
use alloc::vec::Vec;

pub fn encode_dns_query(domain: &str, tx_id: u16) -> Vec<u8> {
    let mut buf = Vec::new();
    // 12 Bayt DNS Başlığı (RFC 1035)
    buf.extend_from_slice(&tx_id.to_be_bytes());
    buf.extend_from_slice(&0x0100u16.to_be_bytes()); // RD=1 (Recursion Desired)
    buf.extend_from_slice(&1u16.to_be_bytes());      // QDCOUNT: 1
    buf.extend_from_slice(&0u16.to_be_bytes());      // ANCOUNT: 0
    buf.extend_from_slice(&0u16.to_be_bytes());      // NSCOUNT: 0
    buf.extend_from_slice(&0u16.to_be_bytes());      // ARCOUNT: 0

    // Question Name (QNAME) - ornek: "example.com" -> [7]"example"[3]"com"[0]
    for part in domain.trim_matches('.').split('.') {
        let len = part.len() as u8;
        if len > 0 {
            buf.push(len);
            buf.extend_from_slice(part.as_bytes());
        }
    }
    buf.push(0); // Null sonlandırıcı

    buf.extend_from_slice(&1u16.to_be_bytes()); // QTYPE: A (IPv4)
    buf.extend_from_slice(&1u16.to_be_bytes()); // QCLASS: IN (Internet)
    buf
}

pub fn parse_dns_response(data: &[u8]) -> Option<[u8; 4]> {
    if data.len() < 12 {
        return None;
    }
    let an_count = u16::from_be_bytes([data[6], data[7]]);
    if an_count == 0 {
        return None;
    }

    // Başlığı geç (12 bayt)
    let mut idx = 12;

    // Soru bölümünü (Question Section) atla
    while idx < data.len() {
        let len = data[idx] as usize;
        if len == 0 {
            idx += 1;
            break;
        }
        if (len & 0xC0) == 0xC0 {
            idx += 2; // Sıkıştırılmış işaretçi
            break;
        }
        idx += 1 + len;
    }
    idx += 4; // QTYPE ve QCLASS (4 bayt)

    // Yanıt (Answer) kayıtlarını incele
    for _ in 0..an_count {
        if idx >= data.len() {
            break;
        }
        if (data[idx] & 0xC0) == 0xC0 {
            idx += 2;
        } else {
            while idx < data.len() && data[idx] != 0 {
                idx += 1 + data[idx] as usize;
            }
            idx += 1;
        }

        if idx + 10 > data.len() {
            break;
        }

        let r_type = u16::from_be_bytes([data[idx], data[idx + 1]]);
        let r_len = u16::from_be_bytes([data[idx + 8], data[idx + 9]]) as usize;
        idx += 10;

        // Type 1 = A Kaydı (IPv4)
        if r_type == 1 && r_len == 4 && idx + 4 <= data.len() {
            return Some([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]);
        }
        idx += r_len;
    }
    None
}
