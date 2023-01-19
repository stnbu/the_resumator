use crate::*;
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Component, Debug)]
pub enum ToClient {
    Init(resources::InitData),
    SetGameState(resources::GameState),
    ClientJoined {
        id: u64,
        client_data: resources::ClientData,
    },
    SetPhysicsConfig(physics::PhysicsConfig),
    InhabitantRotation {
        client_id: u64,
        rotation: Quat,
    },
    ProjectileFired(ProjectileFlight),
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ToServer {
    Ready,
    Rotation(Quat),
    ProjectileFired(ProjectileFlight),
}

/// `launch_time` is: unix epoch time in milliseconds (u128) according
/// to the client.
#[derive(Debug, Serialize, Deserialize, Resource, Clone, Copy, Component)]
pub struct ProjectileFlight {
    pub launch_time: u128,
    pub from_mass_id: u64,
    pub to_mass_id: u64,
    pub local_impact_direction: Vec3,
}
