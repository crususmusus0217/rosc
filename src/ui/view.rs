//! Phase 1 perfusion view: gizmos for the loop + UI text HUD.

use bevy::math::Isometry2d;
use bevy::prelude::*;
use crate::sim::circuit::{CircuitStatus, LiveCircuit};
use crate::sim::tick::SimClock;

#[derive(Resource)]
pub struct CircuitState(pub LiveCircuit);

#[derive(Component)]
pub struct HudText;

pub struct PerfusionViewPlugin;

impl Plugin for PerfusionViewPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CircuitState(LiveCircuit::closed_loop_demo()))
            .add_systems(Startup, setup_view)
            .add_systems(
                Update,
                (
                    step_circuit_on_circulation_ticks,
                    draw_circuit_gizmos,
                    update_hud,
                    debug_refill_atp,
                ),
            );
    }
}

fn setup_view(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("ROSC Phase 1"),
        TextFont::from_font_size(18.0),
        TextColor(Color::srgb(0.9, 0.95, 1.0)),
        TextLayout::justify(Justify::Left),
        Transform::from_xyz(-460.0, 250.0, 10.0),
        HudText,
    ));

    info!("Phase 1 perfusion view ready. Press R to refill ATP after failure.");
}

fn step_circuit_on_circulation_ticks(
    clock: Res<SimClock>,
    mut last: Local<u64>,
    mut circuit: ResMut<CircuitState>,
) {
    if clock.circulation_steps == *last {
        return;
    }
    // Catch up if multiple ticks were processed in one frame.
    let n = clock.circulation_steps.saturating_sub(*last);
    *last = clock.circulation_steps;
    for _ in 0..n {
        circuit.0.circulation_step();
    }
}

fn o2_to_color(o2: f64, failed: bool) -> Color {
    if failed {
        return Color::srgb(0.15, 0.15, 0.18);
    }
    // Low O₂ → blue, high O₂ → warm red/orange.
    let t = o2.clamp(0.0, 1.0) as f32;
    Color::srgb(0.15 + 0.85 * t, 0.25 + 0.2 * (1.0 - t), 0.85 - 0.55 * t)
}

fn draw_circuit_gizmos(mut gizmos: Gizmos, circuit: Res<CircuitState>) {
    let c = &circuit.0;
    let failed = c.status == CircuitStatus::Failed;
    let base = o2_to_color(c.pools.o2, failed);

    let positions = [
        c.node_pos(c.heart),
        c.node_pos(c.capillary),
        c.node_pos(c.vein),
    ];

    let edges = [
        (c.edge_art, c.heart, c.capillary),
        (c.edge_ven, c.capillary, c.vein),
        (c.edge_return, c.vein, c.heart),
    ];

    let max_q = edges
        .iter()
        .filter_map(|(eid, _, _)| {
            c.solution
                .as_ref()
                .map(|s| crate::sim::flow::edge_flow(s, *eid).abs())
        })
        .fold(1e-12_f64, f64::max);

    for (eid, from, to) in edges {
        let (x0, y0) = c.node_pos(from);
        let (x1, y1) = c.node_pos(to);
        let a = Vec2::new(x0, y0);
        let b = Vec2::new(x1, y1);
        let q = c
            .solution
            .as_ref()
            .map(|s| crate::sim::flow::edge_flow(s, eid).abs())
            .unwrap_or(0.0);
        let thick = 1.0 + 8.0 * (q / max_q) as f32;
        // Approximate thickness with parallel strokes (gizmo lines are 1px).
        let dir = (b - a).normalize_or_zero();
        let n = Vec2::new(-dir.y, dir.x);
        let layers = thick.round().clamp(1.0, 10.0) as i32;
        for i in -layers..=layers {
            let o = n * (i as f32) * 0.6;
            gizmos.line_2d(a + o, b + o, base);
        }
    }

    for (i, (x, y)) in positions.iter().enumerate() {
        let radius = if i == c.heart.0 { 22.0 } else { 16.0 };
        let color = if i == c.heart.0 && !failed {
            Color::srgb(0.95, 0.35, 0.35)
        } else {
            base
        };
        gizmos.circle_2d(Isometry2d::from_translation(Vec2::new(*x, *y)), radius, color);
    }

}

fn update_hud(circuit: Res<CircuitState>, clock: Res<SimClock>, mut q: Query<&mut Text2d, With<HudText>>) {
    let Ok(mut text) = q.single_mut() else {
        return;
    };
    let c = &circuit.0;
    let status = match c.status {
        CircuitStatus::Pumping => "PUMPING",
        CircuitStatus::Failed => "ROSC FAILED — press R to refill ATP",
    };
    let q_art = c.art_flow_abs();
    text.0 = format!(
        "ROSC Phase 1 — closed loop\n\
         status: {status}\n\
         ATP: {:5.1} / {:.0}\n\
         O2 : {:5.2}\n\
         |Q_art|: {:.3e}\n\
         circ ticks: {}\n\
         mass imbalance: {:.2e}\n\
         [R] refill ATP",
        c.pools.atp,
        c.pools.atp_max,
        c.pools.o2,
        q_art,
        clock.circulation_steps,
        c.mass_imbalance(),
    );
}

fn debug_refill_atp(keys: Res<ButtonInput<KeyCode>>, mut circuit: ResMut<CircuitState>) {
    if keys.just_pressed(KeyCode::KeyR) {
        circuit.0.refill_atp();
        info!("ATP refilled / pump restarted");
    }
}
