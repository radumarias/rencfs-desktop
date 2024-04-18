use std::fs;
use std::fs::{File, OpenOptions};
use std::sync::mpsc::Receiver;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::{Child, Command};
use tonic::{Response, Status};
use tracing::{debug, error, info, warn};
use crate::app_details::{APPLICATION, ORGANIZATION, QUALIFIER};

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum VaultHandlerError {
    #[error("cannot lock vault")]
    CannotLockVault,
}
pub struct VaultHandler {
    id: u32,
    child: Option<Child>,
}

impl VaultHandler {
    pub fn new(id: u32) -> Self {
        Self { id, child: None }
    }

    pub async fn lock(&mut self) -> Result<(), VaultHandlerError> {
        info!("VaultHandler {} received lock request", self.id);

        if self.child.is_none() {
            info!("VaultHandler {} already locked", self.id);
            return Ok(());
        }
        info!("VaultHandler {} killing child process to lock the vault", self.id);
        if let Err(err) = self.child.take().unwrap().kill().await {
            error!("Error killing child process: {:?}", err);
            return Err(VaultHandlerError::CannotLockVault.into());
        }

        Ok(())
    }

    pub async fn unlock(&mut self) -> Result<(), VaultHandlerError> {
        info!("VaultHandler {} received unlock request", self.id);

        if self.child.is_some() {
            info!("VaultHandler {} already unlocked", self.id);
            return Ok(());
        }

        let base_data_dir = if let Some(proj_dirs) = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION) {
            proj_dirs.data_local_dir().to_path_buf()
        } else { panic!("Cannot get project directories"); };
        // create logs files
        let stdout = OpenOptions::new().append(true).create(true).open(base_data_dir.join("logs").join(format!("vault_{}.log.out", self.id))).expect("Cannot create stdout file");
        let stderr = OpenOptions::new().append(true).create(true).open(base_data_dir.join("logs").join(format!("vault_{}.log.err", self.id))).expect("Cannot create stderr file");

        // spawn new process
        let child = Command::new("/home/gnome/dev/RustroverProjects/encrypted_fs/target/debug/encrypted_fs")
            .env("ENCRYPTED_FS_PASSWORD", "pass-42")
            .stdout(stdout)
            .stderr(stderr)
            .arg("--mount-point")
            .arg("/home/gnome/encrypted_fs")
            .arg("--data-dir")
            .arg("/home/gnome/encrypted_fs_data")
            .arg("--umount-on-start")
            .spawn()
            .expect("Failed to start process");
        self.child = Some(child);

        Ok(())
    }

    pub(crate) fn change_mount_point(&mut self) -> Result<(), VaultHandlerError> {
        todo!()
    }
}
