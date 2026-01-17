use crate::consts;
use crate::db_config::get_db_config;
use crate::docker::DockerManager;
use crate::exec::Executor;
use crate::exec::local::LocalExecutor;
use crate::runner::Runner;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;
use wfb_storage::{Benchmark, DatabaseKind};

impl<E: Executor + Clone + Send + 'static> Runner<E> {
    pub async fn build_database_images_impl(
        &self,
        db_kinds: Vec<DatabaseKind>,
        mb: &MultiProgress,
    ) -> anyhow::Result<()> {
        if db_kinds.is_empty() {
            return Ok(());
        }

        let mut handles = vec![];
        for db in db_kinds {
            let pb = mb.add(ProgressBar::new_spinner());
            let style =
                match ProgressStyle::default_spinner().template("{spinner:.blue} {prefix} {msg}") {
                    Ok(style) => style,
                    Err(_) => ProgressStyle::default_spinner(),
                };
            pb.set_style(style);
            pb.set_prefix(format!("[{:?}]", db));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message(format!("Building: {:?}", db));

            let runner = self.clone();
            handles.push(tokio::spawn(async move {
                let res = runner.build_database_image(&db, &pb).await;
                let style = match ProgressStyle::default_spinner().template("{msg}") {
                    Ok(style) => style,
                    Err(_) => ProgressStyle::default_spinner(),
                };
                pb.set_style(style);
                match res {
                    Ok(_) => {
                        pb.finish_with_message(format!("{} {:?}", console::style("✔").green(), db));
                        Ok(())
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "{} {:?} Failed: {}",
                            console::style("✘").red(),
                            db,
                            e
                        ));
                        Err(e)
                    }
                }
            }));
        }

        for h in handles {
            h.await??;
        }
        Ok(())
    }

    pub async fn deploy_wrkr_impl(&self, mb: &MultiProgress) -> anyhow::Result<()> {
        let pb = mb.add(ProgressBar::new_spinner());
        let style =
            match ProgressStyle::default_spinner().template("{spinner:.blue} {prefix} {msg}") {
                Ok(style) => style,
                Err(_) => ProgressStyle::default_spinner(),
            };
        pb.set_style(style);
        pb.set_prefix("[wrkr]");
        pb.enable_steady_tick(Duration::from_millis(100));

        pb.set_message("Deploying wrkr...");

        let res = async {
            if self.config.is_remote {
                let local_exec = LocalExecutor::new();
                let local_docker = DockerManager::new(local_exec.clone(), false);

                let image_name = consts::WRKR_IMAGE;
                let tar_path = "/tmp/wrkr.tar";
                let remote_tar_path = format!("{}/wrkr.tar", consts::REMOTE_WRKR_PATH);

                pb.set_message("Detecting remote architecture...");
                let uname = self.wrkr_executor.execute("uname -m", &pb).await?;
                let arch = uname.trim();
                let platform = match arch {
                    "x86_64" => "linux/amd64",
                    "aarch64" | "arm64" => "linux/arm64",
                    _ => {
                        pb.set_message(format!(
                            "Unknown architecture: {}, defaulting to linux/amd64",
                            arch
                        ));
                        "linux/amd64"
                    }
                };

                pb.set_message(format!("Building wrkr image locally for {}", platform));

                local_docker
                    .build_with_platform_and_output(
                        Some("Dockerfile.wrkr"),
                        image_name,
                        ".",
                        platform,
                        &format!("type=docker,dest={}", tar_path),
                        &pb,
                    )
                    .await?;

                pb.set_message("Copying wrkr image to remote");
                pb.set_style(
                    match ProgressStyle::default_spinner()
                        .template("{spinner:.blue} {prefix} [{bar:40.cyan/blue}] {msg}")
                    {
                        Ok(style) => style.progress_chars("#>-"),
                        Err(_) => ProgressStyle::default_spinner().progress_chars("#>-"),
                    },
                );
                self.wrkr_executor
                    .cp(tar_path, &remote_tar_path, &pb)
                    .await?;

                pb.set_style(
                    match ProgressStyle::default_spinner()
                        .template("{spinner:.blue} {prefix} {msg}")
                    {
                        Ok(style) => style,
                        Err(_) => ProgressStyle::default_spinner(),
                    },
                );

                pb.set_message("Loading wrkr image on remote");
                self.wrkr_docker.load(&remote_tar_path, &pb).await?;
            } else {
                let local_exec = LocalExecutor::new();
                let local_docker = DockerManager::new(local_exec.clone(), false);
                pb.set_message("Building wrkr image locally");
                local_docker
                    .build(Some("Dockerfile.wrkr"), consts::WRKR_IMAGE, ".", &pb)
                    .await?;
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;

        let style = match ProgressStyle::default_spinner().template("{msg}") {
            Ok(style) => style,
            Err(_) => ProgressStyle::default_spinner(),
        };
        pb.set_style(style);

        match res {
            Ok(_) => {
                pb.finish_with_message(format!("{} wrkr", console::style("✔").green()));
                Ok(())
            }
            Err(e) => {
                pb.finish_with_message(format!("{} wrkr Failed: {}", console::style("✘").red(), e));
                Err(e)
            }
        }
    }

    pub async fn build_database_image(
        &self,
        db_kind: &DatabaseKind,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let config = get_db_config(db_kind);
        let temp_dir = format!("{}/{}", consts::REMOTE_DB_PATH, config.image_name);

        self.build_image_with_progress(
            &self.db_executor,
            &self.db_docker,
            config.image_name,
            &temp_dir,
            pb,
            || async { self.db_executor.cp(config.build_path, &temp_dir, pb).await },
        )
        .await
    }

    pub async fn build_benchmark_image(
        &self,
        benchmark: &Benchmark,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let temp_dir = format!("{}/{}", consts::REMOTE_APP_PATH, benchmark.name);

        self.build_image_with_progress(
            &self.executor,
            &self.app_docker,
            &benchmark.name,
            &temp_dir,
            pb,
            || async {
                let temp_dir_benchmarks_data = format!("{}/benchmarks_data", temp_dir);
                self.executor.mkdir(&temp_dir_benchmarks_data).await?;
                self.executor.cp(&benchmark.path, &temp_dir, pb).await?;
                pb.set_position(0);
                self.executor
                    .cp(consts::BENCHMARK_DATA, &temp_dir_benchmarks_data, pb)
                    .await?;
                Ok(())
            },
        )
        .await
    }

    async fn build_image_with_progress<F, Fut>(
        &self,
        executor: &E,
        docker: &crate::docker::DockerManager<E>,
        image_name: &str,
        temp_dir: &str,
        pb: &ProgressBar,
        prepare_context: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        executor.mkdir(temp_dir).await?;
        let original_style = pb.style().clone();
        pb.set_style(
            match ProgressStyle::default_bar()
                .template("{spinner:.green} {prefix} [{bar:40.cyan/blue}] {msg}")
            {
                Ok(style) => style.progress_chars("#>-"),
                Err(_) => ProgressStyle::default_bar().progress_chars("#>-"),
            },
        );
        pb.set_length(100);

        prepare_context().await?;

        pb.set_style(original_style);
        pb.set_position(0);

        docker.build(None, image_name, temp_dir, pb).await
    }
}
