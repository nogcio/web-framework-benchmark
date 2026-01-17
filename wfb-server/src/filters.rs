use askama::{Error as AskamaError, Result as AskamaResult, Values};
use std::borrow::Borrow;
use std::error::Error;
use std::fmt;

use crate::assets_manifest;
use crate::view_models::{EnvironmentView, RunView, TestView};

#[askama::filter_fn]
pub fn format_run_date(
    value: &chrono::DateTime<chrono::Utc>,
    _env: &dyn Values,
) -> AskamaResult<String> {
    Ok(format_datetime_inner(value, "%b %d, %Y"))
}

#[askama::filter_fn]
pub fn format_datetime(
    value: &chrono::DateTime<chrono::Utc>,
    _env: &dyn Values,
    fmt: &str,
) -> AskamaResult<String> {
    Ok(format_datetime_inner(value, fmt))
}

fn format_datetime_inner(value: &chrono::DateTime<chrono::Utc>, fmt: &str) -> String {
    value.format(fmt).to_string()
}

#[askama::filter_fn]
pub fn env(
    active_env: impl std::fmt::Display,
    _env: &dyn Values,
    environments: &[EnvironmentView],
) -> AskamaResult<EnvironmentView> {
    let active_env = active_env.to_string();
    Ok(environments
        .iter()
        .find(|env| env.name == active_env)
        .cloned()
        .unwrap_or(EnvironmentView {
            name: active_env.clone(),
            title: active_env,
            icon: "laptop".to_string(),
            spec: None,
        }))
}

#[askama::filter_fn]
pub fn test(
    active_test: impl std::fmt::Display,
    _env: &dyn Values,
    tests: &[TestView],
) -> AskamaResult<TestView> {
    let active_test = active_test.to_string();
    Ok(tests
        .iter()
        .find(|test| test.id == active_test)
        .cloned()
        .unwrap_or(TestView {
            id: active_test.clone(),
            name: active_test,
            icon: "flask-conical".to_string(),
        }))
}

#[askama::filter_fn]
pub fn run(
    active_run_id: impl std::fmt::Display,
    _env: &dyn Values,
    runs: &[RunView],
) -> AskamaResult<RunView> {
    let active_run_id = active_run_id.to_string();
    Ok(runs
        .iter()
        .find(|run| run.id == active_run_id)
        .cloned()
        .unwrap_or(RunView {
            id: active_run_id,
            created_at: chrono::Utc::now(),
        }))
}

#[askama::filter_fn]
pub fn format_number(value: impl Borrow<f64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    Ok(format_number_inner(value))
}

#[askama::filter_fn]
pub fn format_percent0(value: impl Borrow<f64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    Ok(format!("{:.0}", value.round()))
}

#[askama::filter_fn]
pub fn format_throughput(value: impl Borrow<u64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    Ok(format_throughput_inner(value))
}

#[askama::filter_fn]
pub fn format_latency_ms(value: impl Borrow<u64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    let ms = value / 1000;
    Ok(format!("{} ms", ms))
}

#[askama::filter_fn]
pub fn format_bytes(value: impl Borrow<u64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    Ok(format_bytes_inner(value))
}

#[askama::filter_fn]
pub fn format_percent1(value: impl Borrow<f64>, _env: &dyn Values) -> AskamaResult<String> {
    let value = *value.borrow();
    Ok(format!("{:.1}", value))
}

#[askama::filter_fn]
pub fn soften_color(value: impl std::fmt::Display, _env: &dyn Values) -> AskamaResult<String> {
    Ok(soften_color_inner(&value.to_string()))
}

#[askama::filter_fn]
pub fn asset_path(url: &str, _env: &dyn Values) -> AskamaResult<String> {
    let trimmed = url.trim();
    let key = trimmed.trim_start_matches('/');

    if let Some(mapped) = assets_manifest::resolve(key) {
        return Ok(format!("/{}", mapped.trim_start_matches('/')));
    }

    Err(AskamaError::Custom(Box::new(MissingAssetError {
        logical_path: key.to_string(),
    })))
}

fn asset_path_inner(url: &str) -> String {
    let trimmed = url.trim();
    let key = trimmed.trim_start_matches('/');

    let mapped = assets_manifest::resolve(key)
        .unwrap_or_else(|| panic!("asset_path missing in manifest: {key}"));
    format!("/{}", mapped.trim_start_matches('/'))
}

#[derive(Debug)]
struct MissingAssetError {
    logical_path: String,
}

impl fmt::Display for MissingAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "asset_path missing in manifest: {}", self.logical_path)
    }
}

impl Error for MissingAssetError {}

fn format_number_inner(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let rounded = value.round() as i128;
    format_integer_string(&rounded.to_string())
}

fn format_integer_string(input: &str) -> String {
    let (sign, digits) = if let Some(rest) = input.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", input)
    };

    let len = digits.len();
    if len == 0 {
        return sign.to_string();
    }

    let mut formatted = String::with_capacity(input.len() + input.len() / 3);
    formatted.push_str(sign);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(ch);
    }
    formatted
}

fn format_throughput_inner(bytes_per_sec: u64) -> String {
    const UNITS: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];
    if bytes_per_sec == 0 {
        return "0 B/s".to_string();
    }
    let mut value = bytes_per_sec as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    let unit = UNITS[unit_idx];
    if unit_idx == 0 {
        format!(
            "{} {}",
            format_integer_string(&bytes_per_sec.to_string()),
            unit
        )
    } else if value >= 100.0 {
        format!("{:.0} {}", value, unit)
    } else if value >= 10.0 {
        format!("{:.1} {}", value, unit)
    } else {
        format!("{:.2} {}", value, unit)
    }
}

#[allow(dead_code)]
fn format_bytes_inner(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut value = bytes as f64;
    let mut unit_idx = 0;
    while value >= 1024.0 && unit_idx < UNITS.len() - 1 {
        value /= 1024.0;
        unit_idx += 1;
    }
    let unit = UNITS[unit_idx];
    if unit_idx == 0 {
        format!("{} {}", format_integer_string(&bytes.to_string()), unit)
    } else if value >= 100.0 {
        format!("{:.0} {}", value, unit)
    } else if value >= 10.0 {
        format!("{:.1} {}", value, unit)
    } else {
        format!("{:.2} {}", value, unit)
    }
}

fn soften_color_inner(color: &str) -> String {
    let trimmed = color.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            return format!("#{hex}1A");
        }
        if hex.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            return format!("#{expanded}1A");
        }
    }
    format!("color-mix(in srgb, {trimmed} 15%, transparent)")
}

#[askama::filter_fn]
pub fn icon(name: impl std::fmt::Display, _env: &dyn Values, class: &str) -> AskamaResult<String> {
    let name = name.to_string();
    if name.is_empty() {
        return Ok(String::new());
    }
    // Use CSS mask to allow coloring via currentColor (bg-current)
    let base_url = format!("/images/icons/{}.svg", name);
    let url = asset_path_inner(&base_url);
    Ok(format!(
        r#"<span class="inline-block {}" style="background-color: currentColor; -webkit-mask-image: url('{}'); mask-image: url('{}'); -webkit-mask-repeat: no-repeat; mask-repeat: no-repeat; -webkit-mask-position: center; mask-position: center; -webkit-mask-size: contain; mask-size: contain;" aria-hidden="true"></span>"#,
        class, url, url
    ))
}
