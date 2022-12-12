use crate::{
    networking::{client::Inhabitable, *},
    physics::Momentum,
    radius_to_mass, PointMassBundle,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;
use rand::Rng;
use std::f32::consts::TAU;

/// Old rando from way back
pub fn old_rando() -> InitData {
    let mut init_data = InitData::default();

    let mut rng = rand::thread_rng();
    let mut rf = || rng.gen::<f32>();
    let pair_count = 18;
    let mut mass_id = 2000;
    for _ in 0..pair_count {
        let position = latlon_to_cartesian(rf(), rf()) * (rf() * 40.0 + 10.0);
        let velocity = latlon_to_cartesian(rf(), rf()) * Vec3::new(10.0, rf() * 0.1, 10.0) * 0.1;
        let radius = rf() + 2.0;
        for side in [-1.0, 1.0] {
            let color = Color::rgb(rf(), rf(), rf());
            let position = position * side;
            let velocity = velocity * side;
            let mass_init_data = MassInitData {
                position,
                velocity,
                color,
                radius,
            };
            init_data
                .uninhabitable_masses
                .insert(mass_id, mass_init_data);
            mass_id += 1;
        }
    }
    let inhabitable_distance = 70.0;
    for (x, y, z) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
        let velocity = Vec3::ZERO;
        let color_tweak = match (x, y, z) {
            (1, 0, 0) => 1.0,
            (0, 1, 0) => 2.0,
            (0, 0, 1) => 3.0,
            _ => panic!("no!"),
        };
        let position = Vec3::new(x as f32, y as f32, z as f32) * inhabitable_distance;
        let color = Color::rgb(17.0, 19.0 / color_tweak, 23.0 * color_tweak);
        let radius = 1.0;
        let mass_init_data = MassInitData {
            position,
            velocity,
            color,
            radius,
        };
        init_data.inhabitable_masses.insert(mass_id, mass_init_data);
        mass_id += 1;
    }
    init_data
}

/// Make some interesting masses
pub fn cubic() -> InitData {
    let mut init_data = InitData::default();

    let mut mass_id = 2000;
    let radius = 0.5;
    let from_origin = 9.0;
    for n in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
        for side in [1.0, -1.0] {
            let fun_factor = 1.0 + (mass_id as f32 - 2000.0) / 20.0;

            let (a, b, c) = n;
            let speed = 0.15;
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
            let plus_side = side > 0.0;
            let color = if plus_side {
                Color::rgba(r, g, b, 0.8)
            } else {
                Color::rgba((1.0 - r) / 2.0, (1.0 - g) / 2.0, (1.0 - b) / 2.0, 0.8)
            };
            let velocity = if c == 1 {
                velocity
            } else {
                velocity * fun_factor
            };
            let radius = if a == 1 { radius } else { radius * fun_factor };

            let position = if c == 1 {
                position
            } else {
                position * fun_factor
            };

            let mass_init_data = MassInitData {
                position,
                velocity,
                color,
                radius,
            };
            init_data
                .uninhabitable_masses
                .insert(mass_id, mass_init_data);
            mass_id += 1;
        }
    }

    //
    let inhabitable_distance = 20.0;
    for (x, y, z) in [(1, 0, 0), (0, 1, 0), (0, 0, 1)] {
        let velocity = Vec3::ZERO;
        let color_tweak = match (x, y, z) {
            (1, 0, 0) => 1.0,
            (0, 1, 0) => 2.0,
            (0, 0, 1) => 3.0,
            _ => panic!("no!"),
        };
        let position = Vec3::new(x as f32, y as f32, z as f32) * inhabitable_distance;
        let color = Color::rgb(17.0, 19.0 / color_tweak, 23.0 * color_tweak);
        let radius = 1.0;
        let mass_init_data = MassInitData {
            position,
            velocity,
            color,
            radius,
        };
        init_data.inhabitable_masses.insert(mass_id, mass_init_data);
        mass_id += 1;
    }
    //
    init_data
}

pub fn testing_no_unhinhabited() -> InitData {
    let mut init_data = InitData::default();
    let position = Vec3::X * 6.0;
    let velocity = Vec3::Y * 0.5;
    let radius = 1.0;
    init_data.inhabitable_masses.insert(
        0,
        MassInitData {
            position,
            velocity,
            color: Color::RED,
            radius,
        },
    );
    init_data.inhabitable_masses.insert(
        1,
        MassInitData {
            position: position * -1.0,
            velocity: velocity * -1.0,
            color: Color::BLUE,
            radius,
        },
    );
    init_data
}

pub fn spawn_mass<'a>(
    inhabitable: bool,
    mass_id: u64,
    mass_init_data: MassInitData,
    commands: &'a mut Commands,
    meshes: &'a mut ResMut<Assets<Mesh>>,
    materials: &'a mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let MassInitData {
        position,
        velocity,
        color,
        radius,
    } = mass_init_data;
    let mut mass_commands = commands.spawn(PointMassBundle {
        pbr: PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius,
                ..Default::default()
            })),
            material: materials.add(color.into()),
            transform: Transform::from_translation(position).looking_at(Vec3::ZERO, Vec3::Y),
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
        mass_commands.insert(Inhabitable);
    }
    mass_commands.id()
}

/// Given a "latitude" and "longitude" on a unit sphere, return x,y,z
fn latlon_to_cartesian(lat: f32, lon: f32) -> Vec3 {
    let theta = (lat * 2.0 - 1.0).acos(); // latitude. -1 & 1 are poles. 0 is equator.
    let phi = lon * TAU; // portion around the sphere `[0,1)` (from Greenwich)
    let x = theta.sin() * phi.cos();
    let y = theta.sin() * phi.sin();
    let z = theta.cos();
    Vec3::new(x, y, z)
}
