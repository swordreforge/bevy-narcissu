//! bevy-vn-asset-packer — Standalone PAK packer.
//!
//! Packs game assets into BPAK v1 bundles with per-file zstd compression.
//! Config-driven via pack_config.ron: each bundle has a name and include patterns.
//!
//! Usage: asset-packer --input assets --output assets_pak --config pack_config.ron

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const PAK_MAGIC: &[u8; 4] = b"BPAK";
const PAK_VERSION: u32 = 1;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "assets")]
    input: PathBuf,
    #[arg(long, default_value = "assets_pak")]
    output: PathBuf,
    #[arg(long, default_value = "pack_config.ron")]
    config: PathBuf,
    #[arg(long, default_value_t = 3)]
    compression_level: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackConfig {
    bundles: Vec<BundleDef>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BundleDef {
    name: String,
    includes: Vec<String>,
}

struct FileEntry {
    path: String,
    offset: u64,
    compressed_size: u64,
    uncompressed_size: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config: PackConfig = ron::from_str(&fs::read_to_string(&args.config)?)?;
    fs::create_dir_all(&args.output)?;

    for bundle in &config.bundles {
        let mut files: Vec<(String, PathBuf)> = Vec::new();

        // Walk the input directory, assign files to first matching bundle
        for entry in walkdir::WalkDir::new(&args.input)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let Ok(rel_path) = entry.path().strip_prefix(&args.input) else { continue; };
            let Some(rel) = rel_path.to_str() else { continue; };
            let rel = rel.to_string();
            if bundle.includes.iter().any(|pat| rel.starts_with(pat)) {
                files.push((rel, entry.path().to_path_buf()));
            }
        }

        if files.is_empty() {
            println!("Bundle '{}': no files matched", bundle.name);
            continue;
        }

        let output_path = args.output.join(format!("{}.pak", bundle.name));
        let mut writer = BufWriter::new(fs::File::create(&output_path)?);
        let mut index: Vec<FileEntry> = Vec::new();
        let mut data_offset: u64 = 0;

        for (rel_path, disk_path) in &files {
            let raw = fs::read(disk_path)?;
            let uncompressed_size = raw.len() as u64;

            let compressed = zstd::encode_all(raw.as_slice(), args.compression_level)?;
            let compressed_size = compressed.len() as u64;

            writer.write_all(&compressed)?;

            index.push(FileEntry {
                path: rel_path.clone(),
                offset: data_offset,
                compressed_size,
                uncompressed_size,
            });
            data_offset += compressed_size;
        }

        // Write index
        let index_offset = data_offset;
        for entry in &index {
            let path_bytes = entry.path.as_bytes();
            writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(path_bytes)?;
            writer.write_all(&entry.offset.to_le_bytes())?;
            writer.write_all(&entry.compressed_size.to_le_bytes())?;
            writer.write_all(&entry.uncompressed_size.to_le_bytes())?;
        }

        // Write footer: magic (4) + version (4) + index_offset (8) + entry_count (4) = 20 bytes
        writer.write_all(PAK_MAGIC)?;
        writer.write_all(&PAK_VERSION.to_le_bytes())?;
        writer.write_all(&index_offset.to_le_bytes())?;
        writer.write_all(&(index.len() as u32).to_le_bytes())?;
        writer.flush()?;

        println!(
            "Bundle '{}': {} files, {:.1} MB uncompressed → {:.1} MB compressed",
            bundle.name,
            files.len(),
            index.iter().map(|e| e.uncompressed_size).sum::<u64>() as f64 / 1_048_576.0,
            index.iter().map(|e| e.compressed_size).sum::<u64>() as f64 / 1_048_576.0,
        );
    }

    println!("Done — output in {}", args.output.display());
    Ok(())
}
