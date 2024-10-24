use crate::experiment::AvailableExperiments;

#[derive(Debug)]
pub struct Plot {}

impl Plot {
    pub fn plot(exp: &AvailableExperiments) {
        println!("plotting {exp}");
    }
}
