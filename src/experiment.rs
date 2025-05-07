use crate::{
    args::{ImagePullRunArgs, RunCommonArgs, ScaleOutRunArgs, StartUpRunArgs},
    baselines::{
        AvailableBaselines, ImagePullBaselines, ImagePullEncryptionTypes, ImagePullWorkloads,
        StartUpFlavours,
    },
    containerd::Containerd,
    deploy::Deploy,
    env::Env,
    kubernetes::K8s,
};
use chrono::{DateTime, Duration, Utc};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use log::warn;
use std::{
    collections::BTreeMap, fmt, fs, io::Write, path::PathBuf, process::Command, str, thread, time,
};

#[derive(PartialEq)]
pub enum AvailableExperiments {
    ImagePull,
    ScaleOut,
    StartUp,
}

impl fmt::Display for AvailableExperiments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvailableExperiments::ImagePull => write!(f, "image-pull"),
            AvailableExperiments::ScaleOut => write!(f, "scale-out"),
            AvailableExperiments::StartUp => write!(f, "start-up"),
        }
    }
}

pub struct ExecutionResult {
    iter: u32,
    // Single (start, end) timestamp pairs
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    // Breakdown of (start, end) timestamp pairs
    event_ts: BTreeMap<String, (DateTime<Utc>, DateTime<Utc>)>,
}

impl ExecutionResult {
    pub fn new() -> ExecutionResult {
        ExecutionResult {
            iter: 0,
            start_time: Utc::now(),
            end_time: Utc::now(),
            event_ts: BTreeMap::new(),
        }
    }
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Exp {}

impl Exp {
    /// Helper functions
    fn init_data_file(results_file: &PathBuf, exp: &AvailableExperiments) {
        // Open data file
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(results_file)
            .expect("sc2-eval(k8s): failed to open data file at: {results_file:?}");

        match exp {
            AvailableExperiments::ScaleOut => {
                writeln!(file, "Run,TimeMs")
                    .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");
            }
            AvailableExperiments::ImagePull | AvailableExperiments::StartUp => {
                writeln!(file, "Run,Event,TimeMs")
                    .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");
            }
        }
    }

    fn write_results_to_file(
        results_file: &PathBuf,
        exp: &AvailableExperiments,
        exec_results: &ExecutionResult,
    ) {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(results_file)
            .expect("sc2-eval(k8s): failed to open data file at: {results_file:?}");

        match exp {
            AvailableExperiments::ImagePull => {
                // Manually write-down the end-to-end event
                let total_duration: Duration = exec_results.end_time - exec_results.start_time;
                writeln!(
                    file,
                    "{},EndToEnd,{}",
                    exec_results.iter,
                    total_duration.num_milliseconds()
                )
                .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");

                // Write all the events that we decide to record for the
                // break-down of the image-pull time. Keep track of the
                // largest time-stamp here, we will use it, together with the
                // e2e one, to measure the actual runtime
                let mut max_end_ts: DateTime<Utc> = exec_results.start_time;
                for (event, (start_ts, end_ts)) in &exec_results.event_ts {
                    let duration: Duration = *end_ts - *start_ts;
                    writeln!(
                        file,
                        "{},{},{}",
                        exec_results.iter,
                        event,
                        duration.num_milliseconds()
                    )
                    .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");

                    if *end_ts > max_end_ts {
                        max_end_ts = *end_ts;
                    }
                }

                let runtime_duration: Duration = exec_results.end_time - max_end_ts;
                writeln!(
                    file,
                    "{},FuncRuntime,{}",
                    exec_results.iter,
                    runtime_duration.num_milliseconds()
                )
                .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");
            }
            AvailableExperiments::ScaleOut => {
                let duration: Duration = exec_results.end_time - exec_results.start_time;
                writeln!(
                    file,
                    "{},{}",
                    exec_results.iter,
                    duration.num_milliseconds()
                )
                .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");
            }
            AvailableExperiments::StartUp => {
                // Manually write-down the end-to-end event
                let total_duration: Duration = exec_results.end_time - exec_results.start_time;
                writeln!(
                    file,
                    "{},StartUp,{}",
                    exec_results.iter,
                    total_duration.num_milliseconds()
                )
                .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");

                // Write all the events that we decide to record for the
                // break-down of the start-up time
                for (event, (start_ts, end_ts)) in &exec_results.event_ts {
                    let duration: Duration = *end_ts - *start_ts;
                    writeln!(
                        file,
                        "{},{},{}",
                        exec_results.iter,
                        event,
                        duration.num_milliseconds()
                    )
                    .expect("sc2-eval(k8s): failed to write to data file at: {results_file:?}");
                }
            }
        };
    }

    /// Helper function to get a progress bar to visualize experiment progress
    fn get_progress_bar(num_repeats: u64, msg: String) -> ProgressBar {
        let pb = ProgressBar::new(num_repeats);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("sc2-eval(k8s): error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message(msg);
        pb
    }

    /// This method executes a single instance of the experiment by `curl`-ing
    /// the corresponding `service_ip`, and populates the ExecutionResult with
    /// all the fields required by the `AvailableExperiment` we are running
    fn run_knative_experiment_once(
        _exp: &AvailableExperiments,
        service_name: &str,
        service_url: &str,
    ) -> ExecutionResult {
        let lb_ip = K8s::get_knative_lb_ip();

        // Note that this initialises start_time to Utc::now()
        let mut exec_result = ExecutionResult::new();

        // Do single execution
        let output = if service_name == "tf-inference" {
            let mut image_data_path = Env::apps_root();
            image_data_path.push("functions");
            image_data_path.push("tf-inference");
            image_data_path.push("preprocess-image");
            image_data_path.push("image_data.json");

            debug!(
                "{}: running curl command to lb ip: {lb_ip} with host: {service_url} and image data: {}",
                Env::SYS_NAME,
                format!("@{}", image_data_path.to_string_lossy().into_owned()),
            );

            Command::new("curl")
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg(format!("Host: {service_url}/v1/models/mobilenet:predict"))
                .arg("-d")
                .arg(format!(
                    "@{}",
                    image_data_path.to_string_lossy().into_owned()
                ))
                .arg(lb_ip)
                .output()
                .expect("sc2-eval(k8s): failed to spawn curl command")
        } else {
            debug!(
                "{}: running curl command to lb ip: {lb_ip} with host: {service_url}",
                Env::SYS_NAME
            );

            Command::new("curl")
                .arg("-H")
                .arg(format!("Host: {service_url}"))
                .arg(lb_ip)
                .output()
                .expect("sc2-eval(k8s): failed to spawn curl command")
        };

        match output.status.code() {
            Some(0) => {
                exec_result.end_time = Utc::now();

                let stdout = str::from_utf8(&output.stdout)
                    .unwrap_or("sc2-exp(k8s): failed to get stdout")
                    .trim();

                // For some reason, it seems that bad requests do not always
                // trigger a non-zero return code, so we catch them here
                if stdout == "Bad Request" {
                    panic!("{}(k8s): curl received bad request!", Env::SYS_NAME);
                }
                debug!("{}(k8s): got '{stdout}'", Env::SYS_NAME);
            }
            Some(code) => {
                let stdout =
                    str::from_utf8(&output.stdout).unwrap_or("sc2-exp(k8s): failed to get stdout");
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!(
                    "{}(k8s): curl exited with error (code: {code}): stdout: {stdout} - stderr: {stderr}",
                    Env::SYS_NAME
                );
            }
            None => {
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!("{}(k8s): kubectl command failed: {stderr}", Env::SYS_NAME);
            }
        };

        let deployment_id = K8s::get_knative_deployment_id(service_name);
        // Get the cutoff time to filter outputs of the journal log, and leave us some slack
        let cutoff_time = exec_result.start_time - chrono::Duration::milliseconds(500);

        debug!(
            "{}(k8s): got knative deployment id: {deployment_id}",
            Env::SYS_NAME
        );
        exec_result.event_ts = Containerd::get_events_from_journalctl(&deployment_id, &cutoff_time);

        // Common clean-up after single execution
        debug!(
            "{}(k8s): scaling service '{service_name}' to zero",
            Env::SYS_NAME
        );
        K8s::scale_knative_service_to_zero(service_name);

        // Cautionary sleep between runs
        thread::sleep(time::Duration::from_secs(2));

        // Return execution result
        exec_result
    }

    fn clean_up_after_run(_exp: &AvailableExperiments, env_vars: &BTreeMap<&str, String>) {
        let flavour: StartUpFlavours = env_vars["START_UP_FLAVOUR"].parse().unwrap();
        if flavour == StartUpFlavours::Cold {
            Deploy::purge_snapshotters();
        }
    }

    /// This method takes a _single_ deployment configuration, specified as
    /// a YAML file and a map of env. vars to template it, and executes it
    /// according to the requested experiment, using the given run args
    fn run_knative_experiment(
        exp: &AvailableExperiments,
        args: &RunCommonArgs,
        yaml_path: &PathBuf,
        env_vars: &BTreeMap<&str, String>,
    ) {
        // Deploy the baseline
        let service_url = K8s::deploy_knative_service(yaml_path, env_vars);
        warn!("service url: {service_url}");

        // Cautionary sleep before starting the experiment
        thread::sleep(time::Duration::from_secs(2));

        // Initialise data file
        let mut results_file: PathBuf = Env::results_root();
        results_file.push(format!("{exp}"));
        results_file.push("data");
        fs::create_dir_all(results_file.clone()).unwrap();
        results_file.push(match &exp {
            AvailableExperiments::ImagePull => {
                format!(
                    "{}_{}_{}_{}.csv",
                    env_vars["WORKLOAD"],
                    env_vars["ENCRYPTION"],
                    env_vars["IMAGE_PULL_TYPE"],
                    env_vars["START_UP_FLAVOUR"]
                )
            }
            AvailableExperiments::ScaleOut => {
                format!("{}_{}.csv", env_vars["SC2_BASELINE"], env_vars["SCALE_IDX"])
            }
            AvailableExperiments::StartUp => {
                format!(
                    "{}_{}.csv",
                    env_vars["SC2_BASELINE"], env_vars["START_UP_FLAVOUR"]
                )
            }
        });
        Self::init_data_file(&results_file, exp);

        // Run the experiment (warm-up)
        for _ in 0..args.num_warmup_repeats {
            Self::run_knative_experiment_once(exp, &env_vars["KSERVICE_NAME"], &service_url);
            Self::clean_up_after_run(exp, env_vars);
        }

        // Run the actual experiment
        let pb = Self::get_progress_bar(
            args.num_repeats.into(),
            match &exp {
                AvailableExperiments::ImagePull => {
                    format!(
                        "{}/{}-{}/{}/{}",
                        exp,
                        env_vars["WORKLOAD"],
                        env_vars["ENCRYPTION"],
                        env_vars["IMAGE_PULL_TYPE"],
                        env_vars["START_UP_FLAVOUR"],
                    )
                }
                AvailableExperiments::ScaleOut => {
                    format!(
                        "{}/{}/{}",
                        exp, env_vars["SC2_BASELINE"], env_vars["SCALE_IDX"]
                    )
                }
                AvailableExperiments::StartUp => {
                    format!(
                        "{}/{}/{}",
                        exp, env_vars["SC2_BASELINE"], env_vars["START_UP_FLAVOUR"]
                    )
                }
            },
        );
        for i in 0..args.num_repeats {
            // Run experiment
            let mut exec_results =
                Self::run_knative_experiment_once(exp, &env_vars["KSERVICE_NAME"], &service_url);
            Self::clean_up_after_run(exp, env_vars);

            // Write results to file
            exec_results.iter = i;
            Self::write_results_to_file(&results_file, exp, &exec_results);
            pb.inc(1);
        }
        pb.finish();

        // Delete the experiment
        K8s::delete_knative_service(yaml_path, env_vars);
    }

    // -------------------------------------------------------------------------
    // Main entrypoints to run experiments
    // -------------------------------------------------------------------------

    pub fn run_start_up(args: &StartUpRunArgs) {
        // Work-out the start-up flavour
        let start_up_flavours = args
            .flavour
            .clone()
            .map(|val| vec![val])
            .unwrap_or_else(|| vec![StartUpFlavours::Cold, StartUpFlavours::Warm]);

        for baseline in &args.baseline {
            // Work-out the Knative service to deploy
            let yaml_path = Env::apps_root()
                .join("functions")
                .join("hello-world")
                .join("service.yaml");

            // Work-out the env. vars that we need to template in the service file
            let mut env_vars: BTreeMap<&str, String> = BTreeMap::from([
                ("SC2_BASELINE", format!("{baseline}")),
                ("SC2_NAMESPACE", Env::K8S_NAMESPACE.to_string()),
                ("CTR_REGISTRY_URL", Env::CONTAINER_REGISTRY_URL.to_string()),
                (
                    "RUNTIME_CLASS_NAME",
                    match baseline {
                        AvailableBaselines::Runc => "runc".to_string(),
                        AvailableBaselines::Kata => "kata-qemu".to_string(),
                        AvailableBaselines::Snp => "kata-qemu-snp".to_string(),
                        AvailableBaselines::SnpSc2 => "kata-qemu-snp-sc2".to_string(),
                        AvailableBaselines::Tdx => "kata-qemu-tdx".to_string(),
                        AvailableBaselines::TdxSc2 => "kata-qemu-tdx-sc2".to_string(),
                    },
                ),
                ("KSERVICE_NAME", "hello-world".to_string()),
                ("IMAGE_NAME", "hello-world".to_string()),
                ("IMAGE_TAG", "unencrypted".to_string()),
            ]);

            // Per-experiment env. var templating and execution
            for flavour in &start_up_flavours {
                env_vars.insert("START_UP_FLAVOUR", flavour.to_string());
                Self::run_knative_experiment(
                    &AvailableExperiments::StartUp,
                    &args.common,
                    &yaml_path,
                    &env_vars,
                );
            }
        }
    }

    pub fn run_scale_out(args: &ScaleOutRunArgs) {
        // TODO: think if we want different start-up flavours for scale-out
        let start_up_flavour = StartUpFlavours::Cold;

        // TODO: think if we want different baselines for scale-out
        for baseline in &args.baseline {
            // Work-out the Knative service to deploy
            let yaml_path = Env::apps_root()
                .join("functions")
                .join("hello-world-scale-out")
                .join("service.yaml");

            // Work-out the env. vars that we need to template in the service file
            let mut env_vars: BTreeMap<&str, String> = BTreeMap::from([
                ("SC2_BASELINE", format!("{baseline}")),
                ("SC2_NAMESPACE", Env::K8S_NAMESPACE.to_string()),
                ("CTR_REGISTRY_URL", Env::CONTAINER_REGISTRY_URL.to_string()),
                (
                    "RUNTIME_CLASS_NAME",
                    match baseline {
                        AvailableBaselines::Runc => "runc".to_string(),
                        AvailableBaselines::Kata => "kata-qemu".to_string(),
                        AvailableBaselines::Snp => "kata-qemu-snp".to_string(),
                        AvailableBaselines::SnpSc2 => "kata-qemu-snp-sc2".to_string(),
                        AvailableBaselines::Tdx => "kata-qemu-tdx".to_string(),
                        AvailableBaselines::TdxSc2 => "kata-qemu-tdx-sc2".to_string(),
                    },
                ),
                ("KSERVICE_NAME", "hello-world".to_string()),
                ("IMAGE_NAME", "hello-world".to_string()),
                ("IMAGE_TAG", "unencrypted".to_string()),
                ("START_UP_FLAVOUR", start_up_flavour.to_string()),
            ]);

            for i in 1..args.scale_up_range {
                env_vars.insert("SCALE_IDX", i.to_string());
                Self::run_knative_experiment(
                    &AvailableExperiments::ScaleOut,
                    &args.common,
                    &yaml_path,
                    &env_vars,
                );
            }
        }
    }

    /// In this experiment we measure the overheads of different image-pull
    /// mechanisms. We compare the default guest-pull mechanisms, with a
    /// mechanism to mount images from the host using dm-verity, and an
    /// extensions of the guest-pull mechanism that uses lazy-pulling as
    /// enabled by the nydus image format.
    ///
    /// For each image-pull mechanism we compare encrypted and unencrypted
    /// images, and cold and warm starts. We also use three different
    /// workloads.
    pub fn run_image_pull(args: &ImagePullRunArgs) {
        let yaml_path: PathBuf = Env::apps_root().join("functions");
        let baseline = AvailableBaselines::SnpSc2;

        // ---------------------------------------------------------------------
        // Parse command line arguments
        //
        // For each configuration knob, we allow manually selecting one option
        // or, if not, using an array of default values
        // ---------------------------------------------------------------------

        let start_up_flavours = args
            .flavour
            .clone()
            .map(|val| vec![val])
            .unwrap_or_else(|| vec![StartUpFlavours::Cold, StartUpFlavours::Warm]);

        let image_pull_types = args
            .image_pull_type
            .clone()
            .map(|val| vec![val])
            .unwrap_or_else(|| {
                vec![
                    ImagePullBaselines::GuestPull,
                    ImagePullBaselines::GuestLazy,
                    ImagePullBaselines::HostMount,
                ]
            });

        let image_pull_workloads = args
            .image_pull_workload
            .clone()
            .map(|val| vec![val])
            .unwrap_or_else(|| {
                vec![
                    ImagePullWorkloads::HelloWorld,
                    ImagePullWorkloads::Fio,
                    ImagePullWorkloads::TfInference,
                ]
            });

        let image_pull_encryption_types = args
            .image_pull_encryption
            .clone()
            .map(|val| vec![val])
            .unwrap_or_else(|| {
                vec![
                    // TODO: enable image encryption
                    // ImagePullEncryptionTypes::Encrypted,
                    ImagePullEncryptionTypes::UnEncrypted,
                ]
            });

        // ---------------------------------------------------------------------
        // Run experiments
        // ---------------------------------------------------------------------

        let mut env_vars: BTreeMap<&str, String> = BTreeMap::from([
            ("SC2_BASELINE", format!("{baseline}")),
            ("SC2_NAMESPACE", Env::K8S_NAMESPACE.to_string()),
            ("CTR_REGISTRY_URL", Env::CONTAINER_REGISTRY_URL.to_string()),
            (
                "RUNTIME_CLASS_NAME",
                match baseline {
                    AvailableBaselines::Runc => "runc".to_string(),
                    AvailableBaselines::Kata => "kata-qemu".to_string(),
                    AvailableBaselines::Snp => "kata-qemu-snp".to_string(),
                    AvailableBaselines::SnpSc2 => "kata-qemu-snp-sc2".to_string(),
                    AvailableBaselines::Tdx => "kata-qemu-tdx".to_string(),
                    AvailableBaselines::TdxSc2 => "kata-qemu-tdx-sc2".to_string(),
                },
            ),
        ]);

        for workload in &image_pull_workloads {
            let mut image_tag: String;

            env_vars.insert("WORKLOAD", workload.to_string());
            env_vars.insert("KSERVICE_NAME", workload.to_string());
            env_vars.insert("IMAGE_NAME", workload.to_string());

            // Update YAML path
            let mut this_yaml_path = yaml_path.clone();
            this_yaml_path.push(workload.to_string());
            this_yaml_path.push("service.yaml");

            for encryption_type in &image_pull_encryption_types {
                env_vars.insert("ENCRYPTION", encryption_type.to_string());
                image_tag = encryption_type.to_string();

                for image_pull_type in &image_pull_types {
                    // Set the snapshotter mode
                    match image_pull_type {
                        ImagePullBaselines::GuestPull | ImagePullBaselines::GuestLazy => {
                            Deploy::set_snapshotter_mode("guest-pull");
                        }
                        ImagePullBaselines::HostMount => {
                            Deploy::set_snapshotter_mode("host-share");
                        }
                        _ => todo!(),
                    }

                    // Purge to ensure a fresh start with the
                    // new snapshotter
                    Deploy::purge_snapshotters();

                    // Work-out the image tag based on the pull type
                    // and update the yaml path
                    if image_pull_type == &ImagePullBaselines::GuestLazy {
                        image_tag += "-nydus";
                    }

                    env_vars.insert("IMAGE_PULL_TYPE", image_pull_type.to_string());
                    env_vars.insert("IMAGE_TAG", image_tag.clone());

                    for start_up_flavour in &start_up_flavours {
                        env_vars.insert("START_UP_FLAVOUR", start_up_flavour.to_string());

                        Self::run_knative_experiment(
                            &AvailableExperiments::ImagePull,
                            &args.common,
                            &this_yaml_path,
                            &env_vars,
                        );
                    }
                }
            }
        }
    }
}
