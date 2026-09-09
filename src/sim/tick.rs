//! Hierarchical deterministic simulation ticks (design doc §4).
//!
//! | Layer        | Period |
//! |--------------|--------|
//! | neural       | 50 ms  |
//! | circulation  | 250 ms |
//! | endocrine    | 2 s    |
//! | growth       | 30 s   |

use bevy::prelude::*;

pub const NEURAL_DT: f64 = 0.050;
pub const CIRCULATION_DT: f64 = 0.250;
pub const ENDOCRINE_DT: f64 = 2.0;
pub const GROWTH_DT: f64 = 30.0;

#[derive(Resource, Debug)]
pub struct SimClock {
    pub neural_accum: f64,
    pub circulation_accum: f64,
    pub endocrine_accum: f64,
    pub growth_accum: f64,
    pub neural_steps: u64,
    pub circulation_steps: u64,
    pub endocrine_steps: u64,
    pub growth_steps: u64,
}

impl Default for SimClock {
    fn default() -> Self {
        Self {
            neural_accum: 0.0,
            circulation_accum: 0.0,
            endocrine_accum: 0.0,
            growth_accum: 0.0,
            neural_steps: 0,
            circulation_steps: 0,
            endocrine_steps: 0,
            growth_steps: 0,
        }
    }
}

pub struct SimTickPlugin;

impl Plugin for SimTickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimClock>()
            .add_systems(Startup, log_tick_schedule)
            .add_systems(Update, advance_hierarchical_ticks);
    }
}

fn log_tick_schedule() {
    info!(
        "ROSC sim ticks armed: neural={}ms circulation={}ms endocrine={}s growth={}s",
        (NEURAL_DT * 1000.0) as u32,
        (CIRCULATION_DT * 1000.0) as u32,
        ENDOCRINE_DT,
        GROWTH_DT
    );
}

fn advance_hierarchical_ticks(time: Res<Time>, mut clock: ResMut<SimClock>) {
    // Use f64 seconds; Bevy Time is the wall/app clock. Later: fixed-step only.
    let dt = time.delta_secs_f64();
    clock.neural_accum += dt;
    clock.circulation_accum += dt;
    clock.endocrine_accum += dt;
    clock.growth_accum += dt;

    while clock.neural_accum >= NEURAL_DT {
        clock.neural_accum -= NEURAL_DT;
        clock.neural_steps += 1;
        // Phase 0: no neural work.
    }
    while clock.circulation_accum >= CIRCULATION_DT {
        clock.circulation_accum -= CIRCULATION_DT;
        clock.circulation_steps += 1;
        // Phase 1: call flow solve when a live graph resource exists.
    }
    while clock.endocrine_accum >= ENDOCRINE_DT {
        clock.endocrine_accum -= ENDOCRINE_DT;
        clock.endocrine_steps += 1;
    }
    while clock.growth_accum >= GROWTH_DT {
        clock.growth_accum -= GROWTH_DT;
        clock.growth_steps += 1;
    }
}
