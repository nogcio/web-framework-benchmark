use clap::Parser;
use humanize_bytes::humanize_bytes_binary;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use mimalloc::MiMalloc;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wrkr_api::JsonStats;
use wrkr_core::{BenchmarkConfig, WrkConfig, run_benchmark};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install panic hook to log panics before abort
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC OCCURRED:");
        if let Some(location) = panic_info.location() {
            eprintln!(
                "  Location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("  Message: {}", s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("  Message: {}", s);
        }
        eprintln!("  Backtrace:");
        eprintln!("{:?}", std::backtrace::Backtrace::force_capture());
    }));

    let args = cli::Args::parse();

    if args.output == cli::OutputFormat::Text {
        println!("Running {}s test @ {}", args.duration, args.url);
        println!("  {} connections", args.connections);
        println!("  Script: {}", args.script.display());
    }

    let script_content = tokio::fs::read_to_string(&args.script).await?;

    let step_connections = args.step_connections.map(|s| {
        s.split(',')
            .filter_map(|v| v.trim().parse::<u64>().ok())
            .collect()
    });

    let config = BenchmarkConfig {
        duration: Duration::from_secs(args.duration),
        connections: args.connections,
        start_connections: args.start_connections,
        ramp_up: args.ramp_up.map(Duration::from_secs),
        step_connections,
        step_duration: args.step_duration.map(Duration::from_secs),
        timeout: Some(Duration::from_secs(args.timeout)),
        wrk: WrkConfig {
            script_content,
            host_url: args.url.clone(),
            http2: args.http2,
        },
    };

    let mp = if args.output == cli::OutputFormat::Text {
        Some(MultiProgress::new())
    } else {
        None
    };

    let pb = if let Some(mp) = &mp {
        let pb = mp.add(ProgressBar::new(args.duration));
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} {msg} [{elapsed_precise}] [{bar:40.cyan/blue}]",
            )
            .unwrap()
            .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let pb_clone = pb.clone();
    let mp_clone = mp.clone();
    let error_bars: Arc<Mutex<HashMap<String, ProgressBar>>> = Arc::new(Mutex::new(HashMap::new()));
    let error_bars_clone = error_bars.clone();
    let output_format = args.output.clone();

    let mut last_requests = 0;
    let mut last_bytes = 0;
    let mut last_elapsed = Duration::from_secs(0);

    let stats = run_benchmark(
        config,
        Some(move |p: wrkr_core::StatsSnapshot| {
            let interval_secs = (p.elapsed - last_elapsed).as_secs_f64();
            let current_rps = if interval_secs > 0.0 {
                (p.total_requests - last_requests) as f64 / interval_secs
            } else {
                0.0
            };

            let current_tps = if interval_secs > 0.0 {
                ((p.total_bytes_received - last_bytes) as f64 / interval_secs) as u64
            } else {
                0
            };

            last_requests = p.total_requests;
            last_bytes = p.total_bytes_received;
            last_elapsed = p.elapsed;

            match output_format {
                cli::OutputFormat::Text => {
                    if let Some(pb) = &pb_clone {
                        pb.set_position(p.elapsed.as_secs());

                        let msg = format!(
                            "Conns: {} | RPS: {:.0} | TPS: {}",
                            p.connections,
                            current_rps,
                            humanize_bytes_binary!(current_tps),
                        );
                        pb.set_message(msg);

                        let mut bars = error_bars_clone.lock().unwrap();
                        if let Some(mp) = &mp_clone {
                            for (err, count) in p.errors {
                                if !bars.contains_key(&err) {
                                    let bar = mp.insert(0, ProgressBar::new(0));
                                    bar.set_style(ProgressStyle::with_template("{msg}").unwrap());
                                    bars.insert(err.clone(), bar);
                                }
                                if let Some(bar) = bars.get(&err) {
                                    bar.set_message(format!("Error: {} - {}", err, count));
                                }
                            }
                        }
                    }
                }
                cli::OutputFormat::Json => {
                    let rps = current_rps;
                    let tps = current_tps;

                    let rps_mean = if !p.rps_samples.is_empty() {
                        p.rps_samples.iter().sum::<f64>() / p.rps_samples.len() as f64
                    } else {
                        0.0
                    };

                    let rps_max = p.rps_samples.iter().fold(0.0f64, |a, &b| a.max(b));

                    let rps_stdev = if p.rps_samples.len() > 1 {
                        let variance = p
                            .rps_samples
                            .iter()
                            .map(|value| {
                                let diff = rps_mean - *value;
                                diff * diff
                            })
                            .sum::<f64>()
                            / (p.rps_samples.len() - 1) as f64;
                        variance.sqrt()
                    } else {
                        0.0
                    };

                    let rps_stdev_pct = if rps_mean > 0.0 {
                        rps_stdev / rps_mean * 100.0
                    } else {
                        0.0
                    };

                    let lat_mean = p.latency_histogram.mean();
                    let lat_stdev = p.latency_histogram.stdev();
                    let lat_stdev_pct = if lat_mean > 0.0 {
                        lat_stdev / lat_mean * 100.0
                    } else {
                        0.0
                    };

                    let mut latency_distribution = Vec::new();
                    for percent in &[10, 25, 50, 75, 90, 99] {
                        latency_distribution.push((
                            *percent,
                            p.latency_histogram
                                .value_at_quantile(*percent as f64 / 100.0),
                        ));
                    }

                    // Skip output if no requests have been made yet (avoids empty first line)
                    if p.total_requests > 0 {
                        let json_stats = JsonStats {
                            elapsed_secs: p.elapsed.as_secs(),
                            connections: p.connections,
                            requests_per_sec: rps,
                            bytes_per_sec: tps,
                            total_requests: p.total_requests,
                            total_bytes: p.total_bytes_received,
                            total_errors: p.total_errors,
                            latency_mean: lat_mean,
                            latency_stdev: lat_stdev,
                            latency_max: p.latency_histogram.max(),
                            latency_p50: p.latency_histogram.value_at_quantile(0.50),
                            latency_p75: p.latency_histogram.value_at_quantile(0.75),
                            latency_p90: p.latency_histogram.value_at_quantile(0.90),
                            latency_p99: p.latency_histogram.value_at_quantile(0.99),
                            latency_stdev_pct: lat_stdev_pct,
                            latency_distribution,
                            errors: p.errors,
                            req_per_sec_avg: rps_mean,
                            req_per_sec_stdev: rps_stdev,
                            req_per_sec_max: rps_max,
                            req_per_sec_stdev_pct: rps_stdev_pct,
                        };
                        println!("{}", serde_json::to_string(&json_stats).unwrap());
                    }
                }
            }
        }),
    )
    .await?;
    std::io::stdout().flush().ok();

    if let Some(pb) = pb {
        pb.finish_with_message("Done!");
    }

    {
        let bars = error_bars.lock().unwrap();
        for bar in bars.values() {
            bar.finish_and_clear();
        }
    }

    if args.output == cli::OutputFormat::Text {
        let duration_secs = stats.elapsed.as_secs_f64();
        let total_requests = stats.total_requests;
        let total_bytes = stats.total_bytes_received;

        println!("  Thread Stats   Avg      Stdev     Max   +/- Stdev");

        let lat_mean = stats.latency_histogram.mean();
        let lat_stdev = stats.latency_histogram.stdev();
        let lat_max = stats.latency_histogram.max();

        let lat_mean_u64 = lat_mean as u64;
        let lat_stdev_u64 = lat_stdev as u64;
        let min_lat = lat_mean_u64.saturating_sub(lat_stdev_u64);
        let max_lat = lat_mean_u64.saturating_add(lat_stdev_u64);
        let mut count_within_stdev = 0;
        for item in stats.latency_histogram.iter_recorded() {
            let val = item.value_iterated_to();
            if val >= min_lat && val <= max_lat {
                count_within_stdev += item.count_at_value()
            }
        }
        let lat_within_stdev_pct = if !stats.latency_histogram.is_empty() {
            count_within_stdev as f64 / stats.latency_histogram.len() as f64 * 100.0
        } else {
            0.0
        };

        let fmt_time = |micros: f64| -> String {
            if micros >= 1_000_000.0 {
                format!("{:.2}s", micros / 1_000_000.0)
            } else if micros >= 1_000.0 {
                format!("{:.2}ms", micros / 1_000.0)
            } else {
                format!("{:.2}us", micros)
            }
        };

        println!(
            "    Latency   {:>8} {:>8} {:>8} {:>8.2}%",
            fmt_time(lat_mean),
            fmt_time(lat_stdev),
            fmt_time(lat_max as f64),
            lat_within_stdev_pct
        );

        println!("  Latency Distribution");
        for p in &[50.0, 75.0, 90.0, 99.0] {
            let val = stats.latency_histogram.value_at_quantile(p / 100.0);
            println!("     {:.0}%   {:>8}", p, fmt_time(val as f64));
        }

        println!(
            "  {} requests in {:.2}s, {} read",
            total_requests,
            duration_secs,
            humanize_bytes_binary!(total_bytes)
        );

        let rps = total_requests as f64 / duration_secs;
        let tps = total_bytes as f64 / duration_secs;

        println!("Requests/sec: {:.2}", rps);
        println!("Transfer/sec: {}", humanize_bytes_binary!(tps as u64));

        let mut non_2xx = 0;
        let mut timeouts = 0;
        let mut read_errs = 0;
        let mut connect_errs = 0;
        let mut write_errs = 0;
        let mut other_errs = 0;

        for (err, count) in &stats.errors {
            if err == "Non 2xx and non 3xx status code" {
                non_2xx += count;
            } else if err == "Request timeout" {
                timeouts += count;
            } else {
                let err_lower = err.to_lowercase();
                if err_lower.contains("connect")
                    || err_lower.contains("dns")
                    || err_lower.contains("resolve")
                {
                    connect_errs += count;
                } else if err_lower.contains("read")
                    || err_lower.contains("receive")
                    || err_lower.contains("closed")
                    || err_lower.contains("incomplete")
                    || err_lower.contains("response processing")
                {
                    read_errs += count;
                } else if err_lower.contains("write") || err_lower.contains("send") {
                    write_errs += count;
                } else {
                    other_errs += count;
                }
            }
        }

        if non_2xx > 0 {
            println!("  Non-2xx or 3xx responses: {}", non_2xx);
        }

        if other_errs > 0 {
            println!("  Errors: {}", other_errs);
        }

        println!(
            "Socket errors: connect {}, read {}, write {}, timeout {}",
            connect_errs, read_errs, write_errs, timeouts
        );
    }

    std::io::stdout().flush().ok();

    Ok(())
}
