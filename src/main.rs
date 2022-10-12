use bevy::prelude::*;
use particular::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

mod bodies;
mod space_camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ParticleSet::<bodies::Body>::new())
        .add_state(AppState::Startup)
        .add_system_set(
            SystemSet::on_update(AppState::Playing)
                .with_system(space_camera::move_forward)
                .with_system(space_camera::steer)
                .with_system(bodies::update_particles),
        )
        .insert_resource(space_camera::CameraConfig {
            transform: Transform::from_translation(Vec3::new(20.0, 30.0, -12.0))
                .looking_at(Vec3::new(1.0, 1.0, 1.0), Vec3::Y),
        })
        .add_plugin(space_camera::SpaceCamera)
        .add_startup_system(setup)
        .add_system(bevy::window::close_on_esc) // "or prototyping" -- unclean shutdown
        .add_system(handle_game_state)
        .run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    Startup,
    Playing,
    Paused,
}

fn toggle_pause(current: &AppState) -> Option<AppState> {
    match current {
        AppState::Paused => Some(AppState::Playing),
        AppState::Playing => Some(AppState::Paused),
        _ => None,
    }
}

fn handle_game_state(
    mut focus_events: EventReader<bevy::window::WindowFocused>,
    mut app_state: ResMut<State<AppState>>,
    keys: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
) {
    let mut poked = false; // space bar hit or window left-clicked
    for key in keys.get_just_pressed() {
        if *key == KeyCode::Space {
            poked = !poked;
        }
    }
    if mouse_buttons.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
        poked = !poked;
    }

    if !poked {
        for ev in focus_events.iter() {
            if ev.focused {
                app_state.overwrite_set(AppState::Playing).unwrap();
            } else {
                app_state.overwrite_set(AppState::Paused).unwrap();
            }
        }
    } else {
        if *(app_state.current()) == AppState::Startup {
            app_state.overwrite_set(AppState::Playing).unwrap();
        } else {
            if let Some(new_state) = toggle_pause(app_state.current()) {
                app_state.overwrite_set(new_state).unwrap();
            }
        }
    }
}

#[derive(Bundle)]
struct Planet {
    #[bundle]
    pbr: PbrBundle,
    point_mass: bodies::PointMass,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particle_set: ResMut<ParticleSet<bodies::Body>>,
) {
    let mut rng = rand::thread_rng();
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let x = (x * 4) as f32 - 2.0 + rng.gen::<f32>();
                let y = (y * 4) as f32 - 2.0 + rng.gen::<f32>();
                let z = (z * 4) as f32 - 2.0 + rng.gen::<f32>();
                let position = Vec3::new(x, y, z);
                let r = rng.gen::<f32>();
                let g = rng.gen::<f32>();
                let b = rng.gen::<f32>();
                let radius = rng.gen::<f32>() + 0.2;
                let pbr = PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Icosphere {
                        radius,
                        ..Default::default()
                    })),
                    material: materials.add(Color::rgb(r, g, b).into()),
                    transform: Transform::from_translation(position),
                    ..Default::default()
                };
                let entity = commands
                    .spawn_bundle(Planet {
                        pbr,
                        point_mass: bodies::PointMass {},
                    })
                    .id();
                let mass = 0.75 * PI * radius.powf(3.0);
                let velocity = Vec3::new(rng.gen::<f32>(), rng.gen::<f32>(), rng.gen::<f32>());
                particle_set.add_massive(bodies::Body::new(position, mass, velocity, entity));
            }
        }
    }
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 800.0 * 1000.0,
            range: 1000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(220.0, 200.0, 45.0),
        ..Default::default()
    });
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 800.0 * 1000.0 * 0.8,
            range: 1000.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(-120.0, -300.0, -89.0),
        ..Default::default()
    });
}
