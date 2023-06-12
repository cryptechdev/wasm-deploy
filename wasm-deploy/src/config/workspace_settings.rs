use anyhow::bail;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

lazy_static! {
    pub static ref WORKSPACE_SETTINGS: RwLock<Option<Arc<WorkspaceSettings>>> = RwLock::new(None);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// must be an absolute path
    pub(crate) workspace_root: PathBuf,
    /// absolute or relative to workspace root
    pub(crate) config_path: PathBuf,
    /// absolute or relative to workspace root
    pub(crate) target_dir: PathBuf,
    /// absolute or relative to workspace root
    pub(crate) deployment_dir: PathBuf,
    /// absolute or relative to workspace root
    pub(crate) artifacts_dir: PathBuf,
}

impl WorkspaceSettings {
    pub fn new<T: ?Sized + AsRef<OsStr>>(workspace_root: &T) -> anyhow::Result<Self> {
        let workspace_root = PathBuf::from(workspace_root);
        if workspace_root.is_relative() {
            bail!("workspace root must be an absolute path")
        } else if workspace_root.is_file() {
            bail!("workspace root must be a directory")
        }
        Ok(Self {
            workspace_root: workspace_root.clone(),
            config_path: workspace_root.join(".wasm-deploy/config.json"),
            target_dir: workspace_root.join("target"),
            deployment_dir: workspace_root.join("deployment"),
            artifacts_dir: workspace_root.join("artifacts"),
        })
    }

    /// Default path is `.wasm-deploy/config.json`
    pub fn set_config_path<T: ?Sized + AsRef<OsStr>>(
        mut self,
        config_path: &T,
    ) -> anyhow::Result<Self> {
        let mut config_path = PathBuf::from(config_path);
        if config_path.is_dir() {
            config_path = config_path.join("config.json");
        }
        self.config_path = config_path;
        Ok(self)
    }

    /// Default path is `target`
    pub fn set_build_dir<T: ?Sized + AsRef<OsStr>>(
        mut self,
        target_dir: &T,
    ) -> anyhow::Result<Self> {
        let target_dir = PathBuf::from(target_dir);
        if !target_dir.is_dir() {
            bail!("target dir must be a directory")
        }
        self.target_dir = target_dir;
        Ok(self)
    }

    /// Default path is `deployment`
    pub fn set_deployment_dir<T: ?Sized + AsRef<OsStr>>(
        mut self,
        deployment_dir: &T,
    ) -> anyhow::Result<Self> {
        let deployment_dir = PathBuf::from(deployment_dir);
        if !deployment_dir.is_dir() {
            bail!("deployment dir must be a directory")
        }
        self.deployment_dir = deployment_dir;
        Ok(self)
    }

    /// Default path is `artifacts`
    pub fn set_artifacts_dir<T: ?Sized + AsRef<OsStr>>(
        mut self,
        artifacts_dir: &T,
    ) -> anyhow::Result<Self> {
        let artifacts_dir = PathBuf::from(artifacts_dir);
        if !artifacts_dir.is_dir() {
            bail!("artifacts dir must be a directory")
        }
        self.artifacts_dir = artifacts_dir;
        Ok(self)
    }
}
