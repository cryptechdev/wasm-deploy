use std::{ffi::OsStr, path::PathBuf};

pub struct WorkspaceSettings {
    /// must be an absolute path
    pub workspace_root: PathBuf,
    /// absolute or relative to workspace root
    pub config_path: PathBuf,
    /// absolute or relative to workspace root
    pub target_dir: PathBuf,
    /// absolute or relative to workspace root
    pub deployment_dir: PathBuf,
    /// absolute or relative to workspace root
    pub artifacts_dir: PathBuf,
}

impl WorkspaceSettings {
    pub fn new<T: ?Sized + AsRef<OsStr>>(workspace_root: &T) -> Self {
        let workspace_root = PathBuf::from(workspace_root);
        if workspace_root.is_relative() {
            // TODO
            panic!("Workspace root must be an absolute path");
        }
        Self {
            workspace_root: workspace_root.clone(),
            config_path: workspace_root.join(".wasm-deploy/config.json"),
            target_dir: workspace_root.join("target"),
            deployment_dir: workspace_root.join("deployment"),
            artifacts_dir: workspace_root.join("artifacts"),
        }
    }

    pub fn set_config_path<T: ?Sized + AsRef<OsStr>>(mut self, config_path: &T) -> Self {
        self.config_path = PathBuf::from(config_path);
        self
    }

    pub fn set_build_dir<T: ?Sized + AsRef<OsStr>>(mut self, build_dir: &T) -> Self {
        self.target_dir = PathBuf::from(build_dir);
        self
    }

    pub fn set_deployment_dir<T: ?Sized + AsRef<OsStr>>(mut self, deployment_dir: &T) -> Self {
        self.deployment_dir = PathBuf::from(deployment_dir);
        self
    }

    pub fn set_artifacts_dir<T: ?Sized + AsRef<OsStr>>(mut self, artifacts_dir: &T) -> Self {
        self.artifacts_dir = PathBuf::from(artifacts_dir);
        self
    }
}
