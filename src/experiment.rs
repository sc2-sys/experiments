use crate::{containerd::Containerd, cri::Cri, env::Env, kubernetes::K8s};
use chrono::{DateTime, Duration, Utc};
use clap::{Args, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use plotters::prelude::RGBColor;
use std::{
    collections::BTreeMap, fmt, fs, io::Write, path::PathBuf, process::Command, str, str::FromStr,
    thread, time,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum AvailableBaselines {
    Runc,
    Kata,
    Snp,
    SnpSc2,
    Tdx,
    TdxSc2,
}

impl fmt::Display for AvailableBaselines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvailableBaselines::Runc => write!(f, "runc"),
            AvailableBaselines::Kata => write!(f, "kata"),
            AvailableBaselines::Snp => write!(f, "snp"),
            AvailableBaselines::SnpSc2 => write!(f, "snp-sc2"),
            AvailableBaselines::Tdx => write!(f, "tdx"),
            AvailableBaselines::TdxSc2 => write!(f, "tdx-sc2"),
        }
    }
}

impl FromStr for AvailableBaselines {
    type Err = ();

    fn from_str(input: &str) -> Result<AvailableBaselines, Self::Err> {
        match input {
            "runc" => Ok(AvailableBaselines::Runc),
            "kata" => Ok(AvailableBaselines::Kata),
            "snp" => Ok(AvailableBaselines::Snp),
            "snp-sc2" => Ok(AvailableBaselines::SnpSc2),
            "tdx" => Ok(AvailableBaselines::Tdx),
            "tdx-sc2" => Ok(AvailableBaselines::TdxSc2),
            _ => Err(()),
        }
    }
}

impl AvailableBaselines {
    pub fn iter_variants() -> std::slice::Iter<'static, AvailableBaselines> {
        static VARIANTS: [AvailableBaselines; 6] = [
            AvailableBaselines::Runc,
            AvailableBaselines::Kata,
            AvailableBaselines::Snp,
            AvailableBaselines::SnpSc2,
            AvailableBaselines::Tdx,
            AvailableBaselines::TdxSc2,
        ];
        VARIANTS.iter()
    }

    pub fn get_color(&self) -> RGBColor {
        match self {
            AvailableBaselines::Runc => RGBColor(122, 92, 117),
            AvailableBaselines::Kata => RGBColor(171, 222, 230),
            AvailableBaselines::Snp => RGBColor(203, 170, 203),
            AvailableBaselines::SnpSc2 => RGBColor(213, 160, 163),
            AvailableBaselines::Tdx => RGBColor(255, 255, 181),
            AvailableBaselines::TdxSc2 => RGBColor(205, 255, 101),
        }
    }
}

#[derive(Debug, Args)]
pub struct ExpRunArgs {
    #[arg(long, num_args = 1.., value_name = "BASELINE")]
    baseline: Vec<AvailableBaselines>,
    #[arg(long, default_value = "3")]
    num_repeats: u32,
    #[arg(long, default_value = "1")]
    num_warmup_repeats: u32,
    #[arg(long, default_value = "4")]
    scale_up_range: u32,
}

#[derive(PartialEq)]
pub enum AvailableExperiments {
    ScaleOut,
    StartUp,
}

impl fmt::Display for AvailableExperiments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            AvailableExperiments::StartUp => {
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
    fn get_progress_bar(
        num_repeats: u64,
        msg: String,
    ) -> ProgressBar {
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
        service_ip: &str,
    ) -> ExecutionResult {
        // Note that this initialises start_time to Utc::now()
        let mut exec_result = ExecutionResult::new();

        // Do single execution
        debug!(
            "{}: running curl command to ip: {service_ip}",
            Env::SYS_NAME
        );
        let output = Command::new("curl")
            .arg(service_ip)
            .output()
            .expect("sc2-eval(k8s): failed to spawn curl command");

        match output.status.code() {
            Some(0) => {
                exec_result.end_time = Utc::now();

                let stdout = str::from_utf8(&output.stdout)
                    .unwrap_or("sc2-exp(k8s): failed to get stdout")
                    .trim();
                debug!("{}(k8s): got '{stdout}'", Env::SYS_NAME);
            }
            Some(code) => {
                let stdout =
                    str::from_utf8(&output.stdout).unwrap_or("sc2-exp(k8s): failed to get stdout");
                let stderr =
                    str::from_utf8(&output.stderr).unwrap_or("sc2-exp(k8s): failed to get stderr");
                panic!(
                    "{}(k8s): kubectl exited with error (code: {code}): stdout: {stdout} - stderr: {stderr}",
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

    fn clean_up_after_run(exp: &AvailableExperiments, env_vars: &BTreeMap<&str, String>) {
        if exp == &AvailableExperiments::StartUp && env_vars["START_UP_FLAVOUR"] == "cold" {
            Cri::remove_image(format!(
                "{}/helloworld-py:unencrypted",
                env_vars["CTR_REGISTRY_URL"]
            ));
        }
    }

    /// This method takes a _single_ deployment configuration, specified as
    /// a YAML file and a map of env. vars to template it, and executes it
    /// according to the requested experiment, using the given run args
    fn run_knative_experiment(
        exp: &AvailableExperiments,
        args: &ExpRunArgs,
        yaml_path: &PathBuf,
        env_vars: &BTreeMap<&str, String>,
    ) {
        // Deploy the baseline
        let service_ip = K8s::deploy_knative_service(yaml_path, env_vars);

        // Cautionary sleep before starting the experiment
        thread::sleep(time::Duration::from_secs(2));

        // Initialise data file
        let mut results_file: PathBuf = Env::results_root();
        results_file.push(format!("{exp}"));
        results_file.push("data");
        fs::create_dir_all(results_file.clone()).unwrap();
        results_file.push(match &exp {
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
            Self::run_knative_experiment_once(exp, &env_vars["KSERVICE_NAME"], &service_ip);
            Self::clean_up_after_run(exp, env_vars);
        }

        // Run the actual experiment
        let pb = Self::get_progress_bar(
            args.num_repeats.into(),
            match &exp {
                AvailableExperiments::ScaleOut => {
                    format!("{}/{}/{}", exp, env_vars["SC2_BASELINE"], env_vars["SCALE_IDX"])
                }
                AvailableExperiments::StartUp => {
                    format!("{}/{}/{}", exp, env_vars["SC2_BASELINE"], env_vars["START_UP_FLAVOUR"])
                }
            },
        );
        for i in 0..args.num_repeats {
            // Run experiment
            let mut exec_results =
                Self::run_knative_experiment_once(exp, &env_vars["KSERVICE_NAME"], &service_ip);
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

    /// Main entrypoint to execute an experiment in SC2. We iterate over the
    /// different baselines to run, as well as the different experiment args
    /// for each experiment, and populate a map of env. vars to template
    /// the serivce's YAML path. Once we have a single templated yaml path,
    /// we can call run_knative_experiment to handle the deployment, execution,
    /// clean-up, and result aggregation
    pub fn run(exp: &AvailableExperiments, args: &ExpRunArgs) {
        // Work-out the Knative service to deploy
        let mut apps_root = Env::apps_root();
        let yaml_path: PathBuf = match &exp {
            AvailableExperiments::ScaleOut => {
                apps_root.push("functions");
                apps_root.push("helloworld-py-scaleout");
                apps_root.push("service.yaml");
                apps_root
            }
            AvailableExperiments::StartUp => {
                apps_root.push("functions");
                apps_root.push("helloworld-py");
                apps_root.push("service.yaml");
                apps_root
            }
        };

        for baseline in &args.baseline {
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
            ]);

            // Per-experiment env. var templating and execution
            match &exp {
                AvailableExperiments::ScaleOut => {
                    env_vars.insert("KSERVICE_NAME", "helloworld-py".to_string());
                    for i in 1..args.scale_up_range {
                        env_vars.insert("SCALE_IDX", i.to_string());
                        Self::run_knative_experiment(exp, args, &yaml_path, &env_vars);
                    }
                }
                AvailableExperiments::StartUp => {
                    env_vars.insert("KSERVICE_NAME", "helloworld-py".to_string());
                    for flavour in ["cold", "warm"] {
                        env_vars.insert("START_UP_FLAVOUR", flavour.to_string());
                        Self::run_knative_experiment(exp, args, &yaml_path, &env_vars);
                    }
                }
            };
        }
    }
}
