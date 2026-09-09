use bevy::prelude::*;
use bevy::window::WindowResolution;
use rosc::sim::tick::SimTickPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "ROSC".into(),
                // Bevy 0.19+: physical pixels as u32 (no f32 dots).
                resolution: WindowResolution::new(960, 540),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(SimTickPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
    info!("ROSC Phase 0 scaffold — empty scene, ticks running. See README.");
}
