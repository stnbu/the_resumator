use bevy::prelude::*;
use bevy_egui::{
    egui::{style::Margin, Color32, FontId, Frame, RichText, TopBottomPanel},
    EguiContext, EguiPlugin,
};
use bevy_rapier3d::prelude::{
    Collider, NoUserData, QueryFilter, RapierConfiguration, RapierContext, RapierPhysicsPlugin,
};
use rand::Rng;
use std::f32::consts::TAU;

mod physics;
use physics::*;

mod craft;
use craft::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE * 0.1))
        .add_plugins(DefaultPlugins)
        .add_plugin(EguiPlugin)
        .add_state(AppState::Startup)
        .add_system_set(
            SystemSet::on_update(AppState::Playing)
                .with_system(move_forward)
                .with_system(steer)
                .with_system(freefall)
                .with_system(collision_events)
                .with_system(handle_projectile_engagement)
                .with_system(handle_projectile_flight),
        )
        .add_startup_system(setup)
        .add_system(bevy::window::close_on_esc)
        .add_system(handle_game_state)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_system(hud)
        .run();
}

#[derive(Component)]
struct BallisticProjectileTarget {
    planet: Entity,
    local_impact_site: Vec3,
}

fn handle_projectile_flight(
    mut commands: Commands,
    mut projectile_query: Query<(Entity, &mut Transform, &BallisticProjectileTarget)>,
    planet_query: Query<&Transform, (With<Collider>, Without<BallisticProjectileTarget>)>,
    time: Res<Time>,
) {
    for (projectile, mut projectile_transform, target) in projectile_query.iter_mut() {
        if let Ok(planet_transform) = planet_query.get(target.planet) {
            let gloal_impact_site = planet_transform.translation + target.local_impact_site;
            let distance = (projectile_transform.translation - gloal_impact_site).length();
            if distance > 5.0 {
                // spot-the-bug
                let direction = projectile_transform.translation - gloal_impact_site;
                projectile_transform.translation -= direction * time.delta_seconds() * 0.8;
            } else {
                commands.entity(projectile).despawn();
            }
        }
    }
}

fn handle_projectile_engagement(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    optional_keys: Option<Res<Input<KeyCode>>>,
    mut crosshairs_query: Query<&mut Visibility, With<Crosshairs>>,
    planet_query: Query<&Transform, With<Collider>>,
    rapier_context: Res<RapierContext>,
    craft: Query<&Transform, With<Spacecraft>>,
) {
    for pov in craft.iter() {
        let ray_origin = pov.translation;
        let ray_direction = -1.0 * pov.local_z();
        let intersection = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            150.0, // what's reasonable here...?
            true,
            QueryFilter::only_dynamic(),
        );

        if let Some((planet, distance)) = intersection {
            if let Some(ref keys) = optional_keys {
                if keys.just_pressed(KeyCode::F) {
                    let global_impact_site = ray_origin + (ray_direction * distance);
                    let transform = planet_query.get(planet).unwrap();
                    let local_impact_site = transform.translation - global_impact_site;
                    commands
                        .spawn_bundle(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Icosphere {
                                radius: 0.3,
                                ..Default::default()
                            })),
                            material: materials.add(Color::PINK.into()),
                            transform: Transform::from_translation(ray_origin),
                            ..Default::default()
                        })
                        .insert(BallisticProjectileTarget {
                            planet,
                            local_impact_site,
                        });
                }
            }
            for mut crosshairs in crosshairs_query.iter_mut() {
                crosshairs.is_visible = true;
            }
        } else {
            // ////
            for mut crosshairs in crosshairs_query.iter_mut() {
                crosshairs.is_visible = false;
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
enum AppState {
    Startup,
    Playing,
    Paused,
    Menu,
}

fn handle_game_state(mut app_state: ResMut<State<AppState>>, keys: Res<Input<KeyCode>>) {
    use AppState::*;
    use KeyCode::*;
    let next_state =
        keys.get_just_pressed()
            .fold(None, |_state, key| match (*app_state.current(), *key) {
                (Playing, Space) => Some(Paused),
                (Paused, Space) => Some(Playing),
                (Menu, M) => Some(Playing),
                (_, M) => Some(Menu),
                (Startup, _) => Some(Playing),
                _ => None,
            });
    if let Some(state) = next_state {
        let _ = app_state.overwrite_set(state);
    }
}

#[derive(Component)]
struct Crosshairs;

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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    rapier_config.gravity = Vec3::ZERO;

    let mut rng = rand::thread_rng();
    let mut rf = || rng.gen::<f32>();
    let pair_count = 40;
    for _ in 0..pair_count {
        let position = latlon_to_cartesian(rf(), rf()) * (rf() * 40.0 + 10.0);
        let velocity = latlon_to_cartesian(rf(), rf()) * Vec3::new(10.0, rf() * 0.1, 10.0);
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

    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 200.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
            ..Default::default()
        })
        .insert_bundle(VisibilityBundle::default())
        .insert(Spacecraft::default())
        .with_children(|parent| {
            // Possibly the worst way to implement "crosshairs" evar.
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Icosphere {
                        radius: 0.03,
                        ..Default::default()
                    })),
                    material: materials.add(Color::GREEN.into()),
                    transform: Transform::from_xyz(0.0, 0.0, -8.0),
                    visibility: Visibility { is_visible: false },
                    ..Default::default()
                })
                .insert(Crosshairs);
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(0.005, 5.0, 0.1))),
                    material: materials.add(Color::GREEN.into()),
                    transform: Transform::from_xyz(0.0, 0.0, -7.0),
                    visibility: Visibility { is_visible: false },
                    ..Default::default()
                })
                .insert(Crosshairs);
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(5.0, 0.005, 0.1))),
                    material: materials.add(Color::GREEN.into()),
                    transform: Transform::from_xyz(0.0, 0.0, -6.0),
                    visibility: Visibility { is_visible: false },
                    ..Default::default()
                })
                .insert(Crosshairs);

            // Various lights for seeing
            parent.spawn_bundle(PointLightBundle {
                transform: Transform::from_xyz(10.0, -10.0, -25.0),
                point_light: PointLight {
                    intensity: 5000.0 * 1.7,
                    range: 1000.0,
                    ..Default::default()
                },
                ..Default::default()
            });
            parent.spawn_bundle(PointLightBundle {
                transform: Transform::from_xyz(-10.0, 5.0, -35.0),
                point_light: PointLight {
                    intensity: 5000.0 * 1.5,
                    range: 1000.0,
                    ..Default::default()
                },
                ..Default::default()
            });
            parent.spawn_bundle(PointLightBundle {
                transform: Transform::from_xyz(30.0, -20.0, 80.0),
                point_light: PointLight {
                    intensity: 1000000.0 * 0.7,
                    range: 1000.0,
                    ..Default::default()
                },
                ..Default::default()
            });
            parent.spawn_bundle(PointLightBundle {
                transform: Transform::from_xyz(-30.0, 10.0, 100.0),
                point_light: PointLight {
                    intensity: 1000000.0 * 0.8,
                    range: 1000.0,
                    ..Default::default()
                },
                ..Default::default()
            });
        });
}

use bevy_egui::egui::FontFamily::Monospace;
fn hud(mut ctx: ResMut<EguiContext>, query: Query<(&Spacecraft, &Transform)>) {
    let (spacecraft, transform) = query.get_single().unwrap();
    TopBottomPanel::top("hud")
        .frame(Frame {
            outer_margin: Margin::symmetric(10.0, 20.0),
            fill: Color32::TRANSPARENT,
            ..Default::default()
        })
        .show(ctx.ctx_mut(), |ui| {
            ui.label(RichText::new("Keys:").color(Color32::GREEN).font(FontId {
                size: 18.0,
                family: Monospace,
            }));
            ui.label(
                RichText::new("Arrow Keys -  Pitch & Roll")
                    .color(Color32::GREEN)
                    .font(FontId {
                        size: 18.0,
                        family: Monospace,
                    }),
            );
            ui.label(
                RichText::new("Z & X      -  Yaw")
                    .color(Color32::GREEN)
                    .font(FontId {
                        size: 18.0,
                        family: Monospace,
                    }),
            );
            ui.label(
                RichText::new("PgUp/PgDn  -  Speed")
                    .color(Color32::GREEN)
                    .font(FontId {
                        size: 18.0,
                        family: Monospace,
                    }),
            );
            ui.label(
                RichText::new("F          -  Fire")
                    .color(Color32::GREEN)
                    .font(FontId {
                        size: 18.0,
                        family: Monospace,
                    }),
            );
            ui.label(RichText::new("\n"));
            ui.label(
                RichText::new(format!("Your Speed - {}", spacecraft.speed))
                    .color(Color32::GREEN)
                    .font(FontId {
                        size: 18.0,
                        family: Monospace,
                    }),
            );
            ui.label(
                RichText::new(format!(
                    "Your Location:\n     x - {}\n     y - {}\n     z - {}",
                    transform.translation.x, transform.translation.y, transform.translation.z
                ))
                .color(Color32::GREEN)
                .font(FontId {
                    size: 18.0,
                    family: Monospace,
                }),
            );
        });
}
