use chrono::{DateTime, Duration, Utc};
use sgp4::{Constants, Elements};
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use std::thread;

use super::error::TrackerError;
use super::parsing::parse_tle_lines;
use super::types::RadioConfig;
use crate::predict::{build_frequency_plan, predict_trajectory, GroundStation, Sample};
use serde::Serialize;

const DEFAULT_OPEN_ENDED: Duration = Duration::minutes(15);
const STEP: Duration = Duration::seconds(1);

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub enum TrackerMode {
    Idle,
    Running {
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
        tle_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TrackerStatus {
    pub mode: TrackerMode,
    pub last_sample: Option<Sample>,
    pub trajectory: Vec<Sample>,
}

#[derive(Debug)]
struct Shared {
    status: TrackerStatus,
}

#[derive(Debug)]
struct WorkerHandle {
    stop_tx: mpsc::Sender<()>,
    join: thread::JoinHandle<Result<(), TrackerError>>,
}

pub struct Tracker {
    station: GroundStation,
    shared: Arc<StdMutex<Shared>>,
    worker: Option<WorkerHandle>,
}

impl Tracker {
    pub fn new(station: GroundStation) -> Self {
        Self {
            station,
            shared: Arc::new(StdMutex::new(Shared {
                status: TrackerStatus {
                    mode: TrackerMode::Idle,
                    last_sample: None,
                    trajectory: Vec::new(),
                },
            })),
            worker: None,
        }
    }

    /// Execute a tracker command
    pub fn execute_command(&mut self, cmd: &super::types::Command) -> Result<(), TrackerError> {
        log::debug!("execute command {cmd:?}");
        match cmd {
            super::types::Command::Run(r) => {
                self.run(r.tle.clone(), r.end, r.radio.clone())?;
            }
            super::types::Command::Stop => {
                self.stop();
            }
            crate::tracker::Command::RotatorPark { .. } => todo!(),
        }
        Ok(())
    }

    pub fn status(&self) -> TrackerStatus {
        self.shared.lock().unwrap().status.clone()
    }

    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            log::debug!("sending stop signal to worker thread");
            let _ = worker.stop_tx.send(());
            let _ = worker.join.join();
            log::debug!("worker thread joined");
        }

        let mut locked = self.shared.lock().unwrap();
        locked.status.mode = TrackerMode::Idle;
    }

    fn run(
        &mut self,
        tle: String,
        end: Option<DateTime<Utc>>,
        radio: Option<RadioConfig>,
    ) -> Result<(), TrackerError> {
        if self.worker.is_some() {
            log::warn!("worker already exists");
            return Err(TrackerError::AlreadyRunning);
        }

        let shared = self.shared.clone();
        let station = self.station;
        let (stop_tx, stop_rx) = mpsc::channel();

        let join = thread::spawn(move || {
            let result = run_tracker_loop(shared.clone(), station, tle, end, radio, stop_rx);

            if result.is_err() {
                log::error!("thread returned error {result:?}",);
                let mut locked = shared.lock().unwrap();
                locked.status.mode = TrackerMode::Idle;
                locked.status.last_sample = None;
                locked.status.trajectory.clear();
            } else {
                log::info!("thread exited successfully");
            }

            result
        });

        self.worker = Some(WorkerHandle { stop_tx, join });

        {
            let mut locked = self.shared.lock().unwrap();
            locked.status.mode = TrackerMode::Running {
                start: Utc::now(),
                end,
                tle_name: None,
            };
        }

        Ok(())
    }
}

fn run_tracker_loop(
    shared: Arc<StdMutex<Shared>>,
    station: GroundStation,
    tle: String,
    end: Option<DateTime<Utc>>,
    radio: Option<RadioConfig>,
    stop_rx: mpsc::Receiver<()>,
) -> Result<(), TrackerError> {
    log::info!("tracker thread starting, end={end:?}",);

    // Prepare objects used for trajectory calculation
    let (name, line1, line2) = parse_tle_lines(&tle)?;
    let elements = Elements::from_tle(name, line1.as_bytes(), line2.as_bytes())?;
    let constants = Constants::from_elements(&elements)?;
    let frequencies = radio
        .as_ref()
        .map(|r| {
            build_frequency_plan(
                Some(r.frequencies.uplink.clone()),
                Some(r.frequencies.downlink.clone()),
            )
        })
        .unwrap_or_else(|| build_frequency_plan(None, None));

    // Update tracker status with the object we are tracking.
    {
        let mut locked = shared.lock().unwrap();
        locked.status.mode = TrackerMode::Running {
            start: Utc::now(),
            end,
            tle_name: elements.object_name.clone(),
        };
        log::info!("thread running")
    }

    loop {
        // Calculate the target's trajectory for the next time window
        let window_start = Utc::now();
        let window_end = end.unwrap_or(window_start + DEFAULT_OPEN_ENDED);

        log::debug!("computing trajectory from {window_start} to {window_end}",);
        let trajectory = predict_trajectory(
            &station,
            &elements,
            &constants,
            window_start,
            window_end,
            &frequencies,
            STEP,
        )?;
        log::debug!("trajectory computed: {} points", trajectory.len());

        // Update status, make trajectory visible to other consumers
        {
            let mut locked = shared.lock().unwrap();
            locked.status.trajectory = trajectory.clone();
            locked.status.last_sample = None;
        }

        for point in trajectory {
            // Wait until the next point in the target's trajectory
            let now = Utc::now();
            let sleep_duration = if point.timestamp > now {
                (point.timestamp - now)
                    .to_std()
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            } else {
                std::time::Duration::from_secs(0)
            };

            // Have we received a stop signal?
            let should_stop = match stop_rx.recv_timeout(sleep_duration) {
                // Yes
                Ok(()) => true,
                Err(mpsc::RecvTimeoutError::Disconnected) => true,
                // No
                Err(mpsc::RecvTimeoutError::Timeout) => false,
            };
            // We have received a stop signal. Reset the tracker status and return.
            if should_stop {
                log::info!("received stop signal, exiting");
                let mut locked = shared.lock().unwrap();
                locked.status.mode = TrackerMode::Idle;
                return Ok(());
            }

            // Update the current position (sample) of the target in the shared status
            let mut locked = shared.lock().unwrap();
            locked.status.last_sample = Some(point.clone());
        }

        // If we have reached the end time, break out of the loop
        if end.is_some() {
            break;
        }
    }

    // Reset tracker status
    log::info!("loop exited normally");
    let mut locked = shared.lock().unwrap();
    locked.status.mode = TrackerMode::Idle;
    locked.status.last_sample = None;
    locked.status.trajectory.clear();
    Ok(())
}
