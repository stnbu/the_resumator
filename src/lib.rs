use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use std::f32::consts::PI;

pub mod physics;
pub use physics::*;

pub mod systems;

pub struct FullGame;

impl PluginGroup for FullGame {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(Core).add(Spacetime)
    }
}

pub fn let_light(mut commands: Commands) {
    // TODO: These are to be messed with.
    const NORMAL_BIAS: f32 = 0.61;
    const SHADOW_BIAS: f32 = 0.063;
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            shadow_normal_bias: NORMAL_BIAS,
            shadow_depth_bias: SHADOW_BIAS,
            ..default()
        },
        // TODO: figure out what _translation_ means for directional
        transform: Transform::from_xyz(-500000.0, -500000.0, 0.0),
        ..default()
    });
}

pub struct Spacetime;

impl Plugin for Spacetime {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .init_resource::<PhysicsConfig>()
            .add_event::<DeltaEvent>()
            .add_event::<PlanetCollisionEvent>()
            .add_event::<DespawnPlanetEvent>()
            .add_system_set(
                SystemSet::on_update(AppState::Playing)
                    .with_system(handle_despawn_planet)
                    .with_system(signal_freefall_delta.before(handle_despawn_planet))
                    .with_system(handle_freefall.before(handle_despawn_planet))
                    .with_system(handle_planet_collisions.before(handle_despawn_planet))
                    .with_system(merge_planets.before(handle_despawn_planet)),
            );
    }
}

pub struct Core;

impl Plugin for Core {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        {
            debug!("DEBUG LEVEL LOGGING ! !");
            app.add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
                filter: "info,wgpu_core=warn,wgpu_hal=off,mass_gathering=debug".into(),
                level: bevy::log::Level::DEBUG,
            }));
        }

        #[cfg(not(debug_assertions))]
        {
            error!("We have no logging, and yet you SEE this message...?");
            // FIXME: num-triangles on a mesh is a different thing
            app.insert_resource(Msaa { samples: 4 });
            app.add_plugins(DefaultPlugins);
        }

        app.add_startup_system(let_light);
        app.add_system(bevy::window::close_on_esc);

        app.add_state(AppState::Playing)
            .add_startup_system(disable_rapier_gravity)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
enum AppState {
    Playing,
}

fn disable_rapier_gravity(mut rapier_config: ResMut<RapierConfiguration>) {
    rapier_config.gravity = Vec3::ZERO;
}

pub fn radius_to_mass(radius: f32) -> f32 {
    (4.0 / 3.0) * PI * radius.powf(3.0)
}

pub fn mass_to_radius(mass: f32) -> f32 {
    ((mass * (3.0 / 4.0)) / PI).powf(1.0 / 3.0)
}
