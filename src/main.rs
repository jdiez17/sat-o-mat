pub mod abort;
mod executor;
mod predict;
mod radio;
mod scheduler;
mod tracker;
mod web;

use clap::{Parser, Subcommand};
use scheduler::{Command, Schedule};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::predict::GroundStation;
use crate::tracker::Tracker;

#[derive(Parser)]
#[command(name = "sat-o-mat")]
#[command(about = "Satellite ground station control")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a schedule file
    Validate { schedule: String },
    /// Execute a schedule file
    Run { schedule: String },
    /// Start the web API server
    Serve {
        /// Path to configuration file
        #[arg(short, long)]
        config: String,
    },
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { schedule } => validate(&schedule),
        Commands::Run { schedule } => run(&schedule),
        Commands::Serve { config } => serve(&config),
    }
}

fn validate(path: &str) -> ExitCode {
    let yaml = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::FAILURE;
        }
    };

    match Schedule::from_str(&yaml) {
        Ok(schedule) => {
            println!("Schedule is valid ({} steps)", schedule.steps.len());
            for (i, step) in schedule.steps.iter().enumerate() {
                let time_str = match &step.time {
                    Some(t) => format!("{:?}", t),
                    None => "immediate".to_string(),
                };
                println!(
                    "  {}: {} @ {}",
                    i + 1,
                    command_name(&step.command),
                    time_str
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(path: &str) -> ExitCode {
    let yaml = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let schedule = match Schedule::from_str(&yaml) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let start_time = chrono::Utc::now();
    println!("Starting schedule at {}", start_time);

    let base_dir = PathBuf::from("sat-o-mat-data");
    let schedule_id = format!("manual_{}", start_time.format("%Y%m%dT%H%M%SZ"));

    let artifacts_dir = base_dir.join("artifacts").join(&schedule_id);
    if let Err(e) = fs::create_dir_all(&artifacts_dir) {
        eprintln!("Failed to create artifacts directory: {}", e);
        return ExitCode::FAILURE;
    }

    let tracker = Arc::new(Mutex::new(Tracker::new(GroundStation::default())));

    let runner = match scheduler::runner::Runner::new(
        schedule_id.clone(),
        schedule,
        tracker,
        base_dir.clone(),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create runner: {}", e);
            return ExitCode::FAILURE;
        }
    };

    match runner.run() {
        Ok(artifacts) => {
            println!("Schedule completed successfully");
            println!(
                "Execution log saved to: {}/artifacts/{}/execution.yaml",
                base_dir.display(),
                schedule_id
            );
            println!("State: {}", artifacts.execution_log().state);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Schedule execution failed: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Tracker(tracker::Command::RotatorPark { .. }) => "tracker.rotator_park",
        Command::Tracker(tracker::Command::Stop) => "tracker.stop",
        Command::Tracker(tracker::Command::Run(..)) => "tracker.run",
        Command::Executor(executor::Command::RunShell { .. }) => "executor.run_shell",
        Command::Executor(executor::Command::Stop) => "executor.stop",
        Command::Radio(radio::Command::Run { .. }) => "radio.run",
        Command::Radio(radio::Command::Stop) => "radio.stop",
    }
}

fn serve(config_path: &str) -> ExitCode {
    let config = match web::Config::from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Create tokio runtime and run the server
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    if let Err(e) = rt.block_on(web::run_server(config)) {
        eprintln!("Server error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
