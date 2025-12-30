use std::{fs::OpenOptions, io::{self, BufWriter, Write}, path::PathBuf};

use rand::{RngCore, rngs::OsRng};


pub async fn ensure_data() -> std::result::Result<(), io::Error> {
    let mut base = std::env::current_dir()?;
    base.push("benchmarks_data");

    fn create_if_missing_random(path: PathBuf, size: u64) -> std::result::Result<(), io::Error> {
        if path.exists() {
            return Ok(());
        }
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        let mut w = BufWriter::new(f);
        let mut remaining = size;
        let mut buf = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut rng = OsRng;
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, buf.len() as u64) as usize;
            rng.fill_bytes(&mut buf[..to_write]);
            w.write_all(&buf[..to_write])?;
            remaining -= to_write as u64;
        }
        w.flush()?;
        Ok(())
    }

    // sizes in bytes
    let f15 = base.join("15kb.bin");
    let f1m = base.join("1mb.bin");
    //let f10m = base.join("10mb.bin");

    create_if_missing_random(f15, 15 * 1024)?;
    create_if_missing_random(f1m, 1024 * 1024)?;
    //create_if_missing_random(f10m, 10 * 1024 * 1024)?;

    Ok(())
}