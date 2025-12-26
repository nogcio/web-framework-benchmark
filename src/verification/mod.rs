mod db_read_one;
mod db_read_paging;
mod db_write;
mod hello_world;
mod json;
mod static_files;
mod tweet_service;
mod utils;

use crate::benchmark::BenchmarkTests;
use crate::database::DatabaseKind;
use crate::prelude::*;
use reqwest::Client;

pub async fn verify_test(
    test: &BenchmarkTests,
    base_url: &str,
    database: Option<DatabaseKind>,
) -> Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    info!("Verifying logic for test: {:?}", test);

    match test {
        BenchmarkTests::HelloWorld => hello_world::verify(&client, base_url).await,
        BenchmarkTests::Json => json::verify(&client, base_url).await,
        BenchmarkTests::DbReadOne => db_read_one::verify(&client, base_url, database).await,
        BenchmarkTests::DbReadPaging => db_read_paging::verify(&client, base_url, database).await,
        BenchmarkTests::DbWrite => db_write::verify(&client, base_url).await,
        BenchmarkTests::StaticFilesSmall => {
            static_files::verify(&client, base_url, 15 * 1024, "/files/15kb.bin").await
        }
        BenchmarkTests::StaticFilesMedium => {
            static_files::verify(&client, base_url, 1024 * 1024, "/files/1mb.bin").await
        }
        BenchmarkTests::StaticFilesLarge => {
            static_files::verify(&client, base_url, 10 * 1024 * 1024, "/files/10mb.bin").await
        }
        BenchmarkTests::TweetService => tweet_service::verify(&client, base_url).await,
    }?;

    info!("Verification passed for test: {:?}", test);
    Ok(())
}
