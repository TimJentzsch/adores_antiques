use bevy::prelude::*;
use bevy::ecs::system::EntityCommands;
use bevy_rapier3d::prelude::*;
use bevy::ecs::component::StorageType;
use crate::{
    assets,
    game_state,
    AppState,
    ingame,
};
use bevy::gltf::Gltf;
use bevy_scene_hook::{SceneHook, HookedSceneBundle};

const PROP_BREAK_THRESHOLD: f32 = 0.00001;
pub struct PropsPlugin;
impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(AppState::InGame)
                .with_system(handle_break_events)
                .with_system(handle_breakables)
           )
           .add_event::<BreakEvent>();
    }
}


//pub trait AddComponentFn = Fn(&mut EntityCommands);
pub trait ComponentAdder {
    fn add_components(entity_commands: &mut EntityCommands);
}

pub struct BreakEvent;

#[derive(Component)]
pub struct Breakable {
    breakable_type: BreakableType,
}
#[derive(Component)]
pub enum BreakableType {
    Plate,
    Mug,
}

#[derive(Component)]
pub struct Plate;
#[derive(Component)]
pub struct BrokenPlate;
#[derive(Component)]
pub struct Mug;
#[derive(Component)]
pub struct BrokenMug;

fn add_dynamic_rapier_components_for_props(entity_commands: &mut EntityCommands) {
    entity_commands
            .insert(Restitution::coefficient(0.9))
            .insert(ColliderMassProperties::Density(0.01))
            .insert(CollisionGroups::default())
            .insert(Velocity::default())
            .insert(Visibility {
                is_visible: true,
            })
            .insert(RigidBody::Dynamic);
}

pub fn restore_dynamic_rapier_components(entity_commands: &mut EntityCommands) {
    entity_commands
            .insert(CollisionGroups::default())
            .insert(Velocity::default())
            .insert(Visibility {
                is_visible: true,
            })
            .insert(RigidBody::Dynamic);
}

fn remove_dynamic_rapier_components_for_props(entity_commands: &mut EntityCommands) {
    entity_commands
            .insert(CollisionGroups {
                memberships: Group::NONE,
                ..default()
            })
            .insert(Velocity::default())
            .insert(Visibility {
                is_visible: false,
            })
            .insert(RigidBody::Fixed);
}

fn add_breakable_rapier_components(entity_commands: &mut EntityCommands) {
    entity_commands
            .insert(ActiveEvents::CONTACT_FORCE_EVENTS)
            .insert(ContactForceEventThreshold(PROP_BREAK_THRESHOLD));
}

impl ComponentAdder for Plate {
    fn add_components(entity_commands: &mut EntityCommands) {
        entity_commands
            .insert(Collider::cuboid(0.3, 0.05, 0.3))
            .insert(Breakable {
                breakable_type: BreakableType::Plate,
            })
            .insert(Plate)
            .insert(ingame::CleanupMarker);
        add_dynamic_rapier_components_for_props(entity_commands);
        add_breakable_rapier_components(entity_commands);
    }
}


impl ComponentAdder for BrokenPlate {
    fn add_components(entity_commands: &mut EntityCommands) {
        entity_commands
            .insert(Collider::cuboid(0.3, 0.05, 0.3))
            .insert(BrokenPlate)
            .insert(ingame::CleanupMarker);
        add_dynamic_rapier_components_for_props(entity_commands);
    }
}

impl ComponentAdder for Mug {
    fn add_components(entity_commands: &mut EntityCommands) {
        entity_commands
            .insert(Collider::cuboid(0.3, 0.05, 0.3))
            .insert(Breakable {
                breakable_type: BreakableType::Mug,
            })
            .insert(ingame::CleanupMarker)
            .insert(Mug);
        add_dynamic_rapier_components_for_props(entity_commands);
        add_breakable_rapier_components(entity_commands);
    }
}


impl ComponentAdder for BrokenMug {
    fn add_components(entity_commands: &mut EntityCommands) {
        entity_commands
            .insert(Collider::cuboid(0.3, 0.05, 0.3))
            .insert(ingame::CleanupMarker)
            .insert(BrokenMug);
        add_dynamic_rapier_components_for_props(entity_commands);
    }
}

fn handle_break_events(
    mut game_state: ResMut<game_state::GameState>,
    mut break_event_reader: EventReader<BreakEvent>,
) {
    for _ in break_event_reader.iter() {
        game_state.score += 1;
    }
}

fn handle_breakables(
    mut commands: Commands,
    breakables: Query<(&Breakable, &Transform, &Velocity)>,
    mut contact_force_events: EventReader<ContactForceEvent>,
    assets_gltf: Res<Assets<Gltf>>,
    game_assets: Res<assets::GameAssets>,
    mut break_event_writer: EventWriter<BreakEvent>,
) {
    for e in contact_force_events.iter() {
//        println!("contact force event {:?}", e.total_force_magnitude);
        [e.collider1, e.collider2].iter()
            .for_each(|entity| {
                if let Ok((breakable, transform, velocity)) = breakables.get(*entity) {
                    break_event_writer.send(BreakEvent);

                    let mut entity_commands = commands.entity(*entity);
                    remove_dynamic_rapier_components_for_props(&mut entity_commands);

                    let transform = transform.clone();
                    let velocity = velocity.clone();
                    let (asset, mesh_name, adder) = match breakable.breakable_type {
                        BreakableType::Plate => (&game_assets.broken_plate, "plate", 
                                           Box::new(BrokenPlate::add_components) as Box<dyn Fn(&mut EntityCommands) + Send + Sync>),
                        BreakableType::Mug => (&game_assets.broken_mug, "mug", 
                                           Box::new(BrokenMug::add_components) as Box<dyn Fn(&mut EntityCommands) + Send + Sync>),
                    };

                    if let Some(gltf) = assets_gltf.get(asset) {
                        commands.spawn(HookedSceneBundle {
                            scene: SceneBundle { scene: gltf.scenes[0].clone(), ..default() },
                            hook: SceneHook::new(move |entity, cmds, _| {
                                if let Some(name) = entity.get::<Name>().map(|t|t.as_str()) {
                                    if name.contains(mesh_name) {
                                        adder(cmds); 
                                        cmds.insert(velocity.clone());
                                        cmds.insert(transform.clone());
                                    }
                                }
                            })
                        });
                    }
                }
            });
    }
}
