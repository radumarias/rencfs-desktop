use std::process;
use std::fs::OpenOptions;
use std::sync::Arc;

use diesel::{QueryResult, SqliteConnection};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessStatus, System};
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::dao::VaultDao;
use crate::get_logs_dir;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
pub enum VaultHandlerError {
    #[error("cannot lock vault")]
    CannotLockVault,
    #[error("cannot unlock vault")]
    CannotUnlockVault,
    #[error("cannot change mount point")]
    CannotChangeMountPoint,
    #[error("cannot change data dir")]
    CannotChangeDataDir,
}

pub struct VaultHandler {
    id: u32,
    child: Option<Child>,
    db_conn: Arc<Mutex<SqliteConnection>>,
}

impl VaultHandler {
    pub fn new(id: u32, db_conn: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { id, child: None, db_conn }
    }

    pub async fn lock(&mut self, mount_point: Option<String>) -> Result<(), VaultHandlerError> {
        info!("VaultHandler {} received lock request", self.id);

        {
            let mut guard = self.db_conn.lock().await;
            let mut dao = VaultDao::new(&mut *guard);
            match self.db_update_locked(true, &mut dao).await {
                Ok(_) => {}
                Err(err) => {
                    error!("Cannot update vault state {}", err);
                    return Err(VaultHandlerError::CannotLockVault.into());
                }
            }
        }

        if self.child.is_none() {
            info!("VaultHandler {} already locked", self.id);
            return Ok(());
        }
        info!("VaultHandler {} killing child process to lock the vault", self.id);
        if let Err(err) = self.child.take().unwrap().kill().await {
            error!("Error killing child process: {:?}", err);
            return Err(VaultHandlerError::CannotLockVault.into());
        }

        // for some reason of we use 'kill' method the child process doesn't receive the SIGKILL signal
        // for that case we use `umount` command
        // TODO: umount for windows
        if cfg!(any(linux, unix, macos, freebsd, openbsd, netbsd)) {
            let mount_point = if let Some(mount_point) = mount_point {
                mount_point
            } else {
                let mut guard = self.db_conn.lock().await;
                let mut dao = VaultDao::new(&mut *guard);
                match dao.get(self.id as i32) {
                    Ok(vault) => vault.mount_point,
                    Err(err) => {
                        error!("Cannot get vault {}", err);
                        return Err(VaultHandlerError::CannotLockVault.into());
                    }
                }
            };
            if let Err(_) = process::Command::new("umount")
                .arg(&mount_point)
                .output() {
                error!("Cannot umount {}", mount_point);
                return Err(VaultHandlerError::CannotLockVault.into());
            }
        }

        Ok(())
    }

    pub async fn unlock(&mut self) -> Result<(), VaultHandlerError> {
        info!("VaultHandler {} received unlock request", self.id);

        if self.child.is_some() {
            info!("VaultHandler {} already unlocked", self.id);
            return Ok(());
        }

        // create logs files
        let logs_dir = get_logs_dir();
        let stdout = OpenOptions::new().append(true).create(true).open(logs_dir.join(format!("vault_{}.out", self.id))).expect("Cannot create stdout file");
        let stderr = OpenOptions::new().append(true).create(true).open(logs_dir.join(format!("vault_{}.err", self.id))).expect("Cannot create stderr file");

        let vault = {
            let mut guard = self.db_conn.lock().await;
            let mut dao = VaultDao::new(&mut *guard);
            match dao.get(self.id as i32) {
                Ok(vault) => vault,
                Err(err) => {
                    error!("Cannot get vault {}", err);
                    return Err(VaultHandlerError::CannotLockVault.into());
                }
            }
        };

        // spawn new process
        let child = Command::new("/home/gnome/dev/RustroverProjects/encrypted_fs/target/debug/encrypted_fs")
            // TODO get pass from keystore
            .env("ENCRYPTED_FS_PASSWORD", "pass-42")
            .stdout(stdout)
            .stderr(stderr)
            .arg("--mount-point")
            .arg(&vault.mount_point)
            .arg("--data-dir")
            .arg(&vault.data_dir)
            .arg("--umount-on-start")
            .spawn();
        let child = match child {
            Ok(child) => child,
            Err(err) => {
                error!("Cannot start process {}", err);
                return Err(VaultHandlerError::CannotUnlockVault.into());
            }
        };

        // wait few second and check if it started correctly
        tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;
        if child.id().is_none() {
            return Err(VaultHandlerError::CannotUnlockVault.into());
        }
        let mut sys = System::new();
        sys.refresh_processes();
        let mut is_defunct = false;
        match sys.process(Pid::from_u32(child.id().unwrap())) {
            Some(process) => {
                println!("{:?}", process.status());
                if process.status() == ProcessStatus::Dead ||
                    process.status() == ProcessStatus::Zombie ||
                    process.status() == ProcessStatus::Stop {
                    warn!("Process is dead or zombie, killing it");
                    is_defunct = true;
                } else {
                    // try to check if it's defunct with ps command
                    // TODO: ps for windows
                    if cfg!(any(linux, unix, macos, freebsd, openbsd, netbsd)) {
                        let out = Command::new("ps")
                            .arg("-f")
                            .arg(child.id().unwrap().to_string())
                            .output().await
                            .expect("Cannot run ps command");
                        String::from_utf8(out.stdout).unwrap().lines().for_each(|line| {
                            if line.contains("defunct") {
                                warn!("Process is defunct, killing it");
                                is_defunct = true;
                            }
                        });
                    }
                }
            }
            None => return Err(VaultHandlerError::CannotUnlockVault.into())
        }
        if is_defunct {
            // TODO: kill for windows
            if cfg!(any(linux, unix, macos, freebsd, openbsd, netbsd)) {
                process::Command::new("kill")
                    .arg(child.id().unwrap().to_string())
                    .output()
                    .expect("Cannot kill process");
            }
            return Err(VaultHandlerError::CannotUnlockVault.into());
        }

        self.child = Some(child);

        let mut guard = self.db_conn.lock().await;
        let mut dao = VaultDao::new(&mut *guard);
        match self.db_update_locked(false, &mut dao).await {
            Ok(_) => {}
            Err(err) => {
                error!("Cannot update vault state {}", err);
                return Err(VaultHandlerError::CannotUnlockVault.into());
            }
        }

        Ok(())
    }

    pub async fn change_mount_point(&mut self, old_mount_point: String) -> Result<(), VaultHandlerError> {
        let unlocked = self.child.is_some();
        if unlocked {
            self.lock(Some(old_mount_point)).await?;
            self.unlock().await?;
        }

        Ok(())
    }

    pub async fn change_data_dir(&mut self, old_data_dir: String) -> Result<(), VaultHandlerError> {
        let unlocked = self.child.is_some();
        if unlocked {
            let mount_point = {
                let mut guard = self.db_conn.lock().await;
                let mut dao = VaultDao::new(&mut *guard);
                match dao.get(self.id as i32) {
                    Ok(vault) => vault.mount_point,
                    Err(err) => {
                        error!("Cannot get vault {}", err);
                        return Err(VaultHandlerError::CannotChangeDataDir.into());
                    }
                }
            };
            self.lock(Some(mount_point)).await?;
            // TODO: move content to new data dir
            self.unlock().await?;
        }

        Ok(())
    }

    async fn db_update_locked(&self, state: bool, dao: &mut VaultDao<'_>) -> QueryResult<()> {
        use crate::schema::vaults::dsl::locked;
        use diesel::ExpressionMethods;

        dao.update(self.id as i32, locked.eq(if state { 1 } else { 0 }))
    }
}
