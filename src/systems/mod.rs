use crate::{radius_to_mass, InitData, MassID, Momentum, PlanetInitData, PointMassBundle};
use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;

/// Make some interesting planets
pub fn cubic(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut planet_data: ResMut<InitData>,
) {
    let mut planet_id = 2000;
    let radius = 0.5;
    let from_origin = 6.0;
    for n in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
        for side in [1.0, -1.0] {
            let (a, b, c) = n;
            let speed = 0.1;
            let position = Vec3::new(
                a as f32 * side * from_origin,
                b as f32 * side * from_origin,
                c as f32 * side * from_origin,
            );
            let velocity = match (a, b, c) {
                (1, 0, 0) => Vec3::Y * side,
                (0, 1, 0) => Vec3::Z * side,
                (0, 0, 1) => Vec3::X * side,
                _ => panic!(),
            } * speed;
            let (r, g, b) = (a as f32, b as f32, c as f32);
            let (color, tweak) = if side > 0.0 {
                (Color::rgba(r, g, b, 0.8), 1.3)
            } else {
                (
                    Color::rgba((1.0 - r) / 2.0, (1.0 - g) / 2.0, (1.0 - b) / 2.0, 0.8),
                    0.92,
                )
            };
            let velocity = velocity * tweak;
            let radius = radius / tweak * (1.0 + ((planet_id as f32 - 2000.0) / 20.0));
            let position = position * tweak;
            let planet_init_data = PlanetInitData {
                position,
                velocity,
                color,
                radius,
            };
            planet_data.planets.insert(planet_id, planet_init_data);
            planet_id += 1;
            spawn_planet(
                planet_id,
                planet_init_data,
                &mut commands,
                &mut meshes,
                &mut materials,
            );
        }
    }
}

pub fn spawn_planet<'a>(
    planet_id: u64,
    planet_init_data: PlanetInitData,
    commands: &'a mut Commands,
    meshes: &'a mut ResMut<Assets<Mesh>>,
    materials: &'a mut ResMut<Assets<StandardMaterial>>,
) {
    let PlanetInitData {
        position,
        velocity,
        color,
        radius,
    } = planet_init_data;
    commands
        .spawn(PointMassBundle {
            pbr: PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Icosphere {
                    radius,
                    ..Default::default()
                })),
                material: materials.add(color.into()),
                transform: Transform::from_translation(position),
                ..Default::default()
            },
            momentum: Momentum {
                velocity,
                mass: radius_to_mass(radius),
                ..Default::default()
            },
            collider: Collider::ball(radius),
            ..Default::default()
        })
        .insert(MassID(planet_id));
}