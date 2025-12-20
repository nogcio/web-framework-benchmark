pub fn parse_latency(s: &str) -> Option<std::time::Duration> {
    let re = regex::Regex::new(r"([\d.]+)([a-zA-Zμ]+)").unwrap();
    re.captures(s).and_then(|cap| {
        let value: f64 = cap.get(1)?.as_str().parse().ok()?;
        let unit = cap.get(2)?.as_str();
        match unit {
            "us" | "μs" => Some(std::time::Duration::from_micros((value * 1.0) as u64)),
            "ms" => Some(std::time::Duration::from_micros((value * 1000.0) as u64)),
            "s" => Some(std::time::Duration::from_secs_f64(value)),
            _ => None,
        }
    })
}

pub fn parse_metric(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix('k') {
        stripped.parse::<f64>().ok().map(|v| v * 1_000.0)
    } else if let Some(stripped) = s.strip_suffix('M') {
        stripped.parse::<f64>().ok().map(|v| v * 1_000_000.0)
    } else {
        s.parse::<f64>().ok()
    }
}

pub fn parse_mem(s: &str) -> Option<u64> {
    let s = s.trim();
    let mut num = String::new();
    let mut unit = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
        } else {
            unit.push(c);
        }
    }
    let value: f64 = num.parse().ok()?;
    let multiplier = match unit.trim() {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * multiplier) as u64)
}
