use std::{env, path::PathBuf};

pub struct Env {}

impl Env {
    pub const CONTAINER_REGISTRY_URL: &'static str = "ghcr.io/coco-serverless";
    pub const K8S_NAMESPACE: &'static str = "sc2";
    pub const SYS_NAME: &'static str = "sc2-exp";

    pub fn proj_root() -> PathBuf {
        env::current_dir().expect("sc2-exp(env): failed to get current directory")
    }

    pub fn results_root() -> PathBuf {
        let mut path = Self::proj_root();
        path.push("results");
        path
    }

    pub fn apps_root() -> PathBuf {
        let mut path = Self::proj_root();
        path.push("..");
        path.push("apps");
        path
    }
}
