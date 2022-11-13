use bevy::prelude::*;
use mass_gathering::prelude::*;

fn main() {
    App::new()
        .add_plugins(FullGame)
        .insert_resource(SpacecraftConfig {
            start_transform: Transform::from_xyz(0.0, 0.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .add_startup_system(setup)
        .run();
}

const L: f32 = 14.0;
const R: f32 = 1.0;
const B: f32 = 2.0;
const I: f32 = 3.5;
const C: f32 = 2.0;

use std::f32::consts::PI;
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(
            (shape::Icosphere {
                radius: 0.2 * R,
                ..Default::default()
            })
            .into(),
        ),
        material: materials.add(Color::WHITE.into()),
        ..Default::default()
    });

    let S = L - I - B - C;
    commands
        .spawn_bundle(SpatialBundle::default())
        .with_children(|child| {
            child.spawn_bundle(PbrBundle {
                mesh: meshes.add(
                    (Cone {
                        radius: 2.0 * R,
                        height: 2.0 * R,
                        ..Default::default()
                    })
                    .into(),
                ),
                transform: Transform::from_xyz(0.0, L - 2.0 * R, 0.0),
                material: materials.add(Color::GREEN.into()),
                ..Default::default()
            });
            child.spawn_bundle(PbrBundle {
                mesh: meshes.add(
                    (Cylinder {
                        height: S,
                        radius_bottom: R,
                        radius_top: R,
                        ..Default::default()
                    })
                    .into(),
                ),
                transform: Transform::from_xyz(0.0, S * 0.5 + I + B, 0.0),
                material: materials.add(Color::GREEN.into()),
                ..Default::default()
            });
        });
}