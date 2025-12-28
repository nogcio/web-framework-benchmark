use std::time::Duration;
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::style;
use wfb_storage::Benchmark;
use crate::exec::Executor;
use crate::pipeline;

pub async fn run_benchmark<T: Executor>(executor: T, benchmark: &Benchmark, insert_index: usize, m: &MultiProgress, global_pb: &ProgressBar) -> Result<()> {
    let pb = m.insert(insert_index, ProgressBar::new_spinner());
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.blue} {msg}")
        .unwrap());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(format!("Running {}", benchmark.name));

    let result = async {
        pipeline::build_image(&executor, &benchmark.name, &benchmark.path, &pb).await?;
        if let Some(db_kind) = &benchmark.database {
            pipeline::setup_database(&executor, &benchmark.name, db_kind, &pb).await?;
        }
        pipeline::run_app(&executor, benchmark, &pb).await?;
        
        // Verify first
        pb.set_message(format!("Verifying {}", benchmark.name));
        pipeline::run_tests(benchmark, "http://localhost:54320", &pb).await?;
        
        // Run benchmarks
        pb.set_message(format!("Benchmarking {}", benchmark.name));
        let results = pipeline::run_benchmarks(&executor, benchmark, "http://host.docker.internal:54320", &pb, global_pb).await?;
        
        Ok::<_, anyhow::Error>(results)
    }.await;
    pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());

    pipeline::cleanup(&executor, &benchmark.name, &benchmark.database, &pb).await?;

    match result {
        Ok(results) => {
            let mut msg = format!("{} {} Complete", style("✔").green(), benchmark.name);
            for (test, stats) in results {
                msg.push_str(&format!("\n  {} {:?}: {} rps", 
                    style("✔").green(), 
                    test, 
                    style(format!("{:.0}", stats.requests_per_sec)).cyan()
                ));
            }
            pb.finish_with_message(msg);
            Ok(())
        }
        Err(e) => {
            pb.finish_with_message(format!("{} {} Failed: {}", style("✘").red(), benchmark.name, e));
            Err(e)
        }
    }
}
