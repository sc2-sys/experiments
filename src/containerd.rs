use crate::env::Env;
use chrono::{DateTime, Utc};
use log::{debug, warn};
use plotters::prelude::RGBColor;
use regex::Regex;
use serde_json::Value;
use std::process::{Command, Stdio};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
};

#[derive(Debug)]
pub struct Containerd {}

impl Containerd {
    // TODO: consider making this typed, or at least using the same strings
    // below
    pub const CONTAINERD_INFO_EVENTS: [&'static str; 7] = [
        "StartUp", // Fake event that we add to measure end-to-end time
        "RunPodSandbox", // This event captures the time to start the sandbox
        "PullImage", // This event captures the time to pull an image in the host
        "CreateContainerUserContainer",
        "CreateContainerQueueProxy",
        "StartContainerUserContainer", // For CoCo: pull app image in the guest
        "StartContainerQueueProxy", // For CoCo: pull proxy image in the guest
    ];

    pub fn get_color_for_event(event: &str) -> RGBColor {
        match event {
            "StartUp" => RGBColor(102, 102, 255),
            "RunPodSandbox" => RGBColor(102, 255, 178),
            "PullImage" => RGBColor(245, 161, 66),
            "CreateContainerUserContainer" => RGBColor(255, 102, 178),
            "CreateContainerQueueProxy" => RGBColor(255, 102, 178),
            "StartContainerUserContainer" => RGBColor(255, 255, 102),
            "StartContainerQueueProxy" => RGBColor(255, 255, 102),
            _ => panic!("{}(containerd): unrecognised event: {event}", Env::SYS_NAME),
        }
    }

    /// Parse timestamp from journalctl's JSON __REALTIME_TIMESTAMP
    fn parse_timestamp(timestamp: &str) -> DateTime<Utc> {
        let timestamp: i64 = timestamp.parse().unwrap();
        let date_time_fixed = DateTime::from_timestamp_micros(timestamp)
            .ok_or("sc2-exp: invalid timestamp")
            .unwrap();
        date_time_fixed.with_timezone(&Utc)
    }

    /// Given a deployment id, return the timestamps for the RunPodSandbox
    /// and the two CreateContainer and StartContainer events.
    ///
    /// This method is meant to be executed _without_ debug logging, and, for
    /// the time being, has a hardcoded number of events to parse. If we need
    /// to add different types of parsing we may abstract parts of it away.
    ///
    /// Given that we may make measurements multiple times for each deployment
    /// id, we include a cutoff_time to discard entries prior to that timestamp.
    pub fn get_events_from_journalctl(
        deployment_id: &str,
        cutoff_time: &DateTime<Utc>,
    ) -> BTreeMap<String, (DateTime<Utc>, DateTime<Utc>)> {
        debug!(
            "{}(containerd): parsing journalctl logs for deployment: {deployment_id}",
            Env::SYS_NAME
        );

        // Load the journalctl output into a buffer reader
        let mut journalctl = Command::new("sudo")
            .args(["journalctl", "-xeu", "containerd", "-o", "json"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = journalctl
            .stdout
            .take()
            .ok_or("sc2-exp: failed to open journalctl stdout")
            .unwrap();
        let reader = BufReader::new(stdout);

        // Prepare the output map
        let mut ts_map: BTreeMap<String, (DateTime<Utc>, DateTime<Utc>)> = BTreeMap::new();

        // Helper start timestamps for different events
        let mut run_sandbox_start: Option<DateTime<Utc>> = None;
        let mut pull_image_start: Option<DateTime<Utc>> = None;
        let mut user_container_start: Option<DateTime<Utc>> = None;
        let mut queue_proxy_start: Option<DateTime<Utc>> = None;
        let mut user_container_create: Option<DateTime<Utc>> = None;
        let mut queue_proxy_create: Option<DateTime<Utc>> = None;

        // Sandbox and container ids
        let mut sbx_id = String::new();
        let mut user_container_id = String::new();
        let mut queue_proxy_container_id = String::new();

        // Regex expressions to get the sandbox/container ids
        let sandbox_id_regex =
            Regex::new(r#"returns sandbox id \\\"(?P<sbx_id>[a-fA-F0-9]+)\\\""#).unwrap();
        let container_id_regex =
            Regex::new(r#"returns container id \\\"(?P<ctr_id>[a-fA-F0-9]+)\\\""#).unwrap();

        // Parse JSON log entries line by line
        for line in reader.lines() {
            let line = line.unwrap();
            let json: Value = serde_json::from_str(&line).unwrap();

            // Extract the timestamp and message fields from JSON
            if let (Some(timestamp), Some(message)) =
                (json.get("__REALTIME_TIMESTAMP"), json.get("MESSAGE"))
            {
                let message = message.as_str().unwrap_or("");
                let timestamp = timestamp.as_str().unwrap_or("");
                let timestamp = Self::parse_timestamp(timestamp);

                // Skip log entries before the cutoff timestamp
                if timestamp < *cutoff_time {
                    continue;
                }

                // ---------- RunPodSandbox ----------

                if run_sandbox_start.is_none()
                    && message.contains("RunPodSandbox")
                    && message.contains(deployment_id)
                {
                    run_sandbox_start = Some(timestamp);
                    continue;
                }

                if message.contains("RunPodSandbox") && message.contains("returns sandbox id") {
                    if let Some(caps) = sandbox_id_regex.captures(message) {
                        sbx_id = caps.name("sbx_id").unwrap().as_str().to_string();
                        debug!("{}(containerd): got sandbox id: {sbx_id}", Env::SYS_NAME);
                        if let (Some(start), Some(end)) = (run_sandbox_start, Some(timestamp)) {
                            ts_map.insert("RunPodSandbox".to_string(), (start, end));
                        }
                        continue;
                    }
                }

                // ---------- PullImage ----------

                if pull_image_start.is_none()
                    && message.contains("PullImage")
                {
                    pull_image_start = Some(timestamp);
                    continue;
                }

                if message.contains("PullImage") && message.contains("returns image reference") && !pull_image_start.is_none() {
                    if let (Some(start), Some(end)) = (pull_image_start, Some(timestamp)) {
                        ts_map.insert("PullImage".to_string(), (start, end));
                    }
                    continue;
                }

                // ---------- CreateContainer ----------

                if !sbx_id.is_empty()
                    && message.contains("CreateContainer")
                    && message.contains(&sbx_id)
                {
                    // There are two CreateContainer events, one for the
                    // user-container and one for the queue-proxy
                    if message.contains("user-container") {
                        // Start timestamp for CreateContainer in user-container
                        if user_container_start.is_none() {
                            user_container_start = Some(timestamp);
                            continue;
                        }

                        // End timestamp and capture container ID
                        if message.contains("returns container id") {
                            if let Some(caps) = container_id_regex.captures(message) {
                                user_container_id =
                                    caps.name("ctr_id").unwrap().as_str().to_string();
                                debug!(
                                    "{}(containerd): got user container id: {user_container_id}",
                                    Env::SYS_NAME
                                );
                                ts_map.insert(
                                    "CreateContainerUserContainer".to_string(),
                                    (user_container_start.unwrap(), timestamp),
                                );
                                user_container_start = None;
                            }
                        }
                    } else if message.contains("queue-proxy") {
                        // Start timestamp for CreateContainer in queue-proxy
                        if queue_proxy_start.is_none() {
                            queue_proxy_start = Some(timestamp);
                            continue;
                        }

                        // End timestamp and capture container ID
                        if message.contains("returns container id") {
                            if let Some(caps) = container_id_regex.captures(message) {
                                queue_proxy_container_id =
                                    caps.name("ctr_id").unwrap().as_str().to_string();
                                debug!(
                                    "{}(containerd): got queue proxy id: {user_container_id}",
                                    Env::SYS_NAME
                                );
                                ts_map.insert(
                                    "CreateContainerQueueProxy".to_string(),
                                    (queue_proxy_start.unwrap(), timestamp),
                                );
                                queue_proxy_start = None;
                            }
                        }
                    }
                }

                // ---------- StartContainer ----------

                if message.contains("StartContainer") {
                    // There are two StartContainer events, one for the
                    // user-container and one for the queue-proxy
                    if message.contains(&user_container_id) {
                        // Start timestamp for StartContainer in user-container
                        if user_container_create.is_none() {
                            user_container_create = Some(timestamp);
                            continue;
                        }

                        // End timestamp for StartContainer in user-container
                        if message.contains("returns successfully") {
                            ts_map.insert(
                                "StartContainerUserContainer".to_string(),
                                (user_container_create.unwrap(), timestamp),
                            );
                            user_container_create = None;
                        }
                    } else if message.contains(&queue_proxy_container_id) {
                        // Start timestamp for StartContainer in queue-proxy
                        if queue_proxy_create.is_none() {
                            queue_proxy_create = Some(timestamp);
                            continue;
                        }

                        // End timestamp for StartContainer in queue-proxy
                        if message.contains("returns successfully") {
                            ts_map.insert(
                                "StartContainerQueueProxy".to_string(),
                                (queue_proxy_create.unwrap(), timestamp),
                            );
                            queue_proxy_create = None;
                        }
                    }
                }
            }
        }

        // Wait on the process to silent clippy warning
        journalctl
            .wait()
            .expect("Failed to wait on journalctl process");

        debug!(
            "{}(containerd): got a total of {} events",
            Env::SYS_NAME,
            ts_map.len()
        );
        let num_expected_events = 6;
        if ts_map.len() != num_expected_events {
            warn!("{}(containerd): expected {num_expected_events} journalctl events for '{deployment_id}' but got {}",
                  Env::SYS_NAME,
                  ts_map.len());
        }

        ts_map
    }
}
