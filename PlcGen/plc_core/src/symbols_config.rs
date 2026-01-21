use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct FbMembers {
    pub name: String,
    pub members: HashSet<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct SymbolConfig {
    pub function_blocks: Vec<FbMembers>,
}

impl SymbolConfig {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read symbol config file from: {}", path.display()))?;
        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse symbol config JSON from: {}", path.display()))?;
        Ok(config)
    }

    pub fn to_lookup_map(&self) -> HashMap<String, HashSet<String>> {
        self.function_blocks
            .iter()
            .map(|fb| (fb.name.clone(), fb.members.clone()))
            .collect()
    }
}
