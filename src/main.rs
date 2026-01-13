mod executor;
mod radio;
mod scheduler;
mod tracker;

use clap::{Parser, Subcommand};
use scheduler::{Command, Schedule};
use std::fs;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "satomat")]
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

    for (i, step) in schedule.steps.iter().enumerate() {
        // Handle timing
        if let Some(ref time_expr) = step.time {
            let target = time_expr.resolve(start_time);
            let now = chrono::Utc::now();
            if target > now {
                let wait = (target - now).to_std().unwrap_or_default();
                println!("  Waiting {:?} until {}", wait, target);
                std::thread::sleep(wait);
            }
        }

        println!("[Step {}] {}", i + 1, command_name(&step.command));

        // Execute command
        match &step.command {
            Command::Tracker(tracker::Command::Initialize { rotator, tle, .. }) => {
                println!(
                    "  -> tracker.initialize: rotator={}, tle={}...",
                    rotator,
                    &tle[..20.min(tle.len())]
                );
            }
            Command::Tracker(tracker::Command::RotatorPark { rotator }) => {
                println!("  -> tracker.rotator.park: rotator={}", rotator);
            }
            Command::Executor(executor::Command::RunShell { cmd, .. }) => {
                println!("  -> executor.run_shell: cmd={}", cmd);
            }
            Command::Radio(radio::Command::Run {
                radio, bandwidth, ..
            }) => {
                println!("  -> radio.run: radio={}, bandwidth={}", radio, bandwidth);
            }
        }
    }

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
