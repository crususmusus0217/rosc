use bevy::prelude::*;
use bevy::window::WindowResolution;
use rosc::sim::tick::SimTickPlugin;
use rosc::ui::view::PerfusionViewPlugin;

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
        .add_plugins(PerfusionViewPlugin)
        .run();
}
