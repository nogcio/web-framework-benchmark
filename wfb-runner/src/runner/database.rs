use indicatif::ProgressBar;
use wfb_storage::DatabaseKind;
use crate::exec::Executor;
use crate::consts;
use crate::db_config::get_db_config;
use crate::runner::Runner;

impl<E: Executor + Clone + Send + 'static> Runner<E> {
    pub async fn setup_database(&self, db_kind: &DatabaseKind, pb: &ProgressBar) -> anyhow::Result<()> {
        let config = get_db_config(db_kind);

        let mut cmd = self.db_docker.run_command(config.image_name, config.image_name)
            .port(consts::DB_PORT_EXTERNAL, config.port);

        for (k, v) in config.env_vars {
            cmd = cmd.env(k, v);
        }

        self.db_docker.execute_run(cmd, pb).await?;

        Ok(())
    }

    pub async fn wait_for_db_ready(&self, db_kind: &DatabaseKind, pb: &ProgressBar) -> anyhow::Result<()> {
        let config = get_db_config(db_kind);
        pb.set_message(format!("Waiting for DB - {:?}", db_kind));
        self.wait_for_container_ready(&self.db_docker, config.image_name, pb).await
    }
}
