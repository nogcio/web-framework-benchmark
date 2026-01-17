use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;

#[path = "build_support.rs"]
mod build_support;

use build_support::*;

fn main() {
    if let Err(err) = real_main() {
        panic!("build.rs failed: {:#}", err);
    }
}

fn real_main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let assets_src = manifest_dir.join("assets/src");
    let assets_dist = manifest_dir.join("assets/dist");

    let templates_dir = manifest_dir.join("templates");
    let config = manifest_dir.join("tailwind.config.js");

    let tooling = Tooling::from_env();
    let mut manifest: BTreeMap<String, String> = BTreeMap::new();

    // Contact metadata (derived from Cargo package fields).
    // We keep parsing in build.rs so runtime code can stay const/static.
    println!("cargo:rerun-if-env-changed=CARGO_PKG_AUTHORS");
    if let Ok(authors) = env::var("CARGO_PKG_AUTHORS") {
        let email = extract_first_email(&authors).unwrap_or(authors);
        println!("cargo:rustc-env=WFB_CONTACT_EMAIL={email}");
    }

    // Rebuild triggers.
    rerun_if_changed_recursive(&assets_src.join("css"));
    rerun_if_changed_recursive(&assets_src.join("js"));
    rerun_if_changed_recursive(&assets_src.join("images"));
    rerun_if_changed_recursive(&templates_dir);
    assert_partials_contract(&templates_dir)?;
    println!("cargo:rerun-if-changed={}", config.display());

    // CSS
    let css_src = assets_src.join("css/app.css");
    let css_dist = assets_dist.join("css");
    ensure_clean_dir(&css_dist)?;
    let css_out = css_dist.join("app.css");
    run_tailwind(&tooling, &config, &css_src, &css_out)?;
    let (_hash, name) = fingerprint_move(&css_out, "app", ".css")?;
    manifest.insert("css/app.css".to_string(), format!("css/{name}"));

    // JS
    let js_src = assets_src.join("js");
    let js_dist = assets_dist.join("js");
    ensure_clean_dir(&js_dist)?;
    for entry in js_entries(&js_src)? {
        let out_name = entry
            .file_name()
            .and_then(|s| s.to_str())
            .context("js filename")?;
        let stem = entry
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(out_name);

        let out_file = js_dist.join(out_name);
        run_esbuild_minify(&tooling, &entry, &out_file)?;
        let (_hash, fp_name) = fingerprint_move(&out_file, stem, ".js")?;
        manifest.insert(format!("js/{out_name}"), format!("js/{fp_name}"));
    }

    // Images
    let images_src = assets_src.join("images");
    let images_dist = assets_dist.join("images");
    ensure_clean_dir(&images_dist)?;
    fingerprint_images(&images_src, &images_dist, &mut manifest)?;

    // Keep original logo URL stable for OG/meta caches.
    // We still generate fingerprints for other images/icons.
    let logo_src = images_src.join("logo.svg");
    if logo_src.is_file() {
        let logo_dist = images_dist.join("logo.svg");
        if let Some(parent) = logo_dist.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&logo_src, &logo_dist)
            .with_context(|| format!("copy {} -> {}", logo_src.display(), logo_dist.display()))?;

        manifest.insert("images/logo.svg".to_string(), "images/logo.svg".to_string());

        // If a fingerprinted logo was generated earlier, remove it to avoid clutter.
        let hash = hash_file(&logo_src)?;
        let fp = images_dist.join(format!("logo.{hash}.svg"));
        let _ = fs::remove_file(fp);
    }

    // Keep original preview URL stable for OG/twitter cards.
    let preview_src = images_src.join("preview.png");
    if preview_src.is_file() {
        let preview_dist = images_dist.join("preview.png");
        if let Some(parent) = preview_dist.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&preview_src, &preview_dist).with_context(|| {
            format!(
                "copy {} -> {}",
                preview_src.display(),
                preview_dist.display()
            )
        })?;

        manifest.insert(
            "images/preview.png".to_string(),
            "images/preview.png".to_string(),
        );

        // If a fingerprinted preview exists (future-proof), remove it.
        let hash = hash_file(&preview_src)?;
        let fp = images_dist.join(format!("preview.{hash}.png"));
        let _ = fs::remove_file(fp);
    }

    // Manifest
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    write_manifest(&assets_dist, &out_dir, &manifest)?;

    Ok(())
}

fn extract_first_email(authors: &str) -> Option<String> {
    // Typical Cargo authors format: "Name <email@host>" (comma-separated for multiple).
    let lt = authors.find('<')?;
    let gt = authors[lt + 1..].find('>')? + lt + 1;
    let email = authors[lt + 1..gt].trim();
    if email.is_empty() {
        None
    } else {
        Some(email.to_string())
    }
}
