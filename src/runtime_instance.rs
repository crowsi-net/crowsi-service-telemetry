use crate::{TelemetryError, error::Result};

const HEX: &[u8; 16] = b"0123456789abcdef";

pub fn generate() -> Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        TelemetryError::Io(std::io::Error::other(format!(
            "random source failed: {error}"
        )))
    })?;
    let mut encoded = String::with_capacity(32);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(encoded)
}
