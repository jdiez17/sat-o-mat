use chrono::{DateTime, Duration, Utc};
use sgp4::{Constants, Elements};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};

use super::error::TrackerError;
use super::ground_station::GroundStation;
use super::parsing::parse_tle_lines;
use super::sample::TrackerSample;
use super::trajectory::{build_frequency_plan, build_trajectory};
use super::types::RadioConfig;
use serde::Serialize;

const DEFAULT_OPEN_ENDED: Duration = Duration::minutes(15);
const STEP: Duration = Duration::seconds(1);

#[derive(Clone)]
pub struct FrequencyPlan {
    pub uplink_hz: Option<f64>,
    pub downlink_hz: Option<f64>,
}

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
    pub last_sample: Option<TrackerSample>,
    pub trajectory: Vec<TrackerSample>,
}

#[derive(Debug)]
struct Shared {
    status: TrackerStatus,
}

#[derive(Debug)]
struct WorkerHandle {
    stop_tx: oneshot::Sender<()>,
    join: JoinHandle<Result<(), TrackerError>>,
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

    pub fn status(&self) -> TrackerStatus {
        self.shared.lock().unwrap().status.clone()
    }

    pub async fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.stop_tx.send(());
            let _ = worker.join.await;
        }
        let mut locked = self.shared.lock().unwrap();
        locked.status.mode = TrackerMode::Idle;
    }

    pub async fn run(
        &mut self,
        tle: String,
        end: Option<DateTime<Utc>>,
        radio: Option<RadioConfig>,
    ) -> Result<(), TrackerError> {
        if self.worker.is_some() {
            return Err(TrackerError::AlreadyRunning);
        }

        let shared = self.shared.clone();
        let station = self.station;
        let (stop_tx, stop_rx) = oneshot::channel();

        let join = tokio::spawn(async move {
            let result = run_tracker_loop(shared.clone(), station, tle, end, radio, stop_rx).await;

            if result.is_err() {
                let mut locked = shared.lock().unwrap();
                locked.status.mode = TrackerMode::Idle;
                locked.status.last_sample = None;
                locked.status.trajectory.clear();
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

async fn run_tracker_loop(
    shared: Arc<StdMutex<Shared>>,
    station: GroundStation,
    tle: String,
    end: Option<DateTime<Utc>>,
    radio: Option<RadioConfig>,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<(), TrackerError> {
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

    {
        let mut locked = shared.lock().unwrap();
        locked.status.mode = TrackerMode::Running {
            start: Utc::now(),
            end,
            tle_name: elements.object_name.clone(),
        };
    }

    loop {
        let window_start = Utc::now();
        let window_end = end.unwrap_or(window_start + DEFAULT_OPEN_ENDED);
        let trajectory = build_trajectory(
            &station,
            &elements,
            &constants,
            window_start,
            window_end,
            &frequencies,
            STEP,
        )?;

        {
            let mut locked = shared.lock().unwrap();
            locked.status.trajectory = trajectory.clone();
            locked.status.last_sample = None;
        }

        for point in trajectory {
            let now = Utc::now();
            let sleep_duration = if point.timestamp > now {
                (point.timestamp - now)
                    .to_std()
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            } else {
                std::time::Duration::from_secs(0)
            };

            let should_stop = tokio::select! {
                _ = sleep_until(Instant::now() + sleep_duration) => false,
                _ = &mut stop_rx => true,
            };
            if should_stop {
                let mut locked = shared.lock().unwrap();
                locked.status.mode = TrackerMode::Idle;
                return Ok(());
            }

            let mut locked = shared.lock().unwrap();
            locked.status.last_sample = Some(point.clone());
        }

        if end.is_some() {
            break;
        }
    }

    let mut locked = shared.lock().unwrap();
    locked.status.mode = TrackerMode::Idle;
    Ok(())
}
