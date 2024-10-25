use crate::env::Env;
use log::debug;
use std::{env, process::Command, thread, time};

#[derive(Debug)]
pub struct K8s {}

impl K8s {
    fn get_kubectl_cmd() -> String {
        // For the moment, we literally run the `kubectl` command installed
        // as part of `coco-serverless`. We may change this in the future
        match env::var("COCO_SOURCE") {
            Ok(value) => format!("{value}/bin/kubectl"),
            Err(_) => panic!("invrs(eval): failed to read COCO_SOURCE env. var"),
        }
    }

    pub fn run_kubectl_cmd(cmd: &str) -> String {
        debug!("{}(k8s): running kubectl command: {cmd}", Env::SYS_NAME);
        let args: Vec<&str> = cmd.split_whitespace().collect();

        let output = Command::new(Self::get_kubectl_cmd())
            .args(&args[0..])
            .output()
            .expect("sc2-eval(k8s): failed to execute kubectl command");

        String::from_utf8(output.stdout)
            .expect("sc2-eval(k8s): failed to convert kube command output to string")
    }

    pub fn wait_for_pods(namespace: &str, label: &str, num_expected: usize) {
        loop {
            thread::sleep(time::Duration::from_secs(2));

            let output = Self::run_kubectl_cmd(&format!("-n {namespace} get pods -l {label} -o jsonpath='{{..status.conditions[?(@.type==\"Ready\")].status}}'"));
            let values: Vec<&str> = output.split_whitespace().collect();

            debug!(
                "{}(k8s): waiting for {num_expected} pods (label: {label}) to be ready...",
                Env::SYS_NAME
            );
            if values.len() != num_expected {
                debug!(
                    "{}(k8s): not enough pods: {} != {num_expected}",
                    Env::SYS_NAME,
                    values.len()
                );
                continue;
            }

            if !values.iter().all(|&item| item == "'True'") {
                debug!("{}(eval): not enough pods in 'Ready' state", Env::SYS_NAME);
                continue;
            }

            break;
        }
    }
}
