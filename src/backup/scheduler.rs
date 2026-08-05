//! # BackupScheduler — Periodic and On-Demand Backup Task
//!
//! Runs a background task that triggers full and incremental backups on fixed
//! intervals and accepts on-demand commands over a channel. Backups are
//! executed through a shared [`BackupManager`] guarded by a mutex.
//!
//! ```text
//! BackupScheduler::new(config, manager, data_dir)
//!   |
//!   +-> spawns run_scheduler task
//!   |     +-> tokio::select!:
//!   |     |     full_interval.tick() -> create_full_backup
//!   |     |     incr_interval.tick() -> create_incremental_backup
//!   |     |     rx.recv()            -> FullBackup / IncrementalBackup / Shutdown
//!   |
//!   +-> trigger_full_backup / trigger_incremental_backup / shutdown
//! ```
//!
//! When `config.enabled` is false the task logs and exits immediately.

use crate::backup::BackupManager;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Commands accepted by the scheduler loop.
#[derive(Debug, Clone)]
pub enum BackupCommand {
    /// Run an on-demand full backup of the given data directory.
    FullBackup(String),
    /// Run an on-demand incremental backup of the given data directory.
    IncrementalBackup(String),
    /// Stop the scheduler loop.
    Shutdown,
}

/// Handle to the background backup scheduler task.
pub struct BackupScheduler {
    command_tx: mpsc::UnboundedSender<BackupCommand>,
    data_dir: String,
}

impl BackupScheduler {
    /// Spawns the scheduler task and returns a handle that can enqueue
    /// commands.
    pub fn new(
        config: crate::backup::BackupScheduleConfig,
        backup_manager: Arc<Mutex<BackupManager>>,
        data_dir: String,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();
        let data_dir_clone = data_dir.clone();

        tokio::spawn(async move {
            Self::run_scheduler(config, backup_manager, rx, tx_clone, data_dir_clone).await;
        });

        Self {
            command_tx: tx,
            data_dir,
        }
    }

    async fn run_scheduler(
        config: crate::backup::BackupScheduleConfig,
        backup_manager: Arc<Mutex<BackupManager>>,
        mut rx: mpsc::UnboundedReceiver<BackupCommand>,
        _tx: mpsc::UnboundedSender<BackupCommand>,
        data_dir: String,
    ) {
        if !config.enabled {
            tracing::info!("Backup scheduler is disabled");
            return;
        }

        let full_interval_secs = config.full_backup_interval_secs;
        let incr_interval_secs = config.incremental_interval_secs;

        let mut full_interval =
            tokio::time::interval(std::time::Duration::from_secs(full_interval_secs));
        let mut incr_interval =
            tokio::time::interval(std::time::Duration::from_secs(incr_interval_secs));

        tracing::info!(
            "Backup scheduler started (full every {}s, incremental every {}s)",
            full_interval_secs,
            incr_interval_secs
        );

        loop {
            tokio::select! {
                _ = full_interval.tick() => {
                    if let Ok(mut manager) = backup_manager.lock() {
                        match manager.create_full_backup(&data_dir) {
                            Ok(manifest) => {
                                tracing::info!("Scheduled full backup completed: {}", manifest.id);
                            }
                            Err(e) => {
                                tracing::error!("Scheduled full backup failed: {}", e);
                            }
                        }
                    }
                }
                _ = incr_interval.tick() => {
                    if let Ok(mut manager) = backup_manager.lock() {
                        match manager.create_incremental_backup(&data_dir) {
                            Ok(manifest) => {
                                tracing::info!("Scheduled incremental backup completed: {}", manifest.id);
                            }
                            Err(e) => {
                                tracing::error!("Scheduled incremental backup failed: {}", e);
                            }
                        }
                    }
                }
                Some(cmd) = rx.recv() => {
                    match cmd {
                        BackupCommand::Shutdown => {
                            tracing::info!("Backup scheduler shutting down");
                            break;
                        }
                        BackupCommand::FullBackup(data_dir) => {
                            if let Ok(mut manager) = backup_manager.lock() {
                                if let Err(e) = manager.create_full_backup(&data_dir) {
                                    tracing::error!("On-demand full backup failed: {}", e);
                                }
                            }
                        }
                        BackupCommand::IncrementalBackup(data_dir) => {
                            if let Ok(mut manager) = backup_manager.lock() {
                                if let Err(e) = manager.create_incremental_backup(&data_dir) {
                                    tracing::error!("On-demand incremental backup failed: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Enqueues an on-demand full backup of `data_dir`.
    pub fn trigger_full_backup(&self, data_dir: String) -> crate::Result<()> {
        self.command_tx
            .send(BackupCommand::FullBackup(data_dir))
            .map_err(|e| {
                crate::Error::ValidationError(format!("Failed to send backup command: {}", e))
            })
    }

    /// Enqueues an on-demand incremental backup of `data_dir`.
    pub fn trigger_incremental_backup(&self, data_dir: String) -> crate::Result<()> {
        self.command_tx
            .send(BackupCommand::IncrementalBackup(data_dir))
            .map_err(|e| {
                crate::Error::ValidationError(format!("Failed to send backup command: {}", e))
            })
    }

    /// Requests a clean shutdown of the scheduler loop.
    pub fn shutdown(&self) -> crate::Result<()> {
        self.command_tx.send(BackupCommand::Shutdown).map_err(|e| {
            crate::Error::ValidationError(format!("Failed to send shutdown command: {}", e))
        })
    }

    /// Returns the data directory the scheduler was started with.
    pub fn data_dir(&self) -> &str {
        &self.data_dir
    }
}

impl Drop for BackupScheduler {
    fn drop(&mut self) {
        let _ = self.command_tx.send(BackupCommand::Shutdown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::BackupConfig;

    #[tokio::test]
    async fn test_backup_scheduler_creation() {
        let config = crate::backup::BackupScheduleConfig::default();
        let manager = Arc::new(Mutex::new(BackupManager::new(BackupConfig::default())));
        let scheduler = BackupScheduler::new(config, manager, "/data".to_string());
        assert!(scheduler.shutdown().is_ok());
    }

    #[tokio::test]
    async fn test_backup_scheduler_trigger() {
        let config = crate::backup::BackupScheduleConfig::default();
        let manager = Arc::new(Mutex::new(BackupManager::new(BackupConfig::default())));
        let scheduler = BackupScheduler::new(config, manager, "/data".to_string());
        assert!(scheduler.trigger_full_backup("/data".to_string()).is_ok());
        assert!(scheduler
            .trigger_incremental_backup("/data".to_string())
            .is_ok());
        assert!(scheduler.shutdown().is_ok());
    }
}
