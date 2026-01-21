use anyhow::{Result, bail};

use crate::ast::UniversalPou;
use crate::ports::PouCodec;

/// Application layer use case wrapper around `PouCodec`.
/// Keeps orchestration (validation, boundary checks) away from adapters.
#[derive(Debug, Clone)]
pub struct PouService<C: PouCodec> {
    codec: C,
}

impl<C: PouCodec> PouService<C> {
    /// Create a new service with the given codec.
    pub fn new(codec: C) -> Self {
        Self { codec }
    }

    /// Decode clipboard bytes into a POU and run lightweight validation.
    pub fn decode(&self, data: &[u8]) -> Result<UniversalPou> {
        let pou = self.codec.decode(data)?;
        validate_pou(&pou)?;
        Ok(pou)
    }

    /// Encode a POU into clipboard bytes after validation.
    pub fn encode(&self, pou: &UniversalPou) -> Result<Vec<u8>> {
        validate_pou(pou)?;
        self.codec.encode(pou)
    }

    /// Clipboard format name for the codec.
    pub fn format_name(&self) -> &'static str {
        self.codec.format_name()
    }
}

fn validate_pou(pou: &UniversalPou) -> Result<()> {
    if pou.name.trim().is_empty() {
        bail!("POU name is empty");
    }
    Ok(())
}
