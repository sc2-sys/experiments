use clap::Args;
use std::fmt;

#[derive(Debug, Args)]
pub struct EvalRunArgs {
    #[arg(long, default_value = "3")]
    num_repeats: u32,
    #[arg(long, default_value = "1")]
    num_warmup_repeats: u32,
    #[arg(long, default_value = "4")]
    scale_up_range: u32,
}

pub enum AvailableExperiments {
    ScaleOut,
    StartUp,
}

impl fmt::Display for AvailableExperiments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvailableExperiments::ScaleOut => write!(f, "scale-out"),
            AvailableExperiments::StartUp => write!(f, "scale-up"),
        }
    }
}

#[derive(Debug)]
pub struct Exp {}

impl Exp {
    pub fn run(_exp: &AvailableExperiments, _args: &EvalRunArgs) {}
}
