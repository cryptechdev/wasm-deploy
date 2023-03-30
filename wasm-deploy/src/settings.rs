use std::{ffi::OsStr, path::PathBuf};

use anyhow::bail;

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

    pub fn set_config_path<T: ?Sized + AsRef<OsStr>>(
        mut self,
        config_path: &T,
    ) -> anyhow::Result<Self> {
        let config_path = PathBuf::from(config_path);
        if !config_path.is_file() {
            bail!("config path must be a file")
        }
        self.config_path = config_path;
        Ok(self)
    }

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
