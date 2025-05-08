use std::{env, path::PathBuf};

pub struct Env {}

impl Env {
    pub const CONTAINER_REGISTRY_URL: &'static str = "sc2cr.io/applications";
    pub const KNATIVE_SIDECAR_IMAGE_NAME: &'static str = "sc2cr.io/system/knative-sidecar";
    pub const K8S_NAMESPACE: &'static str = "sc2";
    pub const SYS_NAME: &'static str = "sc2-exp";

    pub fn proj_root() -> PathBuf {
        env!("CARGO_MANIFEST_DIR").into()
    }

    pub fn results_root() -> PathBuf {
        let mut path = Self::proj_root();
        path.push("results");
        path
    }

    pub fn apps_root() -> PathBuf {
        let mut path = Self::proj_root();
        path.push("..");
        path.push("applications");
        path
    }
}
