use bevy::app::PluginGroupBuilder;
use bevy::input::mouse::MouseButtonInput;
use bevy::log::LogSettings;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use rand::Rng;
use std::f32::consts::{PI, TAU};
mod physics;
use physics::*;

mod craft;
mod helpscreen;
mod mg_shapes;
pub mod prelude;

use prelude::*;

pub struct FullGame;

impl PluginGroup for FullGame {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(Core).add(SpacecraftPlugin).add(Spacetime);
    }
}

pub struct SpacecraftPlugin;

impl Plugin for SpacecraftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpacecraftConfig>()
            .init_resource::<VectorBallData>()
            .add_startup_system(create_vector_ball)
            .add_event::<ProjectileCollisionEvent>()
            .add_event::<HotPlanetEvent>()
            .add_event::<FireProjectileEvent>()
            .add_system_set(
                SystemSet::on_update(AppState::Playing)
                    .with_system(move_forward)
                    .with_system(control)
                    .with_system(stars)
                    .with_system(signal_hot_planet)
                    .with_system(relay_vector_ball_updates.after(signal_hot_planet))
                    .with_system(update_vector_ball.after(relay_vector_ball_updates))
                    .with_system(fire_on_hot_planet)
                    .with_system(animate_projectile_explosion)
                    .with_system(handle_hot_planet)
                    .with_system(set_ar_default_visibility.before(handle_hot_planet))
                    .with_system(move_projectiles.before(handle_despawn_planet))
                    .with_system(transfer_projectile_momentum)
                    // FIXME: even though `handle_despawn_planet` added by another plugin?
                    .with_system(spawn_projectile_explosion_animation.after(handle_despawn_planet))
                    .with_system(
                        handle_projectile_despawn.after(spawn_projectile_explosion_animation),
                    ),
            )
            .add_startup_system(spacecraft_setup)
            .add_system(set_camera_viewports)
            .add_system_set(SystemSet::on_update(AppState::Help).with_system(helpscreen));
    }
}

pub struct Spacetime;

impl Plugin for Spacetime {
    fn build(&self, app: &mut App) {
        app.init_resource::<PhysicsConfig>()
            .add_event::<DeltaEvent>()
            .add_event::<PlanetCollisionEvent>()
            .add_event::<DespawnPlanetEvent>()
            .add_event::<VectorBallUpdate>()
            .add_system_set(
                SystemSet::on_update(AppState::Playing)
                    .with_system(handle_despawn_planet)
                    .with_system(signal_freefall_delta.before(handle_despawn_planet))
                    .with_system(handle_freefall.before(handle_despawn_planet))
                    .with_system(handle_planet_collisions.before(handle_despawn_planet))
                    .with_system(transfer_planet_momentum.before(handle_despawn_planet)),
            );
    }
}

pub struct Core;

impl Plugin for Core {
    fn build(&self, app: &mut App) {
        app.add_plugins(MinimalPlugins);

        #[cfg(debug_assertions)]
        app.insert_resource(LogSettings {
            filter: "warn,mass_gathering=debug".into(),
            level: bevy::log::Level::DEBUG,
        })
        .add_plugin(bevy::log::LogPlugin);
        debug!("DEBUG LEVEL LOGGING ! !");

        // An attempt at minimizing DefaultPlugins for our purposes
        app.add_plugin(bevy::transform::TransformPlugin)
            .add_plugin(bevy::input::InputPlugin)
            .add_plugin(bevy::window::WindowPlugin)
            .add_plugin(bevy::asset::AssetPlugin)
            .add_plugin(bevy::scene::ScenePlugin)
            .add_plugin(bevy::winit::WinitPlugin)
            .add_plugin(bevy::render::RenderPlugin)
            .add_plugin(bevy::core_pipeline::CorePipelinePlugin)
            .add_plugin(bevy::pbr::PbrPlugin)
	    // ...
	    ;

        #[cfg(not(debug_assertions))]
        {
            error!("We have no logging, and yet you SEE this message...?");
            // FIXME: num-triangles on a mesh is a different thing
            app.insert_resource(Msaa { samples: 4 });
        }

        #[cfg(target_arch = "wasm32")]
        app.add_system(handle_browser_resize);

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system(bevy::window::close_on_esc);

        app.add_plugin(EguiPlugin)
            .add_state(AppState::Help)
            .add_startup_system(disable_rapier_gravity)
            .add_system(handle_game_state)
            .add_system(timer_despawn)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default());
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
enum AppState {
    Playing,
    Help,
}

fn disable_rapier_gravity(mut rapier_config: ResMut<RapierConfiguration>) {
    rapier_config.gravity = Vec3::ZERO;
}

fn handle_game_state(
    mut app_state: ResMut<State<AppState>>,
    keys: Res<Input<KeyCode>>,
    mouse_button_input_events: EventReader<MouseButtonInput>,
    mut windows: ResMut<Windows>,
) {
    use AppState::*;
    use KeyCode::*;
    let next_state = if *app_state.current() == Help && !mouse_button_input_events.is_empty() {
        let window = windows.get_primary_mut().unwrap();
        window.set_cursor_visibility(false);
        window.set_cursor_lock_mode(true);
        Some(Playing)
    } else {
        keys.get_just_pressed()
            .fold(None, |_state, key| match (*app_state.current(), *key) {
                (Playing, P | H | M) => {
                    let window = windows.get_primary_mut().unwrap();
                    window.set_cursor_visibility(true);
                    window.set_cursor_lock_mode(false);
                    Some(Help)
                }
                (_, _) => Some(Playing),
            })
    };
    if let Some(state) = next_state {
        let _ = app_state.overwrite_set(state);
    }
}

// Take the latitude (poles are [1,-1]) and the longitude (portion around, starting at (0,0,1))
// and return the x, y, z on the unit sphere.
fn latlon_to_cartesian(lat: f32, lon: f32) -> Vec3 {
    let theta = (lat * 2.0 - 1.0).acos(); // latitude. -1 & 1 are poles. 0 is equator.
    let phi = lon * TAU; // portion around the planet `[0,1)` (from Greenwich)
    let x = theta.sin() * phi.cos();
    let y = theta.sin() * phi.sin();
    let z = theta.cos();
    Vec3::new(x, y, z)
}

pub(crate) fn radius_to_mass(radius: f32) -> f32 {
    (4.0 / 3.0) * PI * radius.powf(3.0)
}

pub(crate) fn mass_to_radius(mass: f32) -> f32 {
    ((mass * (3.0 / 4.0)) / PI).powf(1.0 / 3.0)
}

#[derive(Component)]
pub struct Star;

#[derive(Component)]
pub struct DespawnTimer {
    pub ttl: Timer,
}

pub fn timer_despawn(
    mut commands: Commands,
    mut despawn_query: Query<(Entity, &mut DespawnTimer)>,
    time: Res<Time>,
) {
    for (entity, mut despawn_timer) in despawn_query.iter_mut() {
        despawn_timer.ttl.tick(time.delta());
        if despawn_timer.ttl.finished() {
            debug!("Despawning by timer: {entity:?}");
            commands.entity(entity).despawn();
        }
    }
}

pub fn my_planets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let mut rf = || rng.gen::<f32>();
    let pair_count = 18;
    for _ in 0..pair_count {
        let position = latlon_to_cartesian(rf(), rf()) * (rf() * 40.0 + 10.0);
        let velocity = latlon_to_cartesian(rf(), rf()) * Vec3::new(10.0, rf() * 0.1, 10.0) * 0.1;
        let radius = rf() + 2.0;
        for side in [-1.0, 1.0] {
            let color = Color::rgb(rf(), rf(), rf());
            spawn_planet(
                radius,
                position * side,
                velocity * side,
                color,
                &mut commands,
                &mut meshes,
                &mut materials,
            );
        }
    }

    // poorly implemented stars!!
    let star_count = 40;
    for _ in 0..star_count {
        let position = latlon_to_cartesian(rf(), rf()) * 400.0;
        let radius = rf() * 0.3 + 0.7;
        let (r, w, y) = (rf() * 40.0, rf() * 400.0, rf() * 20.0);
        let star_colored = (Color::RED * r + Color::WHITE * w + Color::YELLOW * y) * 1000.0;
        commands
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius,
                    ..default()
                })),
                material: materials.add(star_colored.into()),
                transform: Transform::from_translation(position),
                ..default()
            })
            .insert(Star);
    }
}

#[derive(Default)]
pub struct Prev(pub Vec3);

fn stars(
    mut stars_query: Query<&mut Transform, (With<Star>, Without<Spacecraft>)>,

    spacecraft_query: Query<&mut Transform, With<Spacecraft>>,
    mut previous: Local<Prev>,
) {
    let spacecraft = spacecraft_query.get_single().unwrap();
    for mut star in stars_query.iter_mut() {
        star.translation += spacecraft.translation - previous.0;
    }
    previous.0 = spacecraft.translation;
}

#[cfg(target_arch = "wasm32")]
fn handle_browser_resize(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    let wasm_window = web_sys::window().unwrap();
    let (target_width, target_height) = (
        wasm_window.inner_width().unwrap().as_f64().unwrap() as f32,
        wasm_window.inner_height().unwrap().as_f64().unwrap() as f32,
    );
    if window.width() != target_width || window.height() != target_height {
        window.set_resolution(target_width, target_height);
    }
}
