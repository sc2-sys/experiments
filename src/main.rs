use crate::experiment::{AvailableExperiments, Exp, ExpRunArgs};
use crate::plot::Plot;
use clap::{Parser, Subcommand};

pub mod containerd;
pub mod cri;
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

#[derive(Debug, Subcommand)]
enum ExpSubCommand {
    /// Run
    Run(ExpRunArgs),
    /// Plot
    Plot {},
}

#[derive(Debug, Subcommand)]
enum ExpCommand {
    /// Evaluate the start-up latency
    StartUp {
        #[command(subcommand)]
        exp_sub_command: ExpSubCommand,
    },
    /// Evaluate scale-out latency
    ScaleOut {
        #[command(subcommand)]
        exp_sub_command: ExpSubCommand,
    },
}

fn main() {
    let cli = Cli::parse();

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
        ExpCommand::ScaleOut {
            exp_sub_command: eval_sub_command,
        } => match eval_sub_command {
            ExpSubCommand::Run(run_args) => {
                Exp::run(&AvailableExperiments::ScaleOut, run_args);
            }
            ExpSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::ScaleOut);
            }
        },
        ExpCommand::StartUp {
            exp_sub_command: eval_sub_command,
        } => match eval_sub_command {
            ExpSubCommand::Run(run_args) => {
                Exp::run(&AvailableExperiments::StartUp, run_args);
            }
            ExpSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::StartUp);
            }
        },
    }
}
