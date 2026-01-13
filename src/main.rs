mod executor;
mod radio;
mod scheduler;
mod tracker;

use clap::{Parser, Subcommand};
use scheduler::{Command, Schedule};
use std::fs;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use crate::executor::Executor;
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { schedule } => validate(&schedule),
        Commands::Run { schedule } => run(&schedule),
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

    let executor = Executor::new();
    let tracker = Arc::new(Mutex::new(Tracker::new()));

    let runner = scheduler::runner::Runner {
        schedule,
        executor,
        tracker,
    };

    let _ = runner.run();

    println!("Schedule completed");
    ExitCode::SUCCESS
}

fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Tracker(tracker::Command::Initialize { .. }) => "tracker.initialize",
        Command::Tracker(tracker::Command::RotatorPark { .. }) => "tracker.rotator_park",
        Command::Executor(executor::Command::RunShell { .. }) => "executor.run_shell",
        Command::Radio(radio::Command::Run { .. }) => "radio.run",
    }
}
