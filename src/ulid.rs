use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
static COUNTER: AtomicU64 = AtomicU64::new(0);

const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Get current timestamp in milliseconds since UNIX epoch
#[cfg(not(target_arch = "wasm32"))]
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_millis() as u64
}

/// Get current timestamp in milliseconds since UNIX epoch (WASM version)
#[cfg(target_arch = "wasm32")]
fn current_timestamp_ms() -> u64 {
    js_sys::Date::now() as u64
}

/// A ULID implementation that works in both standard and WASM environments
/// without requiring getrandom or other problematic dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ulid {
    timestamp: u64,
    randomness_high: u16,  // 16 bits
    randomness_low: u64,   // 64 bits
}

impl Ulid {
    pub fn new() -> Self {
        let timestamp = current_timestamp_ms();

        let last_ts = LAST_TIMESTAMP.load(Ordering::Acquire);
        
        let randomness = if timestamp == last_ts {
            COUNTER.fetch_add(1, Ordering::AcqRel) + 1
        } else {
            LAST_TIMESTAMP.store(timestamp, Ordering::Release);
            let new_counter = timestamp ^ (timestamp << 32);
            COUNTER.store(new_counter, Ordering::Release);
            new_counter
        };

        Self {
            timestamp: timestamp & 0xFFFF_FFFF_FFFF,
            randomness_high: (randomness >> 48) as u16,
            randomness_low: randomness & 0xFFFF_FFFF_FFFF_FFFF,
        }
    }
    
    /// Convert to standard ULID u128 binary format
    pub fn to_u128(&self) -> u128 {
        // ULID format: 48-bit timestamp (MSB) | 80-bit randomness (LSB)
        let timestamp_part = (self.timestamp as u128) << 80;
        let randomness_high_part = (self.randomness_high as u128) << 64;
        let randomness_low_part = self.randomness_low as u128;
        timestamp_part | randomness_high_part | randomness_low_part
    }
    
    /// Create from standard ULID u128 binary format
    pub fn from_u128(value: u128) -> Self {
        Self {
            timestamp: ((value >> 80) & 0xFFFF_FFFF_FFFF) as u64,
            randomness_high: ((value >> 64) & 0xFFFF) as u16,
            randomness_low: (value & 0xFFFF_FFFF_FFFF_FFFF) as u64,
        }
    }
    
    /// Create from a u128 value (for compatibility with standard ulid crate)
    pub fn from(value: u128) -> Self {
        Self::from_u128(value)
    }

    fn encode_base32(mut value: u64, dst: &mut [u8], len: usize) {
        for i in (0..len).rev() {
            dst[i] = CROCKFORD[(value & 0x1F) as usize];
            value >>= 5;
        }
    }

    pub fn to_string(&self) -> String {
        let mut buf = [0u8; 26];
        
        // Encode 48-bit timestamp (10 characters)
        Self::encode_base32(self.timestamp, &mut buf[0..10], 10);
        
        // Encode 80-bit randomness (16 characters)
        // Encode high 16 bits (about 3.2 characters, but we'll use 4)
        Self::encode_base32(self.randomness_high as u64, &mut buf[10..14], 4);
        // Encode low 64 bits (about 12.8 characters, use 12)
        Self::encode_base32(self.randomness_low, &mut buf[14..26], 12);
        
        String::from_utf8(buf.to_vec()).unwrap()
    }
    
    /// Access the internal u128 representation (for compatibility)
    pub fn as_u128(&self) -> u128 {
        self.to_u128()
    }
    
    /// Access as bytes (for compatibility)
    pub fn as_bytes(&self) -> [u8; 16] {
        self.to_u128().to_be_bytes()
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for Ulid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Ulid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        
        if s.len() != 26 {
            return Err(serde::de::Error::custom("ULID must be 26 characters"));
        }
        
        let decode_char = |c: u8| -> Result<u64, D::Error> {
            match c {
                b'0'..=b'9' => Ok((c - b'0') as u64),
                b'A'..=b'H' => Ok((c - b'A' + 10) as u64),
                b'J'..=b'K' => Ok((c - b'J' + 18) as u64),
                b'M'..=b'N' => Ok((c - b'M' + 20) as u64),
                b'P'..=b'T' => Ok((c - b'P' + 22) as u64),
                b'V'..=b'Z' => Ok((c - b'V' + 27) as u64),
                _ => Err(serde::de::Error::custom("Invalid ULID character")),
            }
        };
        
        let bytes = s.as_bytes();
        let mut timestamp = 0u64;
        for i in 0..10 {
            timestamp = (timestamp << 5) | decode_char(bytes[i])?;
        }
        
        let mut randomness = 0u64;
        for i in 10..26 {
            randomness = (randomness << 5) | decode_char(bytes[i])?;
        }
        
        // Extract randomness parts from the decoded value
        let randomness_high = ((randomness >> 60) & 0xFFFF) as u16;
        let randomness_low = randomness & 0xFFFF_FFFF_FFFF_FFFF;
        
        Ok(Self {
            timestamp,
            randomness_high,
            randomness_low,
        })
    }
}

// Provide conversion to/from the u128 that ulid crate uses internally
impl From<u128> for Ulid {
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl From<Ulid> for u128 {
    fn from(ulid: Ulid) -> Self {
        ulid.to_u128()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulid_creation() {
        let ulid1 = Ulid::new();
        // Small delay to ensure we get different counter values
        std::thread::yield_now();
        let ulid2 = Ulid::new();
        assert_ne!(ulid1, ulid2);
    }

    #[test]
    fn test_ulid_string_format() {
        let ulid = Ulid::new();
        let s = ulid.to_string();
        assert_eq!(s.len(), 26);
        assert!(s.chars().all(|c| CROCKFORD.contains(&(c as u8))));
    }

    #[test]
    fn test_ulid_ordering() {
        let ulid1 = Ulid::new();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let ulid2 = Ulid::new();
        assert!(ulid1 < ulid2);
    }

    #[test]
    fn test_ulid_binary_roundtrip() {
        let ulid = Ulid::new();
        let binary = ulid.to_u128();
        let restored = Ulid::from_u128(binary);
        assert_eq!(ulid, restored);
    }
    
    #[test]
    fn test_u128_conversion() {
        let ulid = Ulid::new();
        let as_u128: u128 = ulid.into();
        let back: Ulid = as_u128.into();
        assert_eq!(ulid, back);
    }
}