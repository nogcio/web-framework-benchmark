use crate::prelude::*;
use super::{BenchmarkEnvironment, Endpoint, ServerUsage, WrkResult};

pub struct RemoteBenchmarkEnvironment {
    // TODO: hold SSH clients / config
}

impl RemoteBenchmarkEnvironment {
    pub fn new() -> Self {
        RemoteBenchmarkEnvironment {}
    }
}

#[async_trait::async_trait]
impl BenchmarkEnvironment for RemoteBenchmarkEnvironment {
    async fn prepare(&mut self, _framework_path: &std::path::Path) -> Result<()> {
        unimplemented!()
    }

    async fn start_db(&mut self) -> Result<Endpoint> {
        unimplemented!()
    }

    async fn stop_db(&mut self) -> Result<()> {
        unimplemented!()
    }

    async fn start_app(&mut self, _db_endpoint: &Endpoint) -> Result<Endpoint> {
        unimplemented!()
    }

    async fn stop_app(&mut self) -> Result<ServerUsage> {
        unimplemented!()
    }

    async fn get_app_info(&self, _app_endpoint: &Endpoint) -> Result<crate::http_probe::ServerInfo> {
        unimplemented!()
    }

    async fn exec_wrk(&self, _app_endpoint: &Endpoint, _script: Option<String>) -> Result<WrkResult> {
        unimplemented!()
    }
}
