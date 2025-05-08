use crate::{cri::Cri, env::Env};
use log::debug;
use nix::unistd::{Gid, Uid};
use std::{path::PathBuf, process::Command, thread, time};

/// Wrapper around the sc2-sys/deploy python virtual environment. Ideally,
/// the latter would progressively move towards a Rust crate that we can
/// import here. But that seems a bit far fetched.
#[derive(Debug)]
pub struct Deploy {}

impl Deploy {
    fn deploy_root() -> PathBuf {
        let mut path = Env::proj_root();
        path.push("..");
        path.push("deploy");
        path
    }

    fn run_inv_command(inv_cmd: &str) -> String {
        let command_str = format!("source ./bin/workon.sh && inv {inv_cmd}");

        // Execute the command using bash -c
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command_str)
            .current_dir(Self::deploy_root())
            .output()
            .expect("sc2-exp(deploy): failed to spawn deploy command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("sc2-exp(deploy): error executing inv command: {inv_cmd}: {stderr}");
        }

        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn wait_for_snapshotter_metadata_to_be_gced(snapshotter: &str) {
        let mut bbolt_path = Self::deploy_root();
        bbolt_path.push("bin");
        bbolt_path.push("bbolt");
        let db_path = "/var/lib/containerd/io.containerd.metadata.v1.bolt/meta.db";
        let tmp_db_path = "/tmp/containerd_meta_copy.db";

        loop {
            // Copy DB to temporary file that we can modify
            let output = Command::new("sudo")
                .arg("cp")
                .arg(db_path)
                .arg(tmp_db_path)
                .output()
                .expect("sc2-exp(deploy): failed to spawn cp command");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("sc2-exp(deploy): error executing cp command: {stderr}");
            }

            let output = Command::new("sudo")
                .arg("chown")
                .arg(format!("{}:{}", Uid::current(), Gid::current()))
                .arg(tmp_db_path)
                .output()
                .expect("sc2-exp(deploy): failed to spawn chown command");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("sc2-exp(deploy): error executing chown command: {stderr}");
            }

            // Now query the snapshots present in the DB's metadata
            let output = Command::new(bbolt_path.clone())
                .arg("keys")
                .arg(tmp_db_path)
                .arg("v1")
                .arg("k8s.io")
                .arg("snapshots")
                .arg(snapshotter)
                .output()
                .expect("sc2-exp(deploy): failed to spawn bbolt command");

            let raw_stdout = String::from_utf8_lossy(&output.stdout);
            let stdout = raw_stdout.trim_end_matches("\n");
            if !output.status.success() {
                if output.status.code() == Some(1) && matches!(stdout, "bucket not found") {
                    // This is a benign error if the snapshotter has never been
                    // used before
                    return;
                }

                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("sc2-exp(deploy): error executing bbolt command: {stderr}");
            }

            if stdout.is_empty() {
                // We are done, no snapshot metadata
                return;
            }

            let total_newlines = stdout.matches('\n').count();
            debug!("sc2-exp(deploy): purge-snapshotters: {snapshotter} has {total_newlines} snapshot's metadata");
            thread::sleep(time::Duration::from_secs(2));
        }
    }

    pub fn purge_snapshotters() {
        // This command relies on filtering the stdout of the process, so,
        // unfortunately, we cannot directly rely on the invoke task here.
        // We manually copy the implementation here from:
        // https://github.com/sc2-sys/deploy/tree/main/tasks/nydus_snapshotter.py
        //
        // Self::run_inv_command("nydus-snapshotter.purge");

        for snapshotter in ["nydus", "nydus-hs"] {
            let output = Command::new("sudo")
                .arg("rm")
                .arg("-rf")
                .arg(format!("/var/lib/containerd-{snapshotter}"))
                .output()
                .expect("sc2-exp(deploy): failed to spawn rm command");

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                panic!("sc2-exp(deploy): error executing rm command: {stderr}");
            }
        }

        Cri::remove_image("registry.k8s.io/pause".to_string(), true);
        Cri::remove_image(Env::CONTAINER_REGISTRY_URL.to_string(), true);
        Cri::remove_image(Env::KNATIVE_SIDECAR_IMAGE_NAME.to_string(), true);

        // Restart snapshotter
        let output = Command::new("sudo")
            .arg("service")
            .arg("nydus-snapshotter")
            .arg("restart")
            .output()
            .expect("sc2-exp(deploy): failed to spawn service command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("sc2-exp(deploy): error executing service command: {stderr}");
        }

        // Wait for the metadata to be cleared from containerd's metadata db
        for snapshotter in ["nydus", "nydus-hs"] {
            Self::wait_for_snapshotter_metadata_to_be_gced(snapshotter);
        }

        thread::sleep(time::Duration::from_secs(2));
    }

    pub fn set_snapshotter_mode(mode: &str) {
        let inv_cmd = format!("nydus-snapshotter.set-mode {mode}");
        Self::run_inv_command(inv_cmd.as_str());

        thread::sleep(time::Duration::from_secs(2));
    }
}
