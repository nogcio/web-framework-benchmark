use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let assets_src = manifest_dir.join("assets/src");
    let assets_dist = manifest_dir.join("assets/dist");
    fs::create_dir_all(&assets_dist).ok();

    let input = assets_src.join("css/app.css");
    let output = assets_dist.join("css/app.css");
    let config = manifest_dir.join("tailwind.config.js");
    let templates_dir = manifest_dir.join("templates");

    println!("cargo:rerun-if-changed={}", input.display());
    println!("cargo:rerun-if-changed={}", config.display());
    println!("cargo:rerun-if-changed={}", templates_dir.display());
    print_rerun_if_changed(&assets_src.join("js"));
    print_rerun_if_changed(&assets_src.join("images"));

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("Failed to create css output directory");
    }

    let tailwind_bin = env::var("TAILWINDCSS_BIN").ok();
    let mut command = if let Some(bin) = tailwind_bin {
        Command::new(bin)
    } else if Command::new("tailwindcss").arg("--help").output().is_ok() {
        Command::new("tailwindcss")
    } else {
        let mut cmd = Command::new("npx");
        cmd.arg("--yes").arg("tailwindcss@3.4.17");
        cmd
    };

    let status = command
        .arg("-c")
        .arg(config)
        .arg("-i")
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--minify")
        .status()
        .expect("Failed to run tailwindcss");

    if !status.success() {
        panic!("tailwindcss build failed with status {status}");
    }

    let js_src = assets_src.join("js");
    let js_dist = assets_dist.join("js");
    if js_src.is_dir() {
        fs::create_dir_all(&js_dist).expect("Failed to create js output directory");
        let esbuild_bin = env::var("ESBUILD_BIN").ok();
        let use_global = Command::new("esbuild").arg("--version").output().is_ok();
        for entry in fs::read_dir(&js_src).expect("Failed to read assets/js") {
            let entry = entry.expect("Failed to read js entry");
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("js") {
                continue;
            }
            let file_name = path.file_name().expect("Missing js file name");
            let out_file = js_dist.join(file_name);

            let mut cmd = if let Some(bin) = esbuild_bin.as_ref() {
                Command::new(bin)
            } else if use_global {
                Command::new("esbuild")
            } else {
                let mut cmd = Command::new("npx");
                cmd.arg("--yes").arg("esbuild@0.20.2");
                cmd
            };

            let status = cmd
                .arg(&path)
                .arg("--minify")
                .arg("--target=es2018")
                .arg("--legal-comments=none")
                .arg(format!("--outfile={}", out_file.display()))
                .status()
                .expect("Failed to run esbuild");

            if !status.success() {
                panic!("esbuild failed for {:?} with status {status}", file_name);
            }
        }
    }

    let images_src = assets_src.join("images");
    let images_dist = assets_dist.join("images");
    if images_src.is_dir() {
        copy_dir_recursive(&images_src, &images_dist).expect("Failed to copy images");
    }
}

fn print_rerun_if_changed(path: &Path) {
    if path.is_file() {
        println!("cargo:rerun-if-changed={}", path.display());
    } else if path.is_dir()
        && let Ok(entries) = fs::read_dir(path)
    {
        for entry in entries.flatten() {
            print_rerun_if_changed(&entry.path());
        }
    }
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
