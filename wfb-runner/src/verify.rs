use std::time::Duration;
use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use console::style;
use wfb_storage::Benchmark;
use crate::exec::Executor;
use crate::pipeline;

pub async fn verify_benchmark<T: Executor>(executor: T, benchmark: &Benchmark, insert_index: usize, m: &MultiProgress) -> Result<()> {
    let pb = m.insert(insert_index, ProgressBar::new_spinner());
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.blue} {msg}")
        .unwrap());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(format!("Verifying {}", benchmark.name));

    let result = async {
        pipeline::build_image(&executor, &benchmark.name, &benchmark.path, &pb).await?;
        if let Some(db_kind) = &benchmark.database {
            pipeline::setup_database(&executor, &benchmark.name, db_kind, &pb).await?;
        }
        pipeline::run_app(&executor, benchmark, &pb).await?;
        pipeline::run_tests(benchmark, "http://localhost:54320", &pb).await?;
        
        Ok::<(), anyhow::Error>(())
    }.await;
    pb.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());

    pipeline::cleanup(&executor, &benchmark.name, &benchmark.database, &pb).await?;

    match result {
        Ok(_) => {
            pb.finish_with_message(format!("{} {} Verified", style("✔").green(), benchmark.name));
            Ok(())
        }
        Err(e) => {
            pb.finish_with_message(format!("{} {} Failed: {}", style("✘").red(), benchmark.name, e));
            Err(e)
        }
    }
}