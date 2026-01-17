#[allow(unused_imports)]
use crate::filters;

mod bench;
mod bench_charts;
mod context;
mod github;
mod helpers;
mod index;
mod render;
mod types;

pub use bench::bench_path_handler;
pub use bench_charts::bench_charts_partials_path_handler;
pub use github::github_stars_partials_handler;
pub use index::index_path_handler;
pub use index::index_update_path_handler;
pub use index::root_handler;
