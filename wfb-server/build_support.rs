use anyhow::bail;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub struct Tooling {
    pub tailwindcss_bin: Option<String>,
    pub esbuild_bin: Option<String>,
}

impl Tooling {
    pub fn from_env() -> Self {
        Self {
            tailwindcss_bin: std::env::var("TAILWINDCSS_BIN").ok(),
            esbuild_bin: std::env::var("ESBUILD_BIN").ok(),
        }
    }
}

pub fn rerun_if_changed_recursive(path: &Path) {
    if path.is_file() {
        println!("cargo:rerun-if-changed={}", path.display());
        return;
    }

    if !path.is_dir() {
        return;
    }

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
}

pub fn ensure_clean_dir(dir: &Path) -> Result<()> {
    if dir.exists() {
        let _ = fs::remove_dir_all(dir);
    }
    fs::create_dir_all(dir).with_context(|| format!("create dir {}", dir.display()))?;
    Ok(())
}

pub fn run_tailwind(tooling: &Tooling, config: &Path, input: &Path, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create tailwind output dir {}", parent.display()))?;
    }

    let mut cmd = if let Some(bin) = tooling.tailwindcss_bin.as_deref() {
        Command::new(bin)
    } else if Command::new("tailwindcss").arg("--help").output().is_ok() {
        Command::new("tailwindcss")
    } else {
        let mut c = Command::new("npx");
        c.arg("--yes").arg("tailwindcss@3.4.17");
        c
    };

    let status = cmd
        .arg("build")
        .arg("-c")
        .arg(config)
        .arg("-i")
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--minify")
        .status()
        .context("run tailwindcss")?;

    anyhow::ensure!(status.success(), "tailwindcss failed with {status}");
    Ok(())
}

pub fn run_esbuild_minify(tooling: &Tooling, input: &Path, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create esbuild output dir {}", parent.display()))?;
    }

    let use_global = Command::new("esbuild").arg("--version").output().is_ok();

    let mut cmd = if let Some(bin) = tooling.esbuild_bin.as_deref() {
        Command::new(bin)
    } else if use_global {
        Command::new("esbuild")
    } else {
        let mut c = Command::new("npx");
        c.arg("--yes").arg("esbuild@0.20.2");
        c
    };

    let status = cmd
        .arg(input)
        .arg("--minify")
        .arg("--target=es2018")
        .arg("--legal-comments=none")
        .arg(format!("--outfile={}", output.display()))
        .status()
        .context("run esbuild")?;

    anyhow::ensure!(status.success(), "esbuild failed for {}", input.display());
    Ok(())
}

pub fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let hash = blake3::hash(&bytes);
    Ok(hash.to_hex()[..10].to_string())
}

pub fn remove_old_fingerprints(dir: &Path, stem: &str, ext: &str) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        // Remove only "{stem}.{hash}{ext}" variants.
        // NOTE: Must NOT match the fresh output file "{stem}{ext}" (e.g. app.css).
        if !(name.starts_with(stem) && name.ends_with(ext)) {
            continue;
        }

        let prefix = format!("{stem}.");
        if !name.starts_with(&prefix) {
            continue;
        }

        // Extract the part between the '.' and the extension.
        if name.len() <= prefix.len() + ext.len() {
            continue;
        }
        let hash_part = &name[prefix.len()..(name.len() - ext.len())];
        if hash_part.len() == 10 && hash_part.chars().all(|c| c.is_ascii_hexdigit()) {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn fingerprint_move(output_file: &Path, stem: &str, ext: &str) -> Result<(String, String)> {
    let dir = output_file.parent().context("output file missing parent")?;

    let hash = hash_file(output_file).context("hash output")?;
    remove_old_fingerprints(dir, stem, ext);

    let fingerprinted_name = format!("{stem}.{hash}{ext}");
    let fingerprinted_path = dir.join(&fingerprinted_name);

    if fingerprinted_path.exists() {
        let _ = fs::remove_file(&fingerprinted_path);
    }

    match fs::rename(output_file, &fingerprinted_path) {
        Ok(()) => {}
        Err(err) => {
            // We should be renaming within the same directory, but in practice tooling can
            // sometimes produce surprising filesystem behavior (e.g. atomic writes + temp files).
            // Fall back to copy+remove and attach better diagnostics.
            if !output_file.exists() {
                return Err(err).with_context(|| {
                    format!(
                        "fingerprint_move: output disappeared before move: {}",
                        output_file.display()
                    )
                });
            }

            if let Some(parent) = fingerprinted_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            fs::copy(output_file, &fingerprinted_path).with_context(|| {
                format!(
                    "copy {} -> {}",
                    output_file.display(),
                    fingerprinted_path.display()
                )
            })?;
            fs::remove_file(output_file)
                .with_context(|| format!("remove {}", output_file.display()))?;

            // Preserve original error details for debugging when it's not a simple ENOENT case.
            if err.kind() != io::ErrorKind::NotFound {
                eprintln!(
                    "warning: rename failed ({}); used copy+remove fallback",
                    err
                );
            }
        }
    }

    Ok((hash, fingerprinted_name))
}

pub fn fingerprint_images(
    src_root: &Path,
    dest_root: &Path,
    manifest: &mut BTreeMap<String, String>,
) -> Result<()> {
    for entry in WalkDir::new(src_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel = path.strip_prefix(src_root).context("strip_prefix images")?;

        let ext_os = path.extension().unwrap_or_else(|| OsStr::new(""));
        let ext = if ext_os.is_empty() {
            String::new()
        } else {
            format!(".{}", ext_os.to_string_lossy())
        };

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");

        let hash = hash_file(path)?;

        let rel_parent = rel.parent().unwrap_or_else(|| Path::new(""));
        let out_dir = dest_root.join(rel_parent);
        fs::create_dir_all(&out_dir)
            .with_context(|| format!("create images dir {}", out_dir.display()))?;

        remove_old_fingerprints(&out_dir, stem, &ext);
        let file_name = format!("{stem}.{hash}{ext}");
        let out_path = out_dir.join(&file_name);

        fs::copy(path, &out_path)
            .with_context(|| format!("copy {} -> {}", path.display(), out_path.display()))?;

        let logical_key = format!("images/{}", rel.to_string_lossy().replace('\\', "/"));
        let fingerprint_value = if rel_parent.as_os_str().is_empty() {
            format!("images/{file_name}")
        } else {
            format!(
                "images/{}/{}",
                rel_parent.to_string_lossy().replace('\\', "/"),
                file_name
            )
        };

        manifest.insert(logical_key, fingerprint_value);
    }

    Ok(())
}

pub fn write_manifest(
    assets_dist: &Path,
    out_dir: &Path,
    manifest: &BTreeMap<String, String>,
) -> Result<()> {
    if manifest.is_empty() {
        return Ok(());
    }

    let manifest_json = serde_json::to_string(manifest).context("serialize manifest")?;

    fs::write(assets_dist.join("manifest.json"), &manifest_json)
        .context("write assets/dist/manifest.json")?;
    fs::write(out_dir.join("assets-manifest.json"), manifest_json)
        .context("write OUT_DIR/assets-manifest.json")?;

    Ok(())
}

pub fn js_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(vec![]);
    }

    let mut entries: Vec<PathBuf> = vec![];
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("js") {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

pub fn assert_partials_contract(templates_dir: &Path) -> Result<()> {
    let partials_dir = templates_dir.join("partials");
    if !partials_dir.is_dir() {
        return Ok(());
    }

    // Keep partials lightweight: they must not be full HTML documents nor use template inheritance.
    // (They can include components, JSON script tags, etc.)
    let forbidden: &[(&str, &str)] = &[
        ("{% extends", "template inheritance"),
        ("<!DOCTYPE", "doctype"),
        ("<html", "html tag"),
        ("<head", "head tag"),
        ("<body", "body tag"),
    ];

    for entry in WalkDir::new(&partials_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("j2") {
            continue;
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("read template {}", path.display()))?;

        for (needle, what) in forbidden {
            if content.contains(needle) {
                bail!(
                    "partials contract violation in {}: found {} ({})",
                    path.display(),
                    needle,
                    what
                );
            }
        }
    }

    Ok(())
}
