#[derive(Default, Resource, Debug)]
pub struct Lobby {
    pub clients: HashMap<u64, ClientData>,
}

#[derive(Parser, Resource)]
pub struct ClientCliArgs {
    #[arg(long)]
    pub nickname: String,
    #[arg(long, default_value_t = true)]
    pub autostart: bool,
}

#[derive(Parser, Resource)]
pub struct ServerCliArgs {
    #[arg(long, default_value_t = 1)]
    pub speed: u32,
    #[arg(long, default_value_t = ("").to_string())]
    pub system: String,
}

#[derive(Resource, Default)]
pub struct GameConfig {
    pub nickname: String,
    pub connected: bool,
    pub autostart: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy, Serialize, Deserialize)]
pub enum GameState {
    Running, // full networked game play
    Waiting, // waiting for clients
    Stopped, // initial state
}

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

// FIXME: maybe wrong place?
impl InitData {
    fn init<'a>(
        &self,
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