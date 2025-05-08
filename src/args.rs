use crate::baselines::{
    AvailableBaselines, ImagePullBaselines, ImagePullEncryptionTypes, ImagePullWorkloads,
    StartUpFlavours,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct RunCommonArgs {
    #[arg(long, default_value = "3")]
    pub num_repeats: u32,
    #[arg(long, default_value = "1")]
    pub num_warmup_repeats: u32,
}

#[derive(Debug, Args)]
pub struct StartUpRunArgs {
    #[command(flatten)]
    pub common: RunCommonArgs,
    #[arg(long, num_args = 1.., value_name = "BASELINE")]
    pub baseline: Vec<AvailableBaselines>,
    #[arg(long, value_name = "STARTUP_FLAVOUR")]
    pub flavour: Option<StartUpFlavours>,
}

#[derive(Debug, Args)]
pub struct ScaleOutRunArgs {
    #[command(flatten)]
    pub common: RunCommonArgs,
    #[arg(long, num_args = 1.., value_name = "BASELINE")]
    pub baseline: Vec<AvailableBaselines>,
    #[arg(long, default_value = "4")]
    pub scale_up_range: u32,
}

#[derive(Debug, Args)]
pub struct ImagePullRunArgs {
    #[command(flatten)]
    pub common: RunCommonArgs,
    #[arg(long, value_name = "STARTUP_FLAVOUR")]
    pub flavour: Option<StartUpFlavours>,
    #[arg(long)]
    pub pull_type: Option<ImagePullBaselines>,
    #[arg(long)]
    pub workload: Option<ImagePullWorkloads>,
    #[arg(long)]
    pub encryption: Option<ImagePullEncryptionTypes>,
}

#[derive(Debug, Subcommand)]
pub enum StartUpSubCommand {
    /// Run
    Run(StartUpRunArgs),
    /// Plot
    Plot {},
}

#[derive(Debug, Subcommand)]
pub enum ScaleOutSubCommand {
    /// Run
    Run(ScaleOutRunArgs),
    /// Plot
    Plot {},
}

#[derive(Debug, Subcommand)]
pub enum ImagePullSubCommand {
    /// Run
    Run(ImagePullRunArgs),
    /// Plot
    Plot {},
}

#[derive(Debug, Subcommand)]
pub enum ExpCommand {
    /// Experiment comparing different image pulling techniques
    ImagePull {
        #[command(subcommand)]
        exp_sub_command: ImagePullSubCommand,
    },
    /// Evaluate the start-up latency
    StartUp {
        #[command(subcommand)]
        exp_sub_command: StartUpSubCommand,
    },
    /// Evaluate scale-out latency
    ScaleOut {
        #[command(subcommand)]
        exp_sub_command: ScaleOutSubCommand,
    },
}
