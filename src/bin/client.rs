use std::{net::UdpSocket, time::SystemTime};

use bevy::prelude::*;

use bevy_renet::{
    renet::{ClientAuthentication, RenetClient, RenetError},
    run_if_client_connected, RenetClientPlugin,
};

use mass_gathering::{
    client_connection_config, spawn_server_view_camera, systems::spawn_planet, ClientChannel,
    ClientMessages, FullGame, GameState, PhysicsConfig, ServerChannel, ServerMessages, PORT_NUMBER,
    PROTOCOL_ID, SERVER_ADDR,
};

fn new_renet_client() -> RenetClient {
    let client_id = 0;
    let server_addr = format!("{SERVER_ADDR}:{PORT_NUMBER}").parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let connection_config = client_connection_config();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };
    RenetClient::new(current_time, socket, connection_config, authentication).unwrap()
}

fn main() {
    App::new()
        .add_event::<ClientMessages>()
        .insert_resource(PhysicsConfig { sims_per_frame: 5 })
        .add_plugins(FullGame)
        .add_startup_system(spawn_server_view_camera)
        .add_plugin(RenetClientPlugin::default())
        //.insert_resource(new_renet_client())
        //.add_system(send_client_messages)
        .add_system(client_sync_players.with_run_criteria(run_if_client_connected))
        .add_system(send_client_messages.with_run_criteria(run_if_client_connected))
        .add_system(panic_on_error_system)
        .run();
}

fn panic_on_error_system(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}

fn client_sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut client: ResMut<RenetClient>,
    mut client_messages: EventWriter<ClientMessages>,
    mut app_state: ResMut<State<GameState>>,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::Init(init_data) => {
                info!(
                    "Server sent init data for {} planets to client {}",
                    init_data.planets.len(),
                    client.client_id()
                );
                info!("  spawning planets...");
                for (&planet_id, &planet_init_data) in init_data.planets.iter() {
                    spawn_planet(
                        planet_id,
                        planet_init_data,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                    );
                }
                let message = ClientMessages::Ready;
                info!("  sending message to server `{message:?}`");
                client_messages.send(message);
            }
            ServerMessages::SetGameState(game_state) => {
                info!("Server says set state to {game_state:?}");
                let _ = app_state.overwrite_set(game_state);
            }
        }
    }
}

fn send_client_messages(
    mut client_messages: EventReader<ClientMessages>,
    mut client: ResMut<RenetClient>,
) {
    for command in client_messages.iter() {
        let message = bincode::serialize(command).unwrap();
        client.send_message(ClientChannel::ClientMessages, message);
    }
}
