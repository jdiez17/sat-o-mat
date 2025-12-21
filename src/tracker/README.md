# `tracker` module

## Rust API

Brainstorming (feel free to improve):

```rust
fn calculate_trajectory(start: Time, end: Time, object: EarthSatellite) -> Vec<(Time, ECEFCoordinates)>
fn calculate_tracking(trajectory: Trajectory, station_position: EarthPosition) -> Vec<(Time, TrackingInformation)>
fn calculate_doppler(trajectory: Trajectory, station_position: EarthPosition, frequencies_hz: BTreeMap<String, u32>) -> BTreeMap<String, Vec<(Time, u32)>>

fn rotator_control(input: Vec<(Time, TrackingInformation)>, commands: mpsc::Receiver<Command>) -> Result<(), RotatorControlError>
fn radio_control(input: BTreeMap<String, Vec<(Time, u32)>>) -> Result<(), RadioControlError>
```

The `calculate_*` functions are your basic astrodynamics functions for tracking an object in space. I would like to use `lox-rs` for this.
We should support TLEs and CCSDS OEM/ODMs as appropriate (but we start simple).

# Requirements

We want to have multiple rotator / radio control options but we will start with Hamlib as it already covers a lot of the needed functionality to interface with the hardware / software modems.

# Thoughts for improvement

You could argue that `rotator_control` and `radio_control` should be separate modules under "hardware control" and the other functions are the trajectory calculations.
