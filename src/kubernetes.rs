use crate::env::Env;
use log::debug;
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    str, thread, time,
};

#[derive(Debug)]
pub struct K8s {}

impl K8s {
    fn get_kubectl_cmd() -> String {
        // For the moment, we literally run the `kubectl` command installed
        // as part of `coco-serverless`. We may change this in the future
        match env::var("SC2_DEPLOY_SOURCE") {
            Ok(value) => format!("{value}/bin/kubectl"),
            Err(_) => panic!("invrs(eval): failed to read SC2_DEPLOY_SOURCE env. var"),
        }
    }

    pub fn run_kubectl_cmd(cmd: &str) -> String {
        debug!("{}(k8s): running kubectl command: {cmd}", Env::SYS_NAME);
        let args: Vec<&str> = cmd.split_whitespace().collect();

        let output = Command::new(Self::get_kubectl_cmd())
            .args(&args[0..])
            .output()
            .expect("sc2-eval(k8s): failed to spawn kubectl command");

        match output.status.code() {
            Some(0) => {}
            Some(code) => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!(
                    "{}(k8s): kubectl exited with error (code: {code}): {stderr}",
                    Env::SYS_NAME
                );
            }
            None => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!("{}(k8s): kubectl command failed: {stderr}", Env::SYS_NAME);
            }
        };

        String::from_utf8(output.stdout)
            .expect("sc2-eval(k8s): failed to convert kube command output to string")
            .trim()
            .to_string()
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

    fn template_yaml(yaml_path: &PathBuf, env_vars: &BTreeMap<&str, String>) -> String {
        debug!(
            "{}(k8s): templating yaml file from: {yaml_path:?}",
            Env::SYS_NAME
        );
        let yaml_content =
            fs::read_to_string(yaml_path).expect("sc2-exp(k8s): failed to read yaml");

        // Use envsubst to substitute environment variables in the YAML
        let mut envsubst_cmd = Command::new("envsubst");
        for (key, value) in env_vars {
            envsubst_cmd.env(key, value);
        }

        let mut envsubst = envsubst_cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("sc2-exp(k8s): failed to start envsubst");

        envsubst
            .stdin
            .as_mut()
            .expect("sc2-exp(k8s): failed to open stdin for envsubst")
            .write_all(yaml_content.as_bytes())
            .expect("sc2-exp(k8s): failed to write to envsubst");

        // Collect the output (substituted YAML)
        let result = envsubst
            .wait_with_output()
            .expect("sc2-exp(k8s): failed to read envsubst result");

        let result_str = String::from_utf8(result.stdout)
            .expect("sc2-exp(k8s): failed to convert envsubst output to string");

        // If running the `runc` baseline, we must drop the runtime class line
        // altogether
        if env_vars.get("RUNTIME_CLASS_NAME").unwrap() == "runc" {
            return result_str
                .lines()
                .filter(|line| !line.trim_start().starts_with("runtimeClassName"))
                .collect::<Vec<_>>()
                .join("\n");
        }

        result_str
    }

    /// Returns the URL of a knative service without the http prefix. This
    /// returns something along the lines of service-name.namespace.ip.sslip.io
    fn get_knative_service_url(service_name: &str) -> String {
        // First, wait until the service is ready
        loop {
            let output = Self::run_kubectl_cmd(
                &format!("-n {} get ksvc -o jsonpath={{.items[?(@.metadata.name==\"{service_name}\")].metadata.name}}", Env::K8S_NAMESPACE)
            );

            debug!(
                "{}: waiting for Knative serice to be ready '{service_name}': out: {output}",
                Env::SYS_NAME
            );
            let values: Vec<&str> = output.split_whitespace().collect();
            if values.len() == 1 && values[0] == service_name {
                break;
            }

            thread::sleep(time::Duration::from_secs(2));
        }

        Self::run_kubectl_cmd(
            format!(
                "-n {} get ksvc {service_name} --output=custom-columns=URL:.status.url --no-headers",
                Env::K8S_NAMESPACE
            )
            .as_str(),
        ).strip_prefix("http://").unwrap().to_string()
    }

    /// Returns the ingress URL of the load balancer of the Knative deployment.
    /// In our case it is provided by Kourier.
    pub fn get_knative_lb_ip() -> String {
        Self::run_kubectl_cmd(
            "-n kourier-system get svc kourier -o jsonpath={.status.loadBalancer.ingress[0].ip}",
        )
    }

    fn template_yaml_and_run_cmd(
        cmd: &str,
        yaml_path: &PathBuf,
        env_vars: &BTreeMap<&str, String>,
    ) {
        // First, template the YAML file with the provided env. vars
        let templated_yaml = Self::template_yaml(yaml_path, env_vars);

        let mut kubectl = Command::new(Self::get_kubectl_cmd())
            .arg(cmd)
            .arg("-f")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("sc2-exp(k8s): failed to start kubectl apply");

        kubectl
            .stdin
            .as_mut()
            .expect("sc2-exp(k8s): failed to open stdin for kubectl")
            .write_all(templated_yaml.as_bytes())
            .expect("sc2-exp(k8s): failed to feed kubectl through stdin");

        // Check if the kubectl command succeeded
        let output = kubectl
            .wait_with_output()
            .expect("sc2-exp(k8s): failed to run kubectl command");

        match output.status.code() {
            Some(0) => {
                debug!("sc2-exp(k8s): {}", str::from_utf8(&output.stdout).unwrap());
            }
            Some(code) => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                let stdout =
                    str::from_utf8(&output.stdout).unwrap_or("sc2-exp(k8s): failed to get stdout");
                panic!(
                    "{}(k8s): kubectl exited with error (code: {code}): stdout: {stdout} - stderr: {stderr}",
                    Env::SYS_NAME
                );
            }
            None => {
                let stdout =
                    str::from_utf8(&output.stdout).unwrap_or("sc2-exp(k8s): failed to get stdout");
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!(
                    "{}(k8s): kubectl command failed: stdout: {stdout} - stderr: {stderr}",
                    Env::SYS_NAME
                );
            }
        };
    }

    /// Deploy Knative service from `yaml_path`, templated with `env_vars`, and
    /// return the IP that we can use to `curl` the service
    pub fn deploy_knative_service(
        yaml_path: &PathBuf,
        env_vars: &BTreeMap<&str, String>,
    ) -> String {
        Self::template_yaml_and_run_cmd("apply", yaml_path, env_vars);

        // Return the URL
        Self::get_knative_service_url(&env_vars["KSERVICE_NAME"])
    }

    /// Get the Knative deployment ID given a service name
    pub fn get_knative_deployment_id(service_name: &str) -> String {
        Self::run_kubectl_cmd(
            &format!("-n {} get deployments -l apps.sc2.io/name={service_name} -o jsonpath={{.items..metadata.name}}",
            Env::K8S_NAMESPACE
            )
        )
    }

    pub fn scale_knative_service_to_zero(service_name: &str) {
        // Wait for the scale-to-zero to take effect
        loop {
            let output = Self::run_kubectl_cmd(
                &format!("-n {} get pods -l apps.sc2.io/name={service_name} -o jsonpath={{..status.conditions[?(@.type==\"Ready\")].status}}",
                Env::K8S_NAMESPACE
                )
            );
            debug!(
                "{}: waiting for a scale-down service '{service_name}': out: {output}",
                Env::SYS_NAME
            );
            let values: Vec<&str> = output.split_whitespace().collect();

            if values.is_empty() {
                break;
            }

            thread::sleep(time::Duration::from_secs(2));
        }
    }

    pub fn delete_knative_service(yaml_path: &PathBuf, env_vars: &BTreeMap<&str, String>) {
        Self::template_yaml_and_run_cmd("delete", yaml_path, env_vars);
    }
}
