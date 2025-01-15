use crate::env::Env;
use log::debug;
use std::{error::Error, process::Command, process::Stdio, str};

#[derive(Debug)]
pub struct Cri {}

impl Cri {
    /// Get an image's digest from its tag using docker
    fn get_digest_from_tag(image_tag: &str) -> Result<String, Box<dyn Error>> {
        let pull_status = Command::new("docker")
            .args(["pull", image_tag])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !pull_status.success() {
            return Err(format!(
                "{}(cri): failed to pull docker image with tag: {image_tag}",
                Env::SYS_NAME
            )
            .into());
        }

        // Inspect the image to get its digest
        let output = Command::new("docker")
            .args(["inspect", "--format", "{{.Id}}", image_tag])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "{}(cri): failed to inspect Docker image with tag: {image_tag}: error: {}",
                Env::SYS_NAME,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Extract and return the digest
        let digest = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(digest)
    }

    /// Remove an image from the CRI's image store. Note that removing the
    /// image from tag is, sometimes, unreliable, so we remove it by specifying
    /// its digest.
    pub fn remove_image(image_tag: String) {
        let image_digest = Self::get_digest_from_tag(&image_tag).unwrap();
        debug!(
            "{}(cri): removing image {image_tag} (sha: {image_digest})",
            Env::SYS_NAME
        );

        let output = Command::new("sudo")
            .args([
                "crictl",
                "--runtime-endpoint",
                "unix:///run/containerd/containerd.sock",
                "rmi",
                &image_digest,
            ])
            .output()
            .expect("sc2-exp(cri): error removing image");

        match output.status.code() {
            Some(0) => {}
            Some(code) => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(cri): failed to get stderr");
                panic!(
                    "{}(cri): cri-rmi exited with error (code: {code}): {stderr}",
                    Env::SYS_NAME
                );
            }
            None => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(cri): failed to get stderr");
                panic!("{}(cri): cri-rmi command failed: {stderr}", Env::SYS_NAME);
            }
        };
    }
}
