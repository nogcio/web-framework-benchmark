pub mod command;

use crate::exec::Executor;
use indicatif::ProgressBar;
use self::command::{DockerRunCommand, DockerBuildCommand, DockerStopCommand, DockerRmCommand, DockerInspectCommand, DockerSaveCommand, DockerLoadCommand, DockerStatsCommand};

#[derive(Clone)]
pub struct DockerManager<E: Executor> {
    executor: E,
    sudo: bool,
}

impl<E: Executor> DockerManager<E> {
    pub fn new(executor: E, sudo: bool) -> Self {
        Self { executor, sudo }
    }

    pub async fn build(&self, docker_file: Option<&str>, image_name: &str, context_path: &str, pb: &ProgressBar) -> anyhow::Result<()> {
        let cmd = DockerBuildCommand::new(self.sudo, docker_file, image_name, context_path);
        self.executor.execute(cmd, pb).await.map(|_| ())
    }

    pub async fn build_with_platform_and_output(&self, docker_file: Option<&str>, image_name: &str, context_path: &str, platform: &str, output: &str, pb: &ProgressBar) -> anyhow::Result<()> {
        let cmd = DockerBuildCommand::new(self.sudo, docker_file, image_name, context_path).with_platform(platform).with_output(output);
        self.executor.execute(cmd, pb).await.map(|_| ())
    }

    pub async fn save(&self, image_name: &str, output_path: &str, pb: &ProgressBar) -> anyhow::Result<()> {
        let cmd = DockerSaveCommand::new(self.sudo, image_name, output_path);
        self.executor.execute(cmd, pb).await.map(|_| ())
    }

    pub async fn load(&self, input_path: &str, pb: &ProgressBar) -> anyhow::Result<()> {
        let cmd = DockerLoadCommand::new(self.sudo, input_path);
        self.executor.execute(cmd, pb).await.map(|_| ())
    }

    pub async fn stop_and_remove(&self, container_name: &str, pb: &ProgressBar) {
        let stop_cmd = DockerStopCommand::new(self.sudo, container_name);
        let _ = self.executor.execute(stop_cmd, pb).await;
        
        let rm_cmd = DockerRmCommand::new(self.sudo, container_name);
        let _ = self.executor.execute(rm_cmd, pb).await;
    }

    pub async fn stop_all_containers(&self, pb: &ProgressBar) {
        let docker = if self.sudo { "sudo docker" } else { "docker" };
        // We use || true to ignore errors if no containers exist
        let cmd = format!("{} stop $({} ps -aq) || true", docker, docker);
        let _ = self.executor.execute(cmd, pb).await;
        
        let cmd = format!("{} rm $({} ps -aq) || true", docker, docker);
        let _ = self.executor.execute(cmd, pb).await;
    }

    pub fn run_command<'a>(&'a self, image: &'a str, name: &'a str) -> DockerRunCommand<'a> {
        DockerRunCommand::new(self.sudo, image, name)
    }
    
    pub async fn execute_run(&self, cmd: DockerRunCommand<'_>, pb: &ProgressBar) -> anyhow::Result<String> {
        self.executor.execute(cmd, pb).await
    }

    pub async fn execute_run_with_std_out(&self, cmd: DockerRunCommand<'_>, on_stdout: impl Fn(&str) + Send + Sync + 'static, pb: &ProgressBar) -> anyhow::Result<String> {
        self.executor.execute_with_std_out(cmd, on_stdout, pb).await
    }

    pub async fn inspect(&self, container_name: &str, format: &str) -> anyhow::Result<String> {
        let cmd = DockerInspectCommand::new(self.sudo, container_name, format);
        let pb = ProgressBar::hidden();
        self.executor.execute(cmd, &pb).await
    }

    pub async fn stats(&self, container_name: &str, format: &str) -> anyhow::Result<String> {
        let cmd = DockerStatsCommand::new(self.sudo, container_name, format);
        let pb = ProgressBar::hidden();
        self.executor.execute(cmd, &pb).await
    }
}
