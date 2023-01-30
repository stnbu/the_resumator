use bevy_renet::{
    renet::{ClientAuthentication, DefaultChannel, RenetClient, RenetConnectionConfig},
    run_if_client_connected, RenetClientPlugin,
};
use clap::Parser;
use game::*;
use std::{net::UdpSocket, time::SystemTime};

pub mod plugins;

#[derive(Parser, Resource)]
pub struct ClientCliArgs {
    #[arg(long)]
    pub nickname: String,
    #[arg(long, default_value_t = format!("{SERVER_IP}:{SERVER_PORT}"))]
    pub address: String,
}

pub fn send_messages_to_server(
    mut to_server_events: EventReader<events::ToServer>,
    mut client: ResMut<RenetClient>,
) {
    for message in to_server_events.iter() {
        client.send_message(
            DefaultChannel::Reliable,
            bincode::serialize(message).unwrap(),
        );
    }
}

/// That is, "process 'to-client' events"
/// definitely NOT "process to 'client events'"
pub fn process_to_client_events(
    mut commands: Commands,
    mut game_state: ResMut<State<resources::GameState>>,
    mut to_client_events: EventReader<events::ToClient>,
    client: Res<RenetClient>,
) {
    let my_id = client.client_id();
    for message in to_client_events.iter() {
        match message {
            events::ToClient::SetGameState(new_game_state) => {
                let _ = game_state.overwrite_set(*new_game_state);
            }
            events::ToClient::InhabitantRotation { .. } => {
                // handled by separate system
            }
            events::ToClient::SetGameConfig(game_config) => {
                let inhabited_mass_id = *game_config.client_mass_map.get(&my_id).unwrap();
                game_config
                    .init_data
                    .spawn_masses(&mut commands, Some(inhabited_mass_id));
                commands.insert_resource(game_config.clone());
            }
            events::ToClient::ProjectileFired(_) => {
                // not handled here
            }
        }
    }
}

pub fn receive_messages_from_server(
    mut client: ResMut<RenetClient>,
    mut to_client_events: EventWriter<events::ToClient>,
) {
    while let Some(message) = client.receive_message(DefaultChannel::Reliable) {
        to_client_events.send(bincode::deserialize(&message).unwrap());
    }
}

pub fn new_renet_client(client_id: u64, address: String) -> RenetClient {
    let address = if let Ok(address) = format!("{address}").parse() {
        address
    } else {
        panic!("Cannot parse address `{address}`");
    };
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr: address,
        user_data: None,
    };
    RenetClient::new(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        socket,
        RenetConnectionConfig::default(),
        authentication,
    )
    .unwrap()
}

pub fn set_window_title(mut windows: ResMut<Windows>, client: Res<RenetClient>) {
    let title = "Mass Gathering";
    let id = client.client_id();
    let nickname = to_nick(id).trim_end().to_string();
    let title = format!("{title} | nick: \"{nickname}\"");
    windows.primary_mut().set_title(title);
}

pub fn set_resolution(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    if cfg!(debug_assertions) {
        window.set_resolution(1280.0 / 2.0, 720.0 / 2.0);
    } else {
        window.set_resolution(1280.0, 720.0);
    }
}

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

pub fn visualize_masses(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game_config: Res<resources::GameConfig>,
    masses_query: Query<(
        Entity,
        &components::MassID,
        &Transform,
        Option<&components::Inhabitable>,
        Option<&components::ClientInhabited>,
    )>,
    mut has_run: Local<bool>,
) {
    // FIXME: [HACK] Relying on `bool` having a default of `false`. The goal being "run once"
    if !*has_run && !masses_query.is_empty() {
        *has_run = true;
        info!("Running this system just this one time!");
        for (&mass_id, &resources::MassInitData { color, .. }) in
            game_config.init_data.masses.iter()
        {
            for (entity, &components::MassID(this_mass_id), &transform, inhabitable, inhabited) in
                masses_query.iter()
            {
                let inhabitable = inhabitable.is_some();
                let inhabited = inhabited.is_some();
                assert!(!(inhabitable && inhabited));
                let color: Color = color.into();
                if this_mass_id == mass_id {
                    warn!("Visualizing {mass_id}");
                    commands
                        .entity(entity)
                        .insert(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Icosphere {
                                radius: 1.0,
                                ..Default::default()
                            })),
                            material: materials.add(color.into()),
                            transform, // FIXME: is wierd?
                            ..Default::default()
                        })
                        .with_children(|children| {
                            // mass surface
                            if inhabited {
                                warn!("Mass {mass_id} is inhabted");
                                children.spawn(Camera3dBundle::default());
                                children
                                    .spawn(PbrBundle {
                                        mesh: meshes.add(Mesh::from(shape::Icosphere {
                                            radius: 0.0005,

                                            ..Default::default()
                                        })),
                                        material: materials.add(Color::WHITE.into()),
                                        transform: Transform::from_xyz(0.0, 0.0, -0.2),
                                        visibility: Visibility::INVISIBLE,
                                        ..Default::default()
                                    })
                                    .insert(components::Sights);
                                children
                                    .spawn(PointLightBundle {
                                        transform: Transform::from_xyz(0.0, 0.0, -0.15),
                                        visibility: Visibility::INVISIBLE,
                                        point_light: PointLight {
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    })
                                    .insert(components::Sights);
                            }
                            if inhabitable {
                                warn!("Mass {mass_id} is inhabtable");
                                // barrel
                                children.spawn(PbrBundle {
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
                                });
                                // horizontal stabilizer
                                children.spawn(PbrBundle {
                                    mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                                    material: materials.add(Color::WHITE.into()),
                                    transform: Transform::from_translation(Vec3::Z * 0.5),
                                    ..Default::default()
                                });
                                // vertical stabilizer
                                children.spawn(PbrBundle {
                                    mesh: meshes.add(Mesh::from(shape::Box::new(2.0, 0.075, 1.0))),
                                    material: materials.add(Color::WHITE.into()),
                                    transform: Transform::from_rotation(Quat::from_rotation_z(
                                        TAU / 4.0,
                                    ))
                                    .with_translation(Vec3::Z * 0.5),
                                    ..Default::default()
                                });
                            }
                        });
                    // We found/are done looking for the mass_id in question.
                    break;
                }
            }
        }
    }
}

pub fn control(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut to_server_events: EventWriter<events::ToServer>,
    mut inhabitant_query: Query<&mut Transform, With<components::ClientInhabited>>,
) {
    let nudge = TAU / 10000.0;
    let keys_scaling = 10.0;
    let mut rotation = Vec3::ZERO;
    for key in keys.get_pressed() {
        match key {
            // pitch
            KeyCode::W => {
                rotation.x += nudge;
            }
            KeyCode::S => {
                rotation.x -= nudge;
            }
            // yaw
            KeyCode::A => {
                rotation.y += nudge;
            }
            KeyCode::D => {
                rotation.y -= nudge;
            }
            // roll
            KeyCode::Z => {
                rotation.z -= nudge;
            }
            KeyCode::X => {
                rotation.z += nudge;
            }
            _ => (),
        }
    }
    if rotation.length() > 0.0000001 {
        if let Ok(mut transform) = inhabitant_query.get_single_mut() {
            let frame_time = time.delta_seconds() * 60.0;
            rotation *= keys_scaling * frame_time;
            let local_x = transform.local_x();
            let local_y = transform.local_y();
            let local_z = transform.local_z();
            transform.rotate(Quat::from_axis_angle(local_x, rotation.x));
            transform.rotate(Quat::from_axis_angle(local_z, rotation.z));
            transform.rotate(Quat::from_axis_angle(local_y, rotation.y));
            let message = events::ToServer::Rotation(transform.rotation);
            to_server_events.send(message);
        }
    }
}
