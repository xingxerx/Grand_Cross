use serde::{Deserialize, Serialize};

/// The three supported platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Platform {
    Roblox    = 0x01,
    Minecraft = 0x02,
    Hytale    = 0x03,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Roblox    => write!(f, "Roblox"),
            Platform::Minecraft => write!(f, "Minecraft"),
            Platform::Hytale    => write!(f, "Hytale"),
        }
    }
}
