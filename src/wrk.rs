use regex::Regex;
use tokio::process::Command;

use crate::{
    parsers::{parse_latency, parse_metric},
    prelude::*,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WrkResult {
    pub requests_per_sec: f64,
    pub transfer_per_sec: String,
    pub latency_avg: std::time::Duration,
    pub latency_stdev: std::time::Duration,
    pub latency_max: std::time::Duration,
    pub latency_stdev_pct: f64,
    pub latency_distribution: Vec<(u8, std::time::Duration)>,
    pub req_per_sec_avg: f64,
    pub req_per_sec_stdev: f64,
    pub req_per_sec_max: f64,
    pub req_per_sec_stdev_pct: f64,
    pub errors: i64,
}

pub async fn start_wrk(
    url: &str,
    duration: u64,
    threads: u32,
    connections: u32,
    script: Option<&str>,
) -> Result<WrkResult> {
    let url = url.replace("localhost", "host.docker.internal");

    let mut cmd = Command::new("docker");
    cmd.arg("run")
        .arg("--rm")
        .arg("-v")
        .arg("./scripts:/work/scripts")
        .arg("-w")
        .arg("/work")
        .arg("williamyeh/wrk")
        .arg("-t")
        .arg(threads.to_string())
        .arg("-c")
        .arg(connections.to_string())
        .arg("-d")
        .arg(format!("{}s", duration))
        .arg("--latency");
    if let Some(s) = script {
        cmd.arg("-s").arg(s);
    }
    cmd.arg(&url);
    let output_str = exec(&mut cmd).await?;
    let wrk_output_vec: Vec<String> = output_str.lines().map(|s| s.to_string()).collect();
    let wrk_result = parse_wrk_output(&wrk_output_vec)?;
    Ok(wrk_result)
}

fn parse_wrk_output(lines: &[String]) -> Result<WrkResult> {
    let mut requests_per_sec = None;
    let mut transfer_per_sec = None;
    let mut latency_avg = None;
    let mut latency_stdev = None;
    let mut latency_max = None;
    let mut latency_stdev_pct = None;
    let mut latency_distribution = Vec::new();
    let mut req_per_sec_avg = None;
    let mut req_per_sec_stdev = None;
    let mut req_per_sec_max = None;
    let mut req_per_sec_stdev_pct = None;
    let mut errors = None;

    let re_rps = Regex::new(r"Requests/sec:\s+([\d.]+)").unwrap();
    let re_tps = Regex::new(r"Transfer/sec:\s+([\w./]+)").unwrap();
    let re_latency_full = Regex::new(r"Latency\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+)%\s+([\d.]+)%").unwrap();
    let re_latency_simple =
        Regex::new(r"Latency\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)")
            .unwrap();
    let re_latency_dist = Regex::new(r"\s*(\d+)%\s+([\d.]+[a-zA-Zμ]+)").unwrap();
    let re_thread_stats_latency = Regex::new(
        r"^\s*Latency\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+[a-zA-Zμ]+)\s+([\d.]+)%$",
    )
    .unwrap();
    let re_thread_stats_req =
        Regex::new(r"^\s*Req/Sec\s+([\d.]+[kM]?)\s+([\d.]+[kM]?)\s+([\d.]+[kM]?)\s+([\d.]+)%$")
            .unwrap();
    let re_errors = Regex::new(r"^(Errors):\s+(\d+)$").unwrap();

    for line in lines {
        if let Some(cap) = re_thread_stats_latency.captures(line) {
            latency_avg = cap.get(1).and_then(|m| parse_latency(m.as_str()));
            latency_stdev = cap.get(2).and_then(|m| parse_latency(m.as_str()));
            latency_max = cap.get(3).and_then(|m| parse_latency(m.as_str()));
            latency_stdev_pct = cap.get(4).and_then(|m| m.as_str().parse::<f64>().ok());
        } else if let Some(cap) = re_thread_stats_req.captures(line) {
            req_per_sec_avg = cap.get(1).and_then(|m| parse_metric(m.as_str()));
            req_per_sec_stdev = cap.get(2).and_then(|m| parse_metric(m.as_str()));
            req_per_sec_max = cap.get(3).and_then(|m| parse_metric(m.as_str()));
            req_per_sec_stdev_pct = cap.get(4).and_then(|m| m.as_str().parse::<f64>().ok());
        } else if let Some(cap) = re_rps.captures(line) {
            requests_per_sec = cap.get(1).and_then(|m| m.as_str().parse::<f64>().ok());
        } else if let Some(cap) = re_tps.captures(line) {
            transfer_per_sec = cap.get(1).map(|m| m.as_str().to_string());
        } else if let Some(cap) = re_latency_full.captures(line) {
            latency_avg = cap.get(1).and_then(|m| parse_latency(m.as_str()));
            latency_stdev = cap.get(2).and_then(|m| parse_latency(m.as_str()));
            latency_max = cap.get(3).and_then(|m| parse_latency(m.as_str()));
            latency_stdev_pct = cap.get(4).and_then(|m| m.as_str().parse::<f64>().ok());
        } else if let Some(cap) = re_latency_simple.captures(line) {
            latency_avg = cap.get(1).and_then(|m| parse_latency(m.as_str()));
            latency_stdev = cap.get(2).and_then(|m| parse_latency(m.as_str()));
            latency_max = cap.get(3).and_then(|m| parse_latency(m.as_str()));
            latency_stdev_pct = Some(0.0);
        } else if let Some(cap) = re_errors.captures(line) {
            errors = cap.get(2).and_then(|m| m.as_str().parse::<i64>().ok());
        } else if let Some(cap) = re_latency_dist.captures(line) {
            let pct = cap.get(1).and_then(|m| m.as_str().parse::<u8>().ok());
            let dur = cap.get(2).and_then(|m| parse_latency(m.as_str()));
            if let (Some(pct), Some(dur)) = (pct, dur) {
                latency_distribution.push((pct, dur));
            }
        }
    }
    if let (
        Some(requests_per_sec),
        Some(transfer_per_sec),
        Some(latency_avg),
        Some(latency_stdev),
        Some(latency_max),
        Some(latency_stdev_pct),
        Some(req_per_sec_avg),
        Some(req_per_sec_stdev),
        Some(req_per_sec_max),
        Some(req_per_sec_stdev_pct),
    ) = (
        requests_per_sec,
        transfer_per_sec,
        latency_avg,
        latency_stdev,
        latency_max,
        latency_stdev_pct,
        req_per_sec_avg,
        req_per_sec_stdev,
        req_per_sec_max,
        req_per_sec_stdev_pct,
    ) {
        if latency_distribution.is_empty() {
            return Err(Error::WrkParseError(
                "wrk output: missing latency_distribution".to_string(),
            ));
        }
        let errors = errors.unwrap_or(0);
        Ok(WrkResult {
            requests_per_sec,
            transfer_per_sec,
            latency_avg,
            latency_stdev,
            latency_max,
            latency_stdev_pct,
            latency_distribution,
            req_per_sec_avg,
            req_per_sec_stdev,
            req_per_sec_max,
            req_per_sec_stdev_pct,
            errors,
        })
    } else {
        Err(Error::WrkParseError(
            "wrk output: missing required field".to_string(),
        ))
    }
}
