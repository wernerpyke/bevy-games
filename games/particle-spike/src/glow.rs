//! Additive blending (step 7b). Enoki draws with alpha blending only, so the vendored copy of the crate
//! (`vendor/bevy_enoki`) has one change: `Particle2dMaterial::blend_state()`. The two materials here are the Enoki sprite
//! and colour materials with that method set to additive. G switches the next spawned effects (env `SPIKE_GLOW=1` starts on).

use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
};
use bevy::shader::ShaderRef;
use bevy_enoki::prelude::*;
use bevy_enoki::{PARTICLE_COLOR_FRAG, PARTICLE_SPRITE_FRAG};

pub struct GlowPlugin;

impl Plugin for GlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Particle2dMaterialPlugin::<AdditiveSprite>::default(),
            Particle2dMaterialPlugin::<AdditiveColor>::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle);
    }
}

/// Source alpha times source colour, added to what is already there.
fn additive() -> BlendState {
    BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent::OVER,
    }
}

/// `SpriteParticle2dMaterial` with additive blending (same shader and bindings).
#[derive(AsBindGroup, Asset, TypePath, Clone)]
pub struct AdditiveSprite {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
    #[uniform(2)]
    frame_data: UVec4,
}

impl AdditiveSprite {
    pub fn new(texture: Handle<Image>, hframes: u32) -> Self {
        Self {
            texture,
            frame_data: UVec4::new(hframes, 1, 0, 0),
        }
    }
}

impl Particle2dMaterial for AdditiveSprite {
    fn fragment_shader() -> ShaderRef {
        PARTICLE_SPRITE_FRAG.into()
    }
    fn blend_state() -> BlendState {
        additive()
    }
}

/// `ColorParticle2dMaterial` with additive blending.
#[derive(AsBindGroup, Asset, TypePath, Clone)]
pub struct AdditiveColor {
    #[uniform(0)]
    color: LinearRgba,
}

impl Particle2dMaterial for AdditiveColor {
    fn fragment_shader() -> ShaderRef {
        PARTICLE_COLOR_FRAG.into()
    }
    fn blend_state() -> BlendState {
        additive()
    }
}

#[derive(Resource)]
pub struct Glow {
    pub on: bool,
    color: Handle<AdditiveColor>,
}

fn setup(mut materials: ResMut<Assets<AdditiveColor>>, mut commands: Commands) {
    commands.insert_resource(Glow {
        on: std::env::var("SPIKE_GLOW").is_ok(),
        color: materials.add(AdditiveColor {
            color: LinearRgba::WHITE,
        }),
    });
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut glow: ResMut<Glow>) {
    if keys.just_pressed(KeyCode::KeyG) {
        glow.on = !glow.on;
        info!("additive blending: {}", glow.on);
    }
}

impl Glow {
    /// Gives the entity a plain colour-square spawner, additive or not.
    pub fn insert_color(&self, entity: &mut EntityCommands) {
        if self.on {
            entity.insert(ParticleSpawner(self.color.clone()));
        } else {
            entity.insert(ParticleSpawner::<ColorParticle2dMaterial>::default());
        }
    }
}

/// One sprite sheet as a normal and an additive material.
#[derive(Clone)]
pub struct SpriteLook {
    normal: Handle<SpriteParticle2dMaterial>,
    add: Handle<AdditiveSprite>,
}

impl SpriteLook {
    pub fn new(
        normals: &mut Assets<SpriteParticle2dMaterial>,
        adds: &mut Assets<AdditiveSprite>,
        texture: Handle<Image>,
        hframes: u32,
    ) -> Self {
        Self {
            normal: normals.add(SpriteParticle2dMaterial::new(texture.clone(), hframes, 1)),
            add: adds.add(AdditiveSprite::new(texture, hframes)),
        }
    }

    pub fn insert(&self, glow: &Glow, entity: &mut EntityCommands) {
        if glow.on {
            entity.insert(ParticleSpawner(self.add.clone()));
        } else {
            entity.insert(ParticleSpawner(self.normal.clone()));
        }
    }
}
