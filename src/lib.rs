use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_rapier3d::prelude::{Collider, NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::TAU;

pub mod inhabitant;
pub mod networking;
pub mod physics;
pub mod systems;
pub mod ui;

#[derive(Resource, Default)]
pub struct GameConfig {
    pub nickname: String,
    pub connected: bool,
    pub autostart: bool,
    pub standalone: bool,
}

pub struct Spacetime;

pub fn let_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-0.5, -0.3, -1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(1.0, -2.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

impl Plugin for Spacetime {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .init_resource::<physics::PhysicsConfig>()
            .add_event::<physics::MassCollisionEvent>()
            .add_event::<physics::DespawnMassEvent>()
            .add_system_set(
                SystemSet::on_update(GameState::Running)
                    .with_system(physics::handle_despawn_mass)
                    .with_system(physics::freefall.before(physics::handle_despawn_mass))
                    .with_system(
                        physics::handle_mass_collisions.before(physics::handle_despawn_mass),
                    )
                    .with_system(physics::merge_masses.before(physics::handle_despawn_mass)),
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
        app.add_event::<networking::ClientMessages>();
        app.add_event::<networking::ServerMessage>();
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
        app.insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            ..Default::default()
        });
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    }
}

#[derive(Parser, Resource)]
pub struct StandaloneCliArgs {
    #[arg(long, default_value_t = 1)]
    pub speed: u32,
    #[arg(long, default_value_t = ("").to_string())]
    pub system: String,
}

pub struct FullGameStandalone;

impl Plugin for FullGameStandalone {
    fn build(&self, app: &mut App) {
        let StandaloneCliArgs { speed, system } = StandaloneCliArgs::parse();

        app.add_plugin(Core);
        app.insert_resource(physics::PhysicsConfig {
            sims_per_frame: speed,
        });
        app.add_plugin(Spacetime);
        app.insert_resource(systems::get_system(&system)());
        app.add_startup_system(setup_standalone);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum GameState {
    Running, // full networked game play
    Waiting, // waiting for clients
    Stopped, // initial state
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

#[derive(Component)]
pub struct MassID(pub u64);

#[derive(Resource, Default, Clone)]
pub struct MassIDToEntity(HashMap<u64, Entity>);

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct MassMotion {
    pub position: Vec3,
    pub velocity: Vec3,
}

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct MassInitData {
    pub inhabitable: bool,
    pub motion: MassMotion,
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
                motion: MassMotion { position, velocity },
                color,
                radius,
            },
        ) in self.masses.iter()
        {
            let mut mass_commands = commands.spawn(physics::PointMassBundle {
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
                momentum: physics::Momentum {
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
                        child.spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Capsule {
                                radius: 0.05,
                                depth: 1.0,
                                ..Default::default()
                            })),
                            material: materials.add(Color::WHITE.into()),
                            transform: Transform::from_rotation(Quat::from_rotation_x(TAU / 4.0))
                                .with_translation(Vec3::Z * -1.5),
                            ..Default::default()
                        });
                        // horizontal stabilizer
                        child.spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                            material: materials.add(Color::WHITE.into()),
                            transform: Transform::from_translation(Vec3::Z * 0.5),
                            ..Default::default()
                        });
                        // vertical stabilizer
                        child.spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                            material: materials.add(Color::WHITE.into()),
                            transform: Transform::from_rotation(Quat::from_rotation_z(TAU / 4.0))
                                .with_translation(Vec3::Z * 0.5),
                            ..Default::default()
                        });
                    });
            }
            mass_to_entity_map.0.insert(mass_id, mass_commands.id());
        }
        mass_to_entity_map
    }
}
