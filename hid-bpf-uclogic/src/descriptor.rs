use std::{collections::HashMap, fmt::Display};

use minijinja;
use serde::Serialize;

use eyre::{Context, OptionExt, Result, bail};

#[derive(Debug)]
pub(crate) struct DeviceInfo {
    pub firmware: String,
    pub magic_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct ParsedDeviceInfo {
    pub x_max: u32,
    pub y_max: u32,
    pub pres_max: u16,
    pub resolution: u16,
    pub num_btns: u8,
}

impl DeviceInfo {
    pub(crate) fn from_str(text: &str) -> Result<Self> {
        fn unquote(s: &str) -> Result<&str> {
            s.strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_eyre("Incorrectly quoted line")
        }

        let info: HashMap<&str, &str> = text
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(key, value)| Ok((key, unquote(value)?)))
            .collect::<Result<_>>()?;

        if info.contains_key("HUION_PAD_MODE") {
            bail!("Unsupported v1 protocol device");
        }

        Ok(Self {
            firmware: info
                .get("HUION_FIRMWARE_ID")
                .copied()
                .ok_or_eyre("No HUION_FIRMWARE_ID found")?
                .to_owned(),
            magic_bytes: hex::decode(
                info.get("HUION_MAGIC_BYTES")
                    .copied()
                    .ok_or_eyre("No HUION_MAGIC_BYTES found")?,
            )?,
        })
    }

    pub(crate) fn parse(&self) -> Result<ParsedDeviceInfo> {
        if self.magic_bytes.len() < 18 {
            bail!("Device info too short");
        }

        if self.magic_bytes[0] as usize != self.magic_bytes.len() {
            bail!("Device info has incorrect length");
        }

        fn le(bytes: &[u8]) -> u32 {
            debug_assert!(bytes.len() <= 4);
            bytes
                .iter()
                .copied()
                .rev()
                .fold(0, |acc, x| (acc << 8) | x as u32)
        }

        let m = &self.magic_bytes;

        Ok(ParsedDeviceInfo {
            x_max: le(&m[2..][..3]),
            y_max: le(&m[5..][..3]),
            pres_max: le(&m[8..][..2]) as _,
            resolution: le(&m[10..][..2]) as _,
            num_btns: m[13] as _,
        })
    }
}

impl ParsedDeviceInfo {
    pub(crate) fn descriptor(&self) -> Result<Vec<u8>> {
        let mut env = minijinja::Environment::new();
        fn bytes(bs: &[u8]) -> String {
            bs.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        }
        env.add_filter("u16", |val: u16| bytes(&val.to_le_bytes()));
        env.add_filter("u32", |val: u32| bytes(&val.to_le_bytes()));
        env.add_filter("u8", |val: u8| format!("{val:02x}"));
        let mut hex_str = env
            .render_str(include_str!("descriptor.j2"), self)
            .wrap_err("Descriptor template error")?;
        hex_str.retain(|x| !x.is_ascii_whitespace());
        Ok(hex::decode(hex_str).wrap_err("Descriptor template output invalid")?)
    }
}

impl Display for ParsedDeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let x_physical = self.x_max as f64 / self.resolution as f64;
        let y_physical = self.y_max as f64 / self.resolution as f64;

        write!(
            f,
            "Device with {} buttons, max pen pressure {}, logical size ({}, {}), resolution {}, physical size in inches ({:.2}, {:.2})",
            self.num_btns,
            self.pres_max,
            self.x_max,
            self.y_max,
            self.resolution,
            x_physical,
            y_physical,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format descriptor hexdump into what hid-decode expects
    ///
    /// ```text
    /// R: {decimal length} {hexdump}
    /// ```
    ///
    /// For example
    ///
    /// ```text
    /// R: 5 aa bb 00 12 34
    /// ```
    fn descriptor_dump(bytes: &[u8]) -> String {
        let byte_hex = |b: u8| -> [u8; 3] {
            const HEX: [u8; 16] = *b"0123456789abcdef";
            [b' ', HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]]
        };
        let hexdump: Vec<u8> = bytes.iter().copied().flat_map(byte_hex).collect();
        let hexdump = String::from_utf8(hexdump).unwrap();

        format!("R: {}{}", bytes.len(), hexdump)
    }

    #[test]
    fn test_desc() {
        let dev = ParsedDeviceInfo {
            x_max: 50800,
            y_max: 31750,
            pres_max: 8191,
            resolution: 5080,
            num_btns: 13,
        };

        let desc = dev.descriptor().unwrap();

        insta::assert_snapshot!(descriptor_dump(&desc));
    }

    #[test]
    fn test_less_buttons_desc() {
        let dev = ParsedDeviceInfo {
            x_max: 50800,
            y_max: 31750,
            pres_max: 8191,
            resolution: 5080,
            num_btns: 5,
        };

        let desc = dev.descriptor().unwrap();
        insta::assert_snapshot!(descriptor_dump(&desc));
    }
}
