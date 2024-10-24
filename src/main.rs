use crate::experiment::{AvailableExperiments, EvalRunArgs, Exp};
use crate::plot::Plot;
use clap::{Parser, Subcommand};
// TODO: configure logger
// use env_logger;

pub mod experiment;
pub mod plot;

#[derive(Parser)]
struct Cli {
    // The name of the task to execute
    #[clap(subcommand)]
    task: EvalCommand,

    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Debug, Subcommand)]
enum EvalSubCommand {
    /// Run
    Run(EvalRunArgs),
    /// Plot
    Plot {},
}

#[derive(Debug, Subcommand)]
enum EvalCommand {
    /// Evaluate the start-up latency
    StartUp {
        #[command(subcommand)]
        eval_sub_command: EvalSubCommand,
    },
    /// Evaluate scale-out latency
    ScaleOut {
        #[command(subcommand)]
        eval_sub_command: EvalSubCommand,
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
        EvalCommand::ScaleOut { eval_sub_command } => match eval_sub_command {
            EvalSubCommand::Run(run_args) => {
                Exp::run(&AvailableExperiments::ScaleOut, run_args);
            }
            EvalSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::ScaleOut);
            }
        },
        EvalCommand::StartUp { eval_sub_command } => match eval_sub_command {
            EvalSubCommand::Run(run_args) => {
                Exp::run(&AvailableExperiments::StartUp, run_args);
            }
            EvalSubCommand::Plot {} => {
                Plot::plot(&AvailableExperiments::StartUp);
            }
        },
    }
}
