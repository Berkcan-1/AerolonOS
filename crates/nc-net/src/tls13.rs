extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use nc_arch::rand::HardwareRng;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tls13State {
    Start,
    WaitServerHello,
    WaitFlight,
    Ready,
    Failed,
}

struct Keys {
    key: [u8; 32],
    iv: [u8; 12],
    seq: u64,
}

fn extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let (prk, _) = Hkdf::<Sha256>::extract(Some(salt), ikm);
    let mut out = [0u8; 32];
    out.copy_from_slice(prk.as_slice());
    out
}

fn expand_label(secret: &[u8; 32], label: &str, context: &[u8], length: usize) -> Vec<u8> {
    let mut info = Vec::new();
    info.extend_from_slice(&(length as u16).to_be_bytes());
    info.push((6 + label.len()) as u8);
    info.extend_from_slice(b"tls13 ");
    info.extend_from_slice(label.as_bytes());
    info.push(context.len() as u8);
    info.extend_from_slice(context);
    let mut out = alloc::vec![0u8; length];
    if let Ok(hk) = Hkdf::<Sha256>::from_prk(secret) {
        let _ = hk.expand(&info, &mut out);
    }
    out
}

fn expand32(secret: &[u8; 32], label: &str, context: &[u8]) -> [u8; 32] {
    let v = expand_label(secret, label, context, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    out
}

fn be16(bytes: &[u8]) -> usize {
    ((bytes[0] as usize) << 8) | bytes[1] as usize
}

impl Keys {
    fn from_secret(secret: &[u8; 32]) -> Keys {
        let mut key = [0u8; 32];
        key.copy_from_slice(&expand_label(secret, "key", &[], 32));
        let mut iv = [0u8; 12];
        iv.copy_from_slice(&expand_label(secret, "iv", &[], 12));
        Keys { key, iv, seq: 0 }
    }

    fn nonce(&self) -> [u8; 12] {
        let mut nonce = self.iv;
        let seq = self.seq.to_be_bytes();
        for i in 0..8 {
            nonce[4 + i] ^= seq[i];
        }
        nonce
    }

    fn seal(&mut self, inner_type: u8, data: &[u8]) -> Vec<u8> {
        let mut inner = Vec::with_capacity(data.len() + 1);
        inner.extend_from_slice(data);
        inner.push(inner_type);
        let length = (inner.len() + 16) as u16;
        let aad = [23u8, 3, 3, (length >> 8) as u8, length as u8];
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
        let nonce = self.nonce();
        let sealed = cipher
            .encrypt(Nonce::from_slice(&nonce), Payload { msg: &inner, aad: &aad })
            .unwrap_or_default();
        self.seq += 1;
        let mut out = Vec::with_capacity(5 + sealed.len());
        out.extend_from_slice(&aad);
        out.extend_from_slice(&sealed);
        out
    }

    fn open(&mut self, header: &[u8], body: &[u8]) -> Option<(u8, Vec<u8>)> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
        let nonce = self.nonce();
        let mut plain = cipher.decrypt(Nonce::from_slice(&nonce), Payload { msg: body, aad: header }).ok()?;
        self.seq += 1;
        while let Some(&0) = plain.last() {
            plain.pop();
        }
        let content_type = plain.pop()?;
        Some((content_type, plain))
    }
}

pub struct Tls13 {
    pub state: Tls13State,
    pub app_in: Vec<u8>,
    pub closed: bool,
    host: String,
    secret: StaticSecret,
    public: [u8; 32],
    random: [u8; 32],
    transcript: Sha256,
    hs_secret: [u8; 32],
    c_hs_secret: [u8; 32],
    write: Option<Keys>,
    read: Option<Keys>,
    rx: Vec<u8>,
}

impl Tls13 {
    pub fn new(host: &str) -> Tls13 {
        let mut random = [0u8; 32];
        let mut private = [0u8; 32];
        HardwareRng::fill_bytes(&mut random);
        HardwareRng::fill_bytes(&mut private);
        let secret = StaticSecret::from(private);
        let public = PublicKey::from(&secret).to_bytes();
        Tls13 {
            state: Tls13State::Start,
            app_in: Vec::new(),
            closed: false,
            host: String::from(host),
            secret,
            public,
            random,
            transcript: Sha256::new(),
            hs_secret: [0; 32],
            c_hs_secret: [0; 32],
            write: None,
            read: None,
            rx: Vec::new(),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == Tls13State::Ready
    }

    pub fn is_failed(&self) -> bool {
        self.state == Tls13State::Failed
    }

    fn transcript_hash(&self) -> [u8; 32] {
        let digest = self.transcript.clone().finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_slice());
        out
    }

    fn push_ext(buf: &mut Vec<u8>, kind: u16, data: &[u8]) {
        buf.extend_from_slice(&kind.to_be_bytes());
        buf.extend_from_slice(&(data.len() as u16).to_be_bytes());
        buf.extend_from_slice(data);
    }

    pub fn client_hello(&mut self) -> Vec<u8> {
        let mut ext = Vec::new();

        let name = self.host.as_bytes();
        let mut sni = Vec::new();
        sni.extend_from_slice(&((name.len() + 3) as u16).to_be_bytes());
        sni.push(0);
        sni.extend_from_slice(&(name.len() as u16).to_be_bytes());
        sni.extend_from_slice(name);
        Self::push_ext(&mut ext, 0x0000, &sni);

        Self::push_ext(&mut ext, 0x000a, &[0x00, 0x02, 0x00, 0x1d]);
        Self::push_ext(&mut ext, 0x000d, &[0x00, 0x08, 0x04, 0x03, 0x08, 0x04, 0x04, 0x01, 0x08, 0x07]);
        Self::push_ext(&mut ext, 0x002b, &[0x02, 0x03, 0x04]);

        let mut share = Vec::new();
        share.extend_from_slice(&36u16.to_be_bytes());
        share.extend_from_slice(&0x001du16.to_be_bytes());
        share.extend_from_slice(&32u16.to_be_bytes());
        share.extend_from_slice(&self.public);
        Self::push_ext(&mut ext, 0x0033, &share);

        let mut body = Vec::new();
        body.extend_from_slice(&[0x03, 0x03]);
        body.extend_from_slice(&self.random);
        body.push(0);
        body.extend_from_slice(&[0x00, 0x02, 0x13, 0x03]);
        body.extend_from_slice(&[0x01, 0x00]);
        body.extend_from_slice(&(ext.len() as u16).to_be_bytes());
        body.extend_from_slice(&ext);

        let mut message = Vec::new();
        message.push(1);
        message.push((body.len() >> 16) as u8);
        message.push((body.len() >> 8) as u8);
        message.push(body.len() as u8);
        message.extend_from_slice(&body);
        self.transcript.update(&message);

        let mut record = Vec::new();
        record.push(22);
        record.extend_from_slice(&[0x03, 0x01]);
        record.extend_from_slice(&(message.len() as u16).to_be_bytes());
        record.extend_from_slice(&message);
        self.state = Tls13State::WaitServerHello;
        record
    }

    fn on_server_hello(&mut self, message: &[u8]) -> bool {
        if message.len() < 48 || message[0] != 2 {
            return false;
        }
        let session_len = message[38] as usize;
        let mut offset = 39 + session_len;
        if message.len() < offset + 5 {
            return false;
        }
        let suite = be16(&message[offset..]);
        offset += 3;
        if suite != 0x1303 {
            return false;
        }
        let ext_len = be16(&message[offset..]);
        offset += 2;
        let end = (offset + ext_len).min(message.len());
        let mut peer: Option<[u8; 32]> = None;
        while offset + 4 <= end {
            let kind = be16(&message[offset..]);
            let len = be16(&message[offset + 2..]);
            offset += 4;
            if kind == 0x0033 && len >= 36 && offset + 36 <= end && be16(&message[offset..]) == 0x001d {
                let mut key = [0u8; 32];
                key.copy_from_slice(&message[offset + 4..offset + 36]);
                peer = Some(key);
            }
            offset += len;
        }
        let peer = match peer {
            Some(key) => key,
            None => return false,
        };
        self.transcript.update(message);

        let shared = self.secret.diffie_hellman(&PublicKey::from(peer));
        let zero = [0u8; 32];
        let early = extract(&zero, &zero);
        let empty = Sha256::digest(b"");
        let derived = expand32(&early, "derived", empty.as_slice());
        self.hs_secret = extract(&derived, shared.as_bytes());

        let hash = self.transcript_hash();
        let client = expand32(&self.hs_secret, "c hs traffic", &hash);
        let server = expand32(&self.hs_secret, "s hs traffic", &hash);
        self.c_hs_secret = client;
        self.write = Some(Keys::from_secret(&client));
        self.read = Some(Keys::from_secret(&server));
        self.state = Tls13State::WaitFlight;
        true
    }

    fn on_server_finished(&mut self, message: &[u8], out: &mut Vec<u8>) {
        self.transcript.update(message);
        let hash = self.transcript_hash();
        let zero = [0u8; 32];
        let empty = Sha256::digest(b"");
        let derived = expand32(&self.hs_secret, "derived", empty.as_slice());
        let master = extract(&derived, &zero);
        let client_app = expand32(&master, "c ap traffic", &hash);
        let server_app = expand32(&master, "s ap traffic", &hash);

        let finished_key = expand32(&self.c_hs_secret, "finished", &[]);
        let (verify, _) = Hkdf::<Sha256>::extract(Some(&finished_key[..]), &hash[..]);
        let mut finished = Vec::new();
        finished.extend_from_slice(&[20, 0, 0, 32]);
        finished.extend_from_slice(verify.as_slice());
        if let Some(keys) = self.write.as_mut() {
            out.extend_from_slice(&keys.seal(22, &finished));
        }
        self.write = Some(Keys::from_secret(&client_app));
        self.read = Some(Keys::from_secret(&server_app));
        self.state = Tls13State::Ready;
    }

    fn on_handshake_messages(&mut self, data: &[u8], out: &mut Vec<u8>) {
        let mut offset = 0;
        while offset + 4 <= data.len() {
            let length = ((data[offset + 1] as usize) << 16) | ((data[offset + 2] as usize) << 8) | data[offset + 3] as usize;
            let end = offset + 4 + length;
            if end > data.len() {
                break;
            }
            let message = &data[offset..end];
            if self.state == Tls13State::WaitFlight {
                if data[offset] == 20 {
                    self.on_server_finished(message, out);
                } else {
                    self.transcript.update(message);
                }
            }
            offset = end;
        }
    }

    fn on_record(&mut self, record: &[u8], out: &mut Vec<u8>) {
        let kind = record[0];
        let body = &record[5..];
        match kind {
            20 => {}
            21 => {
                self.state = Tls13State::Failed;
            }
            22 => {
                if self.state == Tls13State::WaitServerHello {
                    if !self.on_server_hello(body) {
                        self.state = Tls13State::Failed;
                    }
                }
            }
            23 => {
                let opened = match self.read.as_mut() {
                    Some(keys) => keys.open(&record[..5], body),
                    None => None,
                };
                let (inner_type, plain) = match opened {
                    Some(value) => value,
                    None => {
                        self.state = Tls13State::Failed;
                        return;
                    }
                };
                match inner_type {
                    22 => self.on_handshake_messages(&plain, out),
                    23 => self.app_in.extend_from_slice(&plain),
                    21 => self.closed = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn feed(&mut self, data: &[u8], out: &mut Vec<u8>) {
        self.rx.extend_from_slice(data);
        loop {
            if self.rx.len() < 5 || self.state == Tls13State::Failed {
                break;
            }
            let length = be16(&self.rx[3..5]);
            if self.rx.len() < 5 + length {
                break;
            }
            let record: Vec<u8> = self.rx.drain(..5 + length).collect();
            self.on_record(&record, out);
        }
    }

    pub fn encrypt_app(&mut self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(keys) = self.write.as_mut() {
            for chunk in data.chunks(16000) {
                out.extend_from_slice(&keys.seal(23, chunk));
            }
        }
        out
    }
}
