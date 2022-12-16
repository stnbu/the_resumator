use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_rapier3d::prelude::{Collider, NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::TAU;

pub mod ui;
pub use ui::*;
pub mod physics;
pub use physics::*;
pub mod inhabitant;
pub mod networking;
pub mod systems;

#[derive(Resource, Default)]
pub struct GameConfig {
    pub nickname: String,
    pub connected: bool,
    pub autostart: bool,
    pub standalone: bool,
}

pub struct Spacetime;

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
        transform: Transform::from_xyz(-500000.0, -500000.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            shadow_normal_bias: NORMAL_BIAS,
            shadow_depth_bias: SHADOW_BIAS,
            ..default()
        },
        // TODO: figure out what _translation_ means for directional
        transform: Transform::from_xyz(500000.0, 500000.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

impl Plugin for Spacetime {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .init_resource::<PhysicsConfig>()
            .add_event::<DeltaEvent>()
            .add_event::<MassCollisionEvent>()
            .add_event::<DespawnMassEvent>()
            .add_system_set(
                SystemSet::on_update(GameState::Running)
                    .with_system(handle_despawn_mass)
                    .with_system(signal_freefall_delta.before(handle_despawn_mass))
                    .with_system(handle_freefall.before(handle_despawn_mass))
                    .with_system(handle_mass_collisions.before(handle_despawn_mass))
                    .with_system(merge_masses.before(handle_despawn_mass)),
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
                filter: "info,wgpu_core=warn,wgpu_hal=off,mass_gathering=debug,mass_gathering::networking=debug".into(),
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
        app.insert_resource(MassIDToEntity::default());
        app.add_event::<inhabitant::ClientRotation>();
        app.init_resource::<GameConfig>();
        app.add_state(GameState::Stopped);
        app.add_system_set(
            SystemSet::on_update(GameState::Running)
                .with_system(inhabitant::control)
                .with_system(inhabitant::rotate_client_inhabited_mass),
        );
        app.add_plugin(EguiPlugin);
        app.add_startup_system(let_light);
        app.add_system(bevy::window::close_on_esc);
        app.add_startup_system(disable_rapier_gravity);
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    }
}

pub struct FullGameStandalone;

impl Plugin for FullGameStandalone {
    fn build(&self, app: &mut App) {
        app.add_plugin(Core);
        app.add_plugin(Spacetime);
        app.insert_resource(systems::testing_no_unhinhabited());
        app.add_startup_system(setup_standalone);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum GameState {
    Running, // full networked game play
    Waiting, // waiting for clients
    Stopped, // initial state
}

fn disable_rapier_gravity(mut rapier_config: ResMut<RapierConfiguration>) {
    rapier_config.gravity = Vec3::ZERO;
}

pub fn radius_to_mass(radius: f32) -> f32 {
    (2.0 / 3.0) * TAU * radius.powf(3.0)
}

pub fn mass_to_radius(mass: f32) -> f32 {
    ((mass * (3.0 / 2.0)) / TAU).powf(1.0 / 3.0)
}

fn setup_standalone(
    init_data: Res<InitData>,
    mut mass_to_entity_map: ResMut<MassIDToEntity>,
    mut game_state: ResMut<State<GameState>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // FIXME: some logic overlap with ClientJoined handler
    *mass_to_entity_map = init_data
        .clone()
        .init(&mut commands, &mut meshes, &mut materials);

    let mut mass_id_ = None;
    for (mass_id, mass_init_data) in init_data.masses.iter() {
        if mass_init_data.inhabitable {
            mass_id_ = Some(mass_id);
            break;
        }
    }
    let mass_id = mass_id_.unwrap();
    let inhabited_mass = mass_to_entity_map.0.get(mass_id).unwrap();
    let mut inhabited_mass_commands = commands.entity(*inhabited_mass);
    inhabited_mass_commands.insert(inhabitant::ClientInhabited);
    inhabited_mass_commands.despawn_descendants();
    debug!("Appending camera to inhabited mass {inhabited_mass:?}");
    inhabited_mass_commands.with_children(|child| {
        child.spawn(Camera3dBundle::default());
    });
    let _ = game_state.overwrite_set(GameState::Running);
}

pub fn set_window_title(
    game_state: Res<State<GameState>>,
    mut windows: ResMut<Windows>,
    game_config: Res<GameConfig>,
) {
    let title = if game_config.standalone {
        "Mass Gathering".to_string()
    } else {
        let nickname = if game_config.nickname.is_empty() {
            "<unset>"
        } else {
            &game_config.nickname
        };
        format!("Client[{:?}] : nick={nickname}", game_state.current())
    };
    windows.primary_mut().set_title(title);
}

#[derive(Component)]
pub struct MassID(pub u64);

#[derive(Resource, Default, Clone)]
pub struct MassIDToEntity(HashMap<u64, Entity>);

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct MassInitData {
    pub inhabitable: bool,
    pub position: Vec3,
    pub velocity: Vec3,
    pub color: Color,
    pub radius: f32,
}

#[derive(Default, Serialize, Deserialize, Resource, Debug)]
pub struct InitData {
    pub masses: HashMap<u64, MassInitData>,
}

impl Clone for InitData {
    fn clone(&self) -> Self {
        let mut masses = HashMap::new();
        masses.extend(&self.masses);
        Self { masses }
    }

    fn clone_from(&mut self, source: &Self) {
        let mut masses = HashMap::new();
        masses.extend(&source.masses);
        self.masses = masses;
    }
}

#[derive(Component)]
pub struct Garb;

impl InitData {
    fn init<'a>(
        &mut self,
        commands: &'a mut Commands,
        meshes: &'a mut ResMut<Assets<Mesh>>,
        materials: &'a mut ResMut<Assets<StandardMaterial>>,
    ) -> MassIDToEntity {
        let mut mass_to_entity_map = MassIDToEntity::default();
        for (
            &mass_id,
            &MassInitData {
                inhabitable,
                position,
                velocity,
                color,
                radius,
            },
        ) in self.masses.iter()
        {
            let mut mass_commands = commands.spawn(PointMassBundle {
                pbr: PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Icosphere {
                        radius,
                        ..Default::default()
                    })),
                    material: materials.add(color.into()),
                    transform: Transform::from_translation(position)
                        .looking_at(Vec3::ZERO, Vec3::Y),
                    ..Default::default()
                },
                momentum: Momentum {
                    velocity,
                    mass: radius_to_mass(radius),
                    ..Default::default()
                },
                collider: Collider::ball(radius),
                ..Default::default()
            });
            mass_commands.insert(MassID(mass_id));
            if inhabitable {
                mass_commands
                    .insert(inhabitant::Inhabitable)
                    .with_children(|child| {
                        // barrel
                        child
                            .spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Capsule {
                                    radius: 0.05,
                                    depth: 1.0,
                                    ..Default::default()
                                })),
                                material: materials.add(Color::WHITE.into()),
                                transform: Transform::from_rotation(Quat::from_rotation_x(
                                    TAU / 4.0,
                                ))
                                .with_translation(Vec3::Z * -1.5),
                                ..Default::default()
                            })
                            .insert(Garb);
                        // horizontal stabilizer
                        child
                            .spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                                material: materials.add(Color::WHITE.into()),
                                transform: Transform::from_translation(Vec3::Z * 0.5),
                                ..Default::default()
                            })
                            .insert(Garb);
                        // vertical stabilizer
                        child
                            .spawn(PbrBundle {
                                mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                                material: materials.add(Color::WHITE.into()),
                                transform: Transform::from_rotation(Quat::from_rotation_z(
                                    TAU / 4.0,
                                ))
                                .with_translation(Vec3::Z * 0.5),
                                ..Default::default()
                            })
                            .insert(Garb);
                    });
            }
            mass_to_entity_map.0.insert(mass_id, mass_commands.id());
        }
        mass_to_entity_map
    }
}
