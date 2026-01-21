use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use plc_core::adapters::hollysys::HollysysCodec;
use plc_core::ast::UniversalPou;
use plc_core::PouCodec;

const DEFAULT_CASE_DIR: &str = "..\\Docs\\样本对比\\测试用例";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let case_dir = args.get(1).map(String::as_str).unwrap_or(DEFAULT_CASE_DIR);
    let case_dir = Path::new(case_dir);

    if !case_dir.exists() {
        anyhow::bail!("case dir not found: {}", case_dir.display());
    }

    let out_dir = case_dir.join("parsed_out");
    fs::create_dir_all(&out_dir)?;

    let mut entries: Vec<PathBuf> = fs::read_dir(case_dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("md"))
                    .unwrap_or(false)
        })
        .collect();
    entries.sort();

    if entries.is_empty() {
        println!("No .md files found in {}", case_dir.display());
        return Ok(());
    }

    for path in entries {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if text.trim().is_empty() {
            println!("[skip] {} is empty", file_name);
            continue;
        }

        let bytes = parse_hex(&text)
            .with_context(|| format!("failed to parse hex in {}", file_name))?;
        let variants = variants_for_name(&file_name);

        for (label, codec) in variants {
            match codec.decode(&bytes) {
                Ok(pou) => {
                    let out_path = out_dir.join(format!("{}_{}.json", file_name, label));
                    write_json(&out_path, &pou)?;
                    print_summary(&file_name, label, &pou, &out_path);
                }
                Err(err) => {
                    println!("[fail] {} {}: {}", file_name, label, err);
                }
            }
        }
    }

    Ok(())
}

fn parse_hex(text: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for (idx, token) in text.split_whitespace().enumerate() {
        if token.len() != 2 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("invalid hex token at {}: '{}'", idx, token);
        }
        let value = u8::from_str_radix(token, 16)
            .with_context(|| format!("invalid hex token at {}: '{}'", idx, token))?;
        bytes.push(value);
    }
    Ok(bytes)
}

fn variants_for_name(file_name: &str) -> Vec<(&'static str, HollysysCodec)> {
    if file_name.contains("安全") {
        vec![("safety", HollysysCodec::safety())]
    } else if file_name.contains("普通") {
        vec![("normal", HollysysCodec::normal())]
    } else {
        vec![
            ("safety", HollysysCodec::safety()),
            ("normal", HollysysCodec::normal()),
        ]
    }
}

fn write_json(path: &Path, pou: &UniversalPou) -> Result<()> {
    let json = serde_json::to_string_pretty(pou)?;
    fs::write(path, json)?;
    Ok(())
}

fn print_summary(file_name: &str, label: &str, pou: &UniversalPou, out_path: &Path) {
    let var_count = count_variables(&pou.variables);
    println!(
        "[ok] {} {} -> {}",
        file_name,
        label,
        out_path.display()
    );
    println!(
        "  POU='{}' networks={} variables={}",
        pou.name,
        pou.networks.len(),
        var_count
    );

    for net in &pou.networks {
        println!(
            "  - network id={} label='{}' comment='{}' elements={} safety_tokens={}",
            net.id,
            net.label,
            net.comment,
            net.elements.len(),
        net.safety_topology.len()
        );
    }
}

fn count_variables(nodes: &[plc_core::ast::VariableNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        match node {
            plc_core::ast::VariableNode::Leaf(_) => count += 1,
            plc_core::ast::VariableNode::Group { children, .. } => {
                count += count_variables(children);
            }
        }
    }
    count
}
