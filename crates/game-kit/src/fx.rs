//! Particle effects with `bevy_enoki`: the plugin, additive ("glow") materials, helpers that spawn an emitter, a
//! one-shot burst or an emitter that follows an entity, and a library that keeps effect files loaded.
//! The kit holds no effect files and no values. A game that wants particles adds `FxPlugin`; R-Town does not, so it
//! runs no Enoki code (`dev-docs/extensions/particles.md`).
//!
//! Enoki 0.8.0 draws with alpha blending only. The vendored copy in `vendor/bevy_enoki` adds
//! `Particle2dMaterial::blend_state()`, which the two additive materials here use.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
};
use bevy::shader::ShaderRef;
use bevy_enoki::prelude::*;
use bevy_enoki::{PARTICLE_COLOR_FRAG, PARTICLE_SPRITE_FRAG};

pub use crate::fx_strip::FxLooks;
pub use bevy_enoki::prelude::{
    OneShot, Particle2dEffect, ParticleEffectHandle, ParticleSpawnerState,
};

/// Adds Enoki, the two additive materials, the `EffectLibrary` and the system that moves `Follows` emitters.
/// Run the follow system `.after` your own movement, or let it lag one frame: it reads the owner's `Transform`.
pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            EnokiPlugin,
            Particle2dMaterialPlugin::<AdditiveSprite>::default(),
            Particle2dMaterialPlugin::<AdditiveColor>::default(),
        ))
        .init_resource::<EffectLibrary>()
        .init_resource::<FxLooks>()
        .add_systems(Startup, setup_materials)
        .add_systems(Update, (emitters_follow, crate::fx_strip::build_looks));
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

/// The shared white additive colour material, made at start-up.
#[derive(Resource)]
pub struct FxMaterials {
    additive_color: Handle<AdditiveColor>,
}

fn setup_materials(mut materials: ResMut<Assets<AdditiveColor>>, mut commands: Commands) {
    commands.insert_resource(FxMaterials {
        additive_color: materials.add(AdditiveColor {
            color: LinearRgba::WHITE,
        }),
    });
}

/// One sprite sheet (a horizontal strip of `hframes` frames) as a normal and an additive material.
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
}

/// What the particles of an emitter look like.
#[derive(Clone, Default)]
pub enum Look {
    /// Flat colour squares (the Enoki default).
    #[default]
    Squares,
    /// A sprite sheet.
    Sprites(SpriteLook),
}

/// Keeps effect files loaded for the whole run. Without it a file is dropped when its last emitter dies, and the next
/// emitter loads it again.
#[derive(Resource, Default)]
pub struct EffectLibrary(Vec<Handle<Particle2dEffect>>);

impl EffectLibrary {
    /// Loads `file` and keeps the handle.
    pub fn keep(&mut self, server: &AssetServer, file: &str) {
        self.0.push(server.load(file.to_owned()));
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Spawns emitters. A system takes it as a parameter: `mut fx: Fx`.
#[derive(SystemParam)]
pub struct Fx<'w, 's> {
    pub commands: Commands<'w, 's>,
    server: Res<'w, AssetServer>,
    materials: Res<'w, FxMaterials>,
}

impl Fx<'_, '_> {
    /// Spawns an emitter from an effect file. `glow` draws its particles with additive blending. The emitter keeps
    /// spawning until it is despawned or its `ParticleSpawnerState` is set inactive.
    pub fn emitter(&mut self, file: &str, transform: Transform, look: &Look, glow: bool) -> Entity {
        let mut entity = self.commands.spawn((
            ParticleEffectHandle(self.server.load(file.to_owned())),
            transform,
        ));
        match (look, glow) {
            (Look::Squares, true) => {
                entity.insert(ParticleSpawner(self.materials.additive_color.clone()));
            }
            (Look::Squares, false) => {
                entity.insert(ParticleSpawner::<ColorParticle2dMaterial>::default());
            }
            (Look::Sprites(sprites), true) => {
                entity.insert(ParticleSpawner(sprites.add.clone()));
            }
            (Look::Sprites(sprites), false) => {
                entity.insert(ParticleSpawner(sprites.normal.clone()));
            }
        }
        entity.id()
    }

    /// Spawns an emitter that plays its burst once and removes itself after the last particle.
    pub fn one_shot(
        &mut self,
        file: &str,
        transform: Transform,
        look: &Look,
        glow: bool,
    ) -> Entity {
        let emitter = self.emitter(file, transform, look, glow);
        self.commands.entity(emitter).insert(OneShot::Despawn);
        emitter
    }

    /// Spawns an emitter that follows `owner` (see `Follows`).
    pub fn following(
        &mut self,
        owner: Entity,
        file: &str,
        transform: Transform,
        look: &Look,
        glow: bool,
    ) -> Entity {
        let emitter = self.emitter(file, transform, look, glow);
        self.commands.entity(emitter).insert(Follows::new(owner));
        emitter
    }
}

/// An emitter that moves with its `owner`: it takes the owner's x and y plus `offset`, and the owner's rotation
/// turned by `rotation`. It leaves z alone, so a game gives the emitter its own draw order (`DepthSort`). When the
/// owner is gone the emitter stops spawning and removes itself after its last particle has died.
#[derive(Component, Clone, Copy, Debug)]
pub struct Follows {
    pub owner: Entity,
    pub offset: Vec2,
    pub rotation: Quat,
}

impl Follows {
    pub fn new(owner: Entity) -> Self {
        Self {
            owner,
            offset: Vec2::ZERO,
            rotation: Quat::IDENTITY,
        }
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }
}

fn emitters_follow(
    mut emitters: Query<(Entity, &Follows, &mut Transform, &mut ParticleSpawnerState)>,
    owners: Query<&Transform, Without<Follows>>,
    mut commands: Commands,
) {
    for (entity, follows, mut transform, mut state) in &mut emitters {
        if let Ok(owner) = owners.get(follows.owner) {
            let at = owner.translation.truncate() + follows.offset;
            transform.translation.x = at.x;
            transform.translation.y = at.y;
            transform.rotation = owner.rotation * follows.rotation;
        } else {
            state.active = false;
            commands
                .entity(entity)
                .remove::<Follows>()
                .insert(OneShot::Despawn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blend_adds_to_what_is_already_drawn() {
        let blend = additive();
        assert_eq!(blend.color.dst_factor, BlendFactor::One);
        assert_eq!(blend.color.src_factor, BlendFactor::SrcAlpha);
        assert_eq!(blend.color.operation, BlendOperation::Add);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, emitters_follow);
        app
    }

    fn emitter_of(app: &mut App, follows: impl FnOnce(Entity) -> Follows, owner: Entity) -> Entity {
        app.world_mut()
            .spawn((
                follows(owner),
                Transform::from_xyz(0.0, 0.0, 7.0),
                ParticleSpawnerState::default(),
            ))
            .id()
    }

    #[test]
    fn an_emitter_takes_the_owner_position_and_keeps_its_own_z() {
        let mut app = app();
        let owner = app
            .world_mut()
            .spawn(Transform::from_xyz(30.0, -12.0, 1.0))
            .id();
        let emitter = emitter_of(
            &mut app,
            |o| Follows::new(o).with_offset(Vec2::new(2.0, 3.0)),
            owner,
        );
        app.update();
        let at = app.world().get::<Transform>(emitter).unwrap().translation;
        assert_eq!(at, Vec3::new(32.0, -9.0, 7.0));
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation
            .x = 50.0;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(emitter).unwrap().translation.x,
            52.0
        );
    }

    #[test]
    fn an_emitter_turns_with_the_owner_plus_its_own_rotation() {
        let mut app = app();
        let owner = app
            .world_mut()
            .spawn(Transform::from_rotation(Quat::from_rotation_z(1.0)))
            .id();
        let emitter = emitter_of(
            &mut app,
            |o| Follows::new(o).with_rotation(Quat::from_rotation_z(0.5)),
            owner,
        );
        app.update();
        let angle = app
            .world()
            .get::<Transform>(emitter)
            .unwrap()
            .rotation
            .to_euler(EulerRot::XYZ)
            .2;
        assert!((angle - 1.5).abs() < 1e-5, "{angle}");
    }

    #[test]
    fn without_its_owner_an_emitter_stops_and_is_set_to_despawn() {
        let mut app = app();
        let owner = app.world_mut().spawn(Transform::default()).id();
        let emitter = emitter_of(&mut app, Follows::new, owner);
        app.update();
        assert!(app.world().get::<Follows>(emitter).is_some());
        app.world_mut().despawn(owner);
        app.update();
        let world = app.world();
        assert!(!world.get::<ParticleSpawnerState>(emitter).unwrap().active);
        assert!(world.get::<Follows>(emitter).is_none());
        assert!(matches!(
            world.get::<OneShot>(emitter),
            Some(OneShot::Despawn)
        ));
    }
}
