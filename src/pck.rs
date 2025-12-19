//! Snail Transfer Protocol – Packet module
//!
//! # Format:
//!
//! ```text
//!  ┌───────────────────────────────────────────────┐
//!  │                     Packet                    │
//!  ├───────────┬───────────────┬───────────────────┤
//!  │ N | A | C | K | F | I | N | S | Y | N |       │
//!  │                unused (fixed zeroes)          │
//!  │                     Checksum                  │
//!  │            (rest of header + data)            │
//!  ├───────────────────────────────────────────────┤
//!  │                 Payload Size                  │
//!  ├───────────────────────────────────────────────┤
//!  │ Application Data (variable length, ≤ 512 B)   │
//!  └───────────────────────────────────────────────┘
//! ```
//!
//! ## Fields
//!
//! - **Flags (8 bit)**  
//!   - `N` – Alternating bit (0 or 1)  
//!   - `ACK` – Acknowledgment flag  
//!   - `FIN` – Finish flag  
//!   - `SYN` – Synchronize flag  
//! - **unused** – reserved bits, always `0`  
//! - **Checksum (8 bit)** – CRC-8/I-432-1 checksum over header + data  
//! - **Payload Size (16 bit)** – size of the following data in bytes  
//! - **Application Data** – variable-length payload (max. 512 bytes)
//!
//! The checksum is computed over the encoded header (without checksum) and the payload.  

use std::io;

pub const MAX_PAYLOAD_SIZE: usize = 512;
pub const HEADER_LEN: usize = 4;

/// CRC-8/I-432-1: https://reveng.sourceforge.io/crc-catalogue/1-15.htm
const CRC_8_I_423_1: crc::Algorithm<u8> = crc::Algorithm {
    width: 8,
    poly: 0x07,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x55,
    check: 0xA1,
    residue: 0xAC,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum Flag {
    SYN,
    ACK,
    FIN,
    FINACK,
    Data,
}

impl Flag {
    fn to_byte(&self, n: bool) -> u8 {
        let mut f = match self {
            Flag::SYN => 0b00010000,
            Flag::ACK => 0b01000000,
            Flag::FIN => 0b00100000,
            Flag::FINACK => 0b01100000,
            Flag::Data => 0b00000000,
        };

        f |= match n {
            // 128
            true => 0b10000000,
            // 0
            false => 0b00000000,
        };
        f
    }

    fn byte_to_flag_and_n(b: u8) -> io::Result<(Flag, bool)> {
        // check for a fixed zero violation
        let fixed_zeros = b & 0b00001111;
        if fixed_zeros > 0 && fixed_zeros <= 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "rcvpkt violates fixed zero convention",
            ));
        }

        // extract n
        let n = (b & 0b10000000) != 0;

        // extract flag bits - ignore n
        let flag_bits = b & 0b01110000;
        let flag = match flag_bits {
            0b00010000 => Flag::SYN,
            0b01000000 => Flag::ACK,
            0b00100000 => Flag::FIN,
            0b01100000 => Flag::FINACK,
            0b00000000 => Flag::Data,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unknown flag combination",
                ));
            }
        };

        Ok((flag, n))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    n: bool,
    flag: Flag,
    checksum: u8,
    payload_len: u16,
    /// MAX_PACKSIZE
    buf: Vec<u8>,
}

impl Packet {
    pub fn max_pck_payload_size() -> usize {
        MAX_PAYLOAD_SIZE - HEADER_LEN
    }

    /// n needs to be bool because it can only be 0 or 1
    /// Condition of Alternating bit protocol
    pub fn new(n: bool, f: Flag, p: Vec<u8>) -> io::Result<Self> {
        // check for valid payload size
        if p.len() > Packet::max_pck_payload_size() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Payload size {} exceeds MAX_PACKET_SIZE {}",
                    p.len(),
                    MAX_PAYLOAD_SIZE
                ),
            ));
        }

        // encoded buf
        let mut buf: Vec<u8> = vec![0; HEADER_LEN + p.len()];
        buf[0] = f.to_byte(n);
        let p_l = p.len() as u16;
        buf[2..HEADER_LEN].copy_from_slice(&p_l.to_be_bytes());
        buf[HEADER_LEN..HEADER_LEN + p.len()].copy_from_slice(&p);

        // calc checksum
        buf[1] = Packet::calc_checksum_crc_8_i_423_1(buf[0], p_l, &p);

        Ok(Self {
            flag: f,
            payload_len: p_l,
            checksum: buf[1],
            buf,
            n,
        })
    }

    // getter

    pub fn n(&self) -> u8 {
        match self.n {
            true => 1,
            false => 0,
        }
    }

    pub fn payload(&self) -> &[u8] {
        &self.buf[HEADER_LEN..HEADER_LEN + self.payload_len as usize]
    }

    // syntax sugar: functions named as in fsm diagram

    #[allow(non_snake_case)]
    pub fn is_SYN(&self) -> bool {
        self.flag == Flag::SYN
    }

    #[allow(non_snake_case)]
    pub fn is_not_SYN(&self) -> bool {
        !self.is_SYN()
    }

    #[allow(non_snake_case)]
    pub fn is_ACK(&self) -> bool {
        self.flag == Flag::ACK
    }

    #[allow(non_snake_case)]
    pub fn is_FIN(&self) -> bool {
        self.flag == Flag::FIN
    }

    #[allow(non_snake_case)]
    pub fn is_Data(&self) -> bool {
        self.flag == Flag::Data
    }

    #[allow(non_snake_case)]
    pub fn is_FINACK(&self) -> bool {
        self.flag == Flag::FINACK
    }

    pub fn notcorrupt(&self) -> bool {
        self.checksum == self.calc_checksum()
    }

    pub fn corrupt(&self) -> bool {
        !self.notcorrupt()
    }

    // checksum

    pub fn calc_checksum(&self) -> u8 {
        Packet::calc_checksum_crc_8_i_423_1(
            self.flag.to_byte(self.n),
            self.payload_len,
            self.payload(),
        )
    }

    fn calc_checksum_crc_8_i_423_1(f_and_n: u8, p_l: u16, p: &[u8]) -> u8 {
        let crc = crc::Crc::<u8>::new(&CRC_8_I_423_1);
        let mut digst = crc.digest();

        digst.update(&[f_and_n]);
        digst.update(&p_l.to_be_bytes());
        digst.update(p);

        digst.finalize()
    }

    // encoding && decoding
    pub fn encode(&self) -> &[u8] {
        &self.buf
    }

    pub fn decode(mut buf: Vec<u8>) -> io::Result<Self> {
        if buf.len() < HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Buffer too short",
            ));
        }

        let (f, n) = Flag::byte_to_flag_and_n(buf[0])?;
        let checksum = buf[1];
        let payload_len = u16::from_be_bytes([buf[2], buf[3]]);

        if buf.len() < HEADER_LEN + payload_len as usize {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Payload missing",
            ));
        }

        buf.shrink_to(HEADER_LEN + payload_len as usize);

        Ok(Self {
            flag: f,
            payload_len,
            checksum,
            buf,
            n,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_checksum() {
        let pck1 = Packet::new(false, Flag::SYN, vec![b'a']).unwrap();
        let pck2 = Packet::new(true, Flag::SYN, vec![b'a']).unwrap();
        let pck3 = Packet::new(false, Flag::SYN, vec![b'a']).unwrap();
        let pck4 = Packet::new(false, Flag::SYN, vec![b'a', b'b']).unwrap();

        assert_eq!(pck1.calc_checksum(), pck3.calc_checksum());
        assert_ne!(pck1.calc_checksum(), pck2.calc_checksum());
        assert_ne!(pck1.calc_checksum(), pck4.calc_checksum());
    }

    #[test]
    fn test_encode() {
        let pck1 = Packet::new(false, Flag::SYN, vec![b'a']).unwrap();
        let pck2 = Packet::new(true, Flag::ACK, vec![b'a', b'b']).unwrap();
        let pck3 = Packet::new(true, Flag::ACK, vec![b'a', b'b']).unwrap();
        let pck4 = Packet::new(true, Flag::ACK, vec![b'a', b'b']).unwrap();

        assert_ne!(pck1.encode(), pck2.encode());
        assert_eq!(pck3.encode(), pck4.encode());
    }

    #[test]
    fn test_decode() {
        let pck1 = Packet::new(false, Flag::SYN, vec![b'a']).unwrap();
        let pck2 = Packet::new(true, Flag::ACK, vec![b'a', b'b']).unwrap();

        assert_eq!(Packet::decode(pck1.encode().to_vec()).unwrap(), pck1);

        assert_eq!(Packet::decode(pck2.encode().to_vec()).unwrap(), pck2,);
    }

    #[test]
    fn test_encode_decode_checksum() {
        let pck1 = Packet::new(false, Flag::SYN, vec![b'a']).unwrap();
        let pck2 = Packet::new(true, Flag::ACK, vec![b'a', b'b']).unwrap();

        let pck1_decoded = Packet::decode(pck1.encode().to_vec()).unwrap();
        let pck2_decoded = Packet::decode(pck2.encode().to_vec()).unwrap();

        assert_eq!(pck1_decoded.calc_checksum(), pck1.calc_checksum());

        assert_eq!(pck2_decoded.calc_checksum(), pck2.calc_checksum());
    }
}
