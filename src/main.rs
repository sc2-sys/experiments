use crate::{
    args::{ExpCommand, ImagePullSubCommand, ScaleOutSubCommand, StartUpSubCommand},
    experiment::{AvailableExperiments, Exp},
    plot::Plot,
};
use clap::Parser;

pub mod args;
pub mod baselines;
pub mod containerd;
pub mod cri;
pub mod deploy;
pub mod env;
pub mod experiment;
pub mod kubernetes;
pub mod plot;

#[derive(Parser)]
struct Cli {
    // The name of the task to execute
    #[clap(subcommand)]
    task: ExpCommand,

    #[arg(short, long, global = true)]
    debug: bool,
}

fn main() {
    let cli = Cli::parse();

    // TODO: make sure that all application images exist in the container registry

    // Initialize the logger based on the debug flag
    if cli.debug {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    match &cli.task {
        ExpCommand::ImagePull {
            exp_sub_command: eval_sub_command,
        } => match eval_sub_command {
            ImagePullSubCommand::Run(run_args) => {
                Exp::run_image_pull(run_args);
            }
            ImagePullSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::ImagePull);
            }
        },
        ExpCommand::ScaleOut {
            exp_sub_command: eval_sub_command,
        } => match eval_sub_command {
            ScaleOutSubCommand::Run(run_args) => {
                Exp::run_scale_out(run_args);
            }
            ScaleOutSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::ScaleOut);
            }
        },
        ExpCommand::StartUp {
            exp_sub_command: eval_sub_command,
        } => match eval_sub_command {
            StartUpSubCommand::Run(run_args) => {
                Exp::run_start_up(run_args);
            }
            StartUpSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::StartUp);
            }
        },
    }
}
