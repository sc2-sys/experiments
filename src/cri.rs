use crate::env::Env;
use log::debug;
use std::{error::Error, process::Command, process::Stdio, str};

#[derive(Debug)]
pub struct Cri {}

impl Cri {
    /// Get an image's digest from its tag using `crictl images`. Note that
    /// the method also works if we provide a prefix for the image
    fn get_digest_from_tag(
        image_tag: &str,
        tolerate_missing: bool,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        // Get the list of images in crictl
        let image_ids_output = Command::new("sudo")
            .arg("crictl")
            .arg("--runtime-endpoint")
            .arg("unix:///run/containerd/containerd.sock")
            .arg("images")
            .stdout(Stdio::piped())
            .output()
            .expect("sc2(cri): failed to execute crictl images command");

        if !image_ids_output.status.success() {
            return Err(format!(
                "{}(cri): failed to get crictl images: error: {}",
                Env::SYS_NAME,
                String::from_utf8_lossy(&image_ids_output.stderr)
            )
            .into());
        }

        // We deliberately only filter by image name, and not by tag, as
        // somtimes the tag appears as none, this means that we may sometimes
        // remove more images than needed, but we are ok with that
        let image_name = image_tag.split(':').next().unwrap();
        let image_ids = String::from_utf8_lossy(&image_ids_output.stdout);
        let filtered_image_ids: Vec<String> = image_ids
            .lines()
            .filter(|line| line.contains(image_name))
            .filter_map(|line| line.split_whitespace().nth(2))
            .map(|s| s.to_string())
            .collect();

        if filtered_image_ids.is_empty() && !tolerate_missing {
            if !tolerate_missing {
                return Err(format!(
                    "{}(cri): did not find any matching image ids for image: {image_tag}",
                    Env::SYS_NAME,
                )
                .into());
            } else {
                return Ok(vec![]);
            }
        }

        // Extract and return the digest
        Ok(filtered_image_ids.clone())
    }

    /// Remove an image from the CRI's image store. Note that removing the
    /// image from tag is, sometimes, unreliable, so we remove it by specifying
    /// its digest. Furthermore, tags do not always appear in crictl images,
    /// so we remove all tags of the same image. This method can be called
    /// either with the full image tag, or just a prefix. In the prefix case
    /// we will remove all images with the prefix.
    pub fn remove_image(image_tag: String, tolerate_missing: bool) {
        let image_digests = Self::get_digest_from_tag(&image_tag, tolerate_missing).unwrap();
        for image_digest in &image_digests {
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
                    image_digest,
                ])
                .output()
                .expect("sc2-exp(cri): error removing image");

            match output.status.code() {
                Some(0) => {}
                Some(code) => {
                    let stderr = str::from_utf8(&output.stderr)
                        .unwrap_or("sc2-exp(cri): failed to get stderr");
                    panic!(
                        "{}(cri): cri-rmi exited with error (code: {code}): {stderr}",
                        Env::SYS_NAME
                    );
                }
                None => {
                    let stderr = str::from_utf8(&output.stderr)
                        .unwrap_or("sc2-exp(cri): failed to get stderr");
                    panic!("{}(cri): cri-rmi command failed: {stderr}", Env::SYS_NAME);
                }
            };
        }
    }
}
