use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub enum FileChangeEvent {
    ConfigChanged,
    DataChanged,
}

pub struct FileWatcherService {
    _watcher: RecommendedWatcher,
}

impl FileWatcherService {
    pub fn new<P1, P2>(
        config_path: P1,
        data_path: P2,
    ) -> Result<(Self, mpsc::Receiver<FileChangeEvent>)>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let (tx, rx) = mpsc::channel(100);

        let config_path = std::fs::canonicalize(config_path)?;
        let data_path = std::fs::canonicalize(data_path)?;

        let tx_clone = tx.clone();
        let runtime_handle = tokio::runtime::Handle::current();

        let watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    for path in &event.paths {
                        let tx = tx_clone.clone();

                        let change_event = if path.starts_with(&config_path) {
                            Some(FileChangeEvent::ConfigChanged)
                        } else if path.starts_with(&data_path) {
                            Some(FileChangeEvent::DataChanged)
                        } else {
                            None
                        };

                        if let Some(evt) = change_event {
                            runtime_handle.spawn(async move {
                                if let Err(e) = tx.send(evt).await {
                                    warn!("Failed to send file change event: {}", e);
                                }
                            });
                        }
                    }
                }
                Err(e) => error!("File watcher error: {:?}", e),
            },
            Config::default(),
        )?;

        let service = FileWatcherService { _watcher: watcher };

        Ok((service, rx))
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        info!("Watching path: {}", path.as_ref().display());
        self._watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)?;
        Ok(())
    }
}
