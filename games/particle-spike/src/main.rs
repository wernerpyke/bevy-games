//! Stand-alone spike: can bevy_enoki draw the Long Way weapon effects, on native and on the web?
//! Rules and results: dev-docs/RESEARCH-particles.md, dev-docs/PLAN-particle-spike.md.

mod fireball;
mod glow;
mod harness;
mod ring;
mod rules;
mod wall;
mod whirlpool;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy_aseprite_ultra::AsepriteUltraPlugin;
use bevy_enoki::prelude::*;

/// Height of the view in art pixels, the same as Long Way (`camera.rs`), so effect sizes carry over.
const VIEW_HEIGHT: f32 = 512.0;
const WINDOW_SIZE: (u32, u32) = (1280, 720);
const BACKGROUND: Color = Color::srgb(0.10, 0.12, 0.16);

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "particle-spike".into(),
                        resolution: WINDOW_SIZE.into(),
                        // The benchmark (`SPIKE_BENCH`) needs the real frame time, not one capped by vsync.
                        present_mode: if std::env::var("SPIKE_BENCH").is_ok() {
                            bevy::window::PresentMode::AutoNoVsync
                        } else {
                            bevy::window::PresentMode::AutoVsync
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins((
            AsepriteUltraPlugin,
            EnokiPlugin,
            glow::GlowPlugin,
            harness::HarnessPlugin,
            fireball::FireballPlugin,
            ring::RingPlugin,
            wall::WallPlugin,
            whirlpool::WhirlpoolPlugin,
        ))
        .insert_resource(ClearColor(BACKGROUND))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // The stand-in for the player.
    commands.spawn((
        harness::Player,
        Sprite::from_color(Color::srgb(0.9, 0.9, 0.9), Vec2::new(32.0, 50.0)),
        Transform::from_translation(harness::PLAYER.extend(1.0)),
    ));
}
