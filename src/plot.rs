use crate::{
    containerd::Containerd,
    env::Env,
    experiment::{AvailableBaselines, AvailableExperiments},
};
use csv::ReaderBuilder;
use log::debug;
use plotters::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

#[derive(Debug)]
pub struct Plot {}

impl Plot {
    /// Collect all CSV files in the data directory for the experiment
    fn get_all_data_files(exp: &AvailableExperiments) -> Vec<PathBuf> {
        let mut data_path = Env::results_root();
        data_path.push(format!("{exp}"));
        data_path.push("data");

        let mut csv_files = Vec::new();
        for entry in fs::read_dir(data_path).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().and_then(|e| e.to_str()) == Some("csv") {
                csv_files.push(entry.path());
            }
        }

        csv_files
    }

    fn plot_start_up_latency(exp: &AvailableExperiments, data_files: &Vec<PathBuf>) {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            #[allow(dead_code)]
            run: u32,
            event: String,
            time_ms: u64,
        }

        // ---------- Collect Data ---------- //

        // This map has one key per baseline, and each baseline holds a map
        // of each event and the average time spent in each event.
        // Note: we stack averages together, which may not be the most
        // statistically-wise thing.
        let mut cold_data = BTreeMap::<AvailableBaselines, BTreeMap<&str, f64>>::new();
        for workflow in AvailableBaselines::iter_variants() {
            let mut inner_map = BTreeMap::<&str, f64>::new();
            for event in Containerd::CONTAINERD_INFO_EVENTS {
                inner_map.insert(event, 0.0);
            }
            cold_data.insert(workflow.clone(), inner_map);
        }
        let mut warm_data = BTreeMap::<AvailableBaselines, BTreeMap<&str, f64>>::new();
        for workflow in AvailableBaselines::iter_variants() {
            let mut inner_map = BTreeMap::<&str, f64>::new();
            for event in Containerd::CONTAINERD_INFO_EVENTS {
                inner_map.insert(event, 0.0);
            }
            warm_data.insert(workflow.clone(), inner_map);
        }

        let mut y_max: f64 = 25.0e3;
        for csv_file in data_files {
            let file_name = csv_file
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();
            let file_name_len = file_name.len();
            let file_name_no_ext = &file_name[0..file_name_len - 4];
            let baseline: AvailableBaselines = file_name_no_ext
                .split('_')
                .collect::<Vec<_>>()[0]
                .parse()
                .unwrap();
            let flavour: String = file_name_no_ext
                .split('_')
                .collect::<Vec<_>>()[1]
                .parse()
                .unwrap();

            // Based on the flavour, we pick one of the data dictionaries
            let data = match flavour.as_str() {
                "cold" => &mut cold_data,
                "warm" => &mut warm_data,
                _ => panic!("unreachable"),
            };

            debug!("Reading data for baseline: {baseline}/{flavour} (file: {csv_file:?}");

            // Open the CSV and deserialize records
            let mut reader = ReaderBuilder::new()
                .has_headers(true)
                .from_path(csv_file)
                .unwrap();
            let mut count = 0;

            // Aggregate all results
            for result in reader.deserialize() {
                let record: Record = result.unwrap();
                let this_event = data
                    .get_mut(&baseline)
                    .unwrap()
                    .get_mut(record.event.as_str())
                    .unwrap();
                *this_event += record.time_ms as f64;

                count += 1;
            }

            // Calculate the average
            let num_reps = count / Containerd::CONTAINERD_INFO_EVENTS.len();
            let mut orchestration_time = 0.0;
            for (event, agg) in data.get_mut(&baseline).unwrap() {
                *agg /= num_reps as f64;

                if *event != "StartUp" {
                    orchestration_time += *agg;
                }

                // Keep track of the highest average
                if *agg > y_max {
                    y_max = *agg;
                }
            }

            // Add an additional event corresponding to "Orchestration" which
            // we define as StartUp - sum(AllOtherEvents)
            orchestration_time =
                data.get(&baseline).unwrap().get("StartUp").unwrap() - orchestration_time;
            data.get_mut(&baseline)
                .unwrap()
                .insert("Orchestration", orchestration_time);
        } // End processing one CSV file

        // ---------- Plot Data ---------- //

        for flavour in ["cold", "warm"] {
            let data = match flavour {
                "cold" => cold_data.clone(),
                "warm" => warm_data.clone(),
                _ => panic!("unreachable")
            };

            for (baseline, times) in data.iter() {
                for (event, avg) in times.iter() {
                    debug!("{baseline}/{flavour}/{event}: {avg} ms");
                }
            }
        }

        let mut plot_path = Env::results_root();
        plot_path.push(format!("{exp}"));
        plot_path.push("plots");
        fs::create_dir_all(plot_path.clone()).unwrap();
        plot_path.push(format!("{}.svg", exp.to_string().replace("-", "_")));

        let chart_height_px = 600;
        let chart_width_px = 400;
        let root =
            SVGBackend::new(&plot_path, (chart_height_px, chart_width_px)).into_drawing_area();
        root.fill(&WHITE).unwrap();

        let x_max = AvailableBaselines::iter_variants().len() as f64;
        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(40)
            .y_label_area_size(40)
            .margin(10)
            .margin_top(40)
            .build_cartesian_2d(0.0..x_max, 0f64..(y_max / 1000.0))
            .unwrap();

        chart
            .configure_mesh()
            .y_label_style(("sans-serif", 20).into_font())
            .y_labels(10)
            .y_max_light_lines(5)
            .disable_x_mesh()
            .disable_x_axis()
            .y_label_formatter(&|y| format!("{:.0}", y))
            .draw()
            .unwrap();

        // Manually draw the y-axis label with a custom font and size
        root.draw(&Text::new(
            "Start-Up Latency [s]",
            (3, 280),
            ("sans-serif", 20)
                .into_font()
                .transform(FontTransform::Rotate270)
                .color(&BLACK),
        ))
        .unwrap();

        let bar_width = 0.5;
        for (data_idx, data) in (0..).zip([cold_data.clone(), warm_data.clone()]) {
            // Draw bars: we draw one series for each event, and we stack them
            // together
            let mut prev_y_map: BTreeMap<&AvailableBaselines, f64> = BTreeMap::new();
            for baseline in AvailableBaselines::iter_variants() {
                prev_y_map.insert(baseline, 0.0);
            }

            for event in Containerd::CONTAINERD_INFO_EVENTS {
                chart
                    .draw_series((0..).zip(data.iter()).map(|(x, (baseline, event_vec))| {
                        let bar_style = ShapeStyle {
                            color: Containerd::get_color_for_event(event).into(),
                            filled: true,
                            stroke_width: 2,
                        };

                        // Handle the StartUp case separately
                        let mut this_y = *event_vec.get(event).unwrap();
                        if event == "StartUp" {
                            this_y = *event_vec.get("Orchestration").unwrap();
                        }
                        let prev_y = prev_y_map.get_mut(baseline).unwrap();
                        this_y /= 1000.0;

                        let x_orig: f64 = x as f64 + 0.5 * data_idx as f64;

                        let mut bar = Rectangle::new(
                            [(x_orig, *prev_y), (x_orig + bar_width, *prev_y + this_y)],
                            bar_style,
                        );
                        *prev_y += this_y;

                        bar.set_margin(0, 0, 2, 2);
                        bar
                    }))
                    .unwrap();
            }

            // Add black frame around each bar
            chart
                .draw_series((0..).zip(data.iter()).map(|(x, (baseline, _))| {
                    // Benefit from the fact that prev_y stores the maximum y
                    // value after we plot the stacked bar chart
                    let this_y = *prev_y_map.get_mut(baseline).unwrap();

                    let x_orig: f64 = x as f64 + 0.5 * data_idx as f64;
                    let margin_px = 2;
                    let x_axis_range = 0.0..x_max;
                    let margin_units = margin_px as f64 * (x_axis_range.end - x_axis_range.start)
                        / chart_width_px as f64;

                    PathElement::new(
                        vec![
                            (x_orig + margin_units, this_y),
                            (x_orig - margin_units + bar_width, this_y),
                            (x_orig - margin_units + bar_width, 0.0),
                            (x_orig + margin_units, 0.0),
                            (x_orig + margin_units, this_y),
                        ],
                        BLACK,
                    )
                }))
                .unwrap();
        }

        // Add solid frames around grid
        chart
            .plotting_area()
            .draw(&PathElement::new(vec![(0.0, y_max), (x_max, y_max)], BLACK))
            .unwrap();
        chart
            .plotting_area()
            .draw(&PathElement::new(
                vec![(x_max, 0 as f64), (x_max, y_max)],
                BLACK,
            ))
            .unwrap();
        chart
            .plotting_area()
            .draw(&PathElement::new(
                vec![(0.0, 0 as f64), (x_max, 0 as f64)],
                BLACK,
            ))
            .unwrap();

        // Manually draw the x-axis labels with a custom font and size
        fn xaxis_pos_for_baseline(baseline: &AvailableBaselines) -> i32 {
            match baseline {
                AvailableBaselines::Runc => 80,
                AvailableBaselines::Kata => 180,
                AvailableBaselines::Snp => 260,
                AvailableBaselines::SnpSc2 => 340,
                AvailableBaselines::Tdx => 445,
                AvailableBaselines::TdxSc2 => 520,
            }
        }

        for (_, baseline) in (0..).zip(AvailableBaselines::iter_variants()) {
            root.draw(&Text::new(
                format!("{baseline}"),
                (xaxis_pos_for_baseline(baseline), 360),
                ("sans-serif", 20).into_font().color(&BLACK),
            ))
            .unwrap();
        }

        // Manually draw the legend outside the grid, above the chart
        let legend_labels = vec!["control-plane", "create-vm", "pull-image-host", "pull-image-guest"];

        fn legend_pos_for_label(label: &str) -> (i32, i32) {
            let legend_x_start = 10;
            let legend_y_pos = 6;

            match label {
                "control-plane" => (legend_x_start, legend_y_pos),
                "create-vm" => (legend_x_start + 50, legend_y_pos),
                "pull-image-host" => (legend_x_start + 180, legend_y_pos),
                "pull-image-guest" => (legend_x_start + 280, legend_y_pos),
                _ => panic!("{}(plot): unrecognised label: {label}", Env::SYS_NAME),
            }
        }

        fn legend_color_for_label(label: &str) -> RGBColor {
            match label {
                "control-plane" => Containerd::get_color_for_event("StartUp"),
                "create-vm" => Containerd::get_color_for_event("RunPodSandbox"),
                "pull-image-host" => Containerd::get_color_for_event("PullImage"),
                "pull-image-guest" => Containerd::get_color_for_event("StartContainerUserContainer"),
                _ => panic!("{}(plot): unrecognised label: {label}", Env::SYS_NAME),
            }
        }

        for label in legend_labels {
            // Calculate position for each legend item
            let (x_pos, y_pos) = legend_pos_for_label(label);

            // Draw the color box (Rectangle)
            root.draw(&Rectangle::new(
                [(x_pos, y_pos), (x_pos + 20, y_pos + 20)],
                legend_color_for_label(label).filled(),
            ))
            .unwrap();

            // Draw the baseline label (Text)
            root.draw(&Text::new(
                label,
                (x_pos + 30, y_pos + 5),
                ("sans-serif", 20).into_font(),
            ))
            .unwrap();
        }

        println!(
            "{}(plot): generated plot at: {}",
            Env::SYS_NAME,
            plot_path.display()
        );
        root.present().unwrap();
    }

    pub fn plot(exp: &AvailableExperiments) {
        // First, get all the data files for the experiment
        let data_files = Self::get_all_data_files(exp);

        match exp {
            AvailableExperiments::ScaleOut => {
                panic!("not implemented :-(");
            }
            AvailableExperiments::StartUp => {
                Self::plot_start_up_latency(exp, &data_files);
            }
        }
    }
}
