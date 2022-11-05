use crate::{
    assets::GameAssets,
    direction,
    game_controller,
    game_state,
    ingame,
    AppState,
    ZeroSignum,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;
use std::collections::HashMap;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InputManagerPlugin::<PlayerAction>::default())
            .add_event::<PlayerMoveEvent>()
            .add_system_set(
                SystemSet::on_update(AppState::InGame)
                    .with_system(handle_controllers.before(handle_input))
                    .with_system(handle_input)
                    .with_system(move_player.after(handle_input)),
            );
    }
}

pub fn move_player(
    time: Res<Time>,
    mut players: Query<(Entity, &mut Transform, &mut Player)>,
    mut player_move_event_reader: EventReader<PlayerMoveEvent>,
    mut game_state: ResMut<game_state::GameState>,
    game_assets: ResMut<GameAssets>,
) {
    let mut move_events = HashMap::new();
    for move_event in player_move_event_reader.iter() {
        move_events.entry(move_event.entity).or_insert(move_event);
    }

    for (entity, mut transform, mut player) in players.iter_mut() {
        let speed: f32 = player.speed;
        let rotation_speed: f32 = player.rotation_speed;
        let friction: f32 = player.friction;

        player.velocity *= friction.powf(time.delta_seconds());
        if let Some(move_event) = move_events.get(&entity) {
            match move_event.movement {
                Movement::Normal(direction) => {
                    let acceleration = Vec3::from(direction);
                    player.velocity += (acceleration.zero_signum() * speed) * time.delta_seconds();
                }
            }
        }

        player.velocity = player.velocity.clamp_length_max(speed);
//      player.velocity.z *= if player.velocity.x > 0.0 { 1.0 } else { 0.0 };
//      player.velocity.y *= if player.velocity.x > 0.0 { 1.0 } else { 0.0 };
//      game_state.driving_speed = player.velocity.x * 0.1;

        let mut new_translation = transform.translation + (player.velocity * time.delta_seconds());

        let angle = (-(new_translation.z - transform.translation.z))
            .atan2(new_translation.x - transform.translation.x);
        let rotation = Quat::from_axis_angle(Vec3::Y, angle);
        transform.translation = new_translation;

//        transform.translation.x = 0.0; // hardcoding for now

        let new_rotation = transform
            .rotation
            .lerp(Quat::from_axis_angle(Vec3::Y, TAU * 0.75), time.delta_seconds() * rotation_speed);

        // don't rotate if we're not moving or if uhh rotation isnt a number?? why isn't it a number? who did this
        if !new_rotation.is_nan() && player.velocity.length() > 0.5 {
            transform.rotation = rotation;
        }
    }
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum PlayerAction {
    Up,
    Down,
    Left,
    Right,

    ActionUp,
    ActionDown,
    ActionLeft,
    ActionRight,
}

impl PlayerAction {
    const DIRECTIONS: [Self; 4] = [
        PlayerAction::Up,
        PlayerAction::Down,
        PlayerAction::Left,
        PlayerAction::Right,
    ];

    fn direction(self) -> direction::Direction {
        match self {
            PlayerAction::Up => direction::Direction::UP,
            PlayerAction::Down => direction::Direction::DOWN,
            PlayerAction::Left => direction::Direction::LEFT,
            PlayerAction::Right => direction::Direction::RIGHT,
            _ => direction::Direction::NEUTRAL,
        }
    }
}

pub struct PlayerMoveEvent {
    pub entity: Entity,
    pub movement: Movement,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub speed: f32,
    pub rotation_speed: f32,
    pub friction: f32,
    pub random: f32,
}

impl Player {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        Player {
            velocity: Vec3::default(),
            speed: 40.0,
            rotation_speed: 1.0,
            friction: 0.10,
            random: rng.gen_range(0.5..1.0),
        }
    }
}

#[derive(Bundle)]
pub struct PlayerBundle {
    player: Player,
    #[bundle]
    input_manager: InputManagerBundle<PlayerAction>,
}

impl PlayerBundle {
    pub fn new() -> Self {
        PlayerBundle {
            player: Player::new(),
            input_manager: InputManagerBundle {
                input_map: PlayerBundle::default_input_map(),
                action_state: ActionState::default(),
            },
        }
    }

    fn default_input_map() -> InputMap<PlayerAction> {
        use PlayerAction::*;
        let mut input_map = InputMap::default();

        input_map.set_gamepad(Gamepad { id: 0 });

        // Movement
//        input_map.insert(KeyCode::Up, Up);
        input_map.insert(KeyCode::W, Up);
        input_map.insert(KeyCode::Z, Up);
        input_map.insert(GamepadButtonType::DPadUp, Up);

//        input_map.insert(KeyCode::Down, Down);
        input_map.insert(KeyCode::S, Down);
        input_map.insert(GamepadButtonType::DPadDown, Down);

//        input_map.insert(KeyCode::Left, Left);
        input_map.insert(KeyCode::A, Left);
        input_map.insert(KeyCode::Q, Left);
        input_map.insert(GamepadButtonType::DPadLeft, Left);

//        input_map.insert(KeyCode::Right, Right);
        input_map.insert(KeyCode::D, Right);
        input_map.insert(GamepadButtonType::DPadRight, Right);

        // Actions
        input_map.insert(KeyCode::I, ActionUp);
        input_map.insert(GamepadButtonType::North, ActionUp);

        input_map.insert(KeyCode::K, ActionDown);
        input_map.insert(GamepadButtonType::South, ActionDown);

        input_map.insert(KeyCode::J, ActionLeft);
        input_map.insert(GamepadButtonType::West, ActionLeft);

        input_map.insert(KeyCode::L, ActionRight);
        input_map.insert(GamepadButtonType::East, ActionRight);

        input_map
    }
}

fn handle_controllers(
    controllers: Res<game_controller::GameController>,
    game_state: Res<game_state::GameState>,
    mut players: Query<(Entity, &mut ActionState<PlayerAction>), With<Player>>,
) {
    for (_, mut action_state) in players.iter_mut() {
        for (_, pressed) in controllers.pressed.iter() {
            // release all buttons
            // this probably affects durations but for
            // this game it might not be a big deal
            action_state.release(PlayerAction::Left);
            action_state.release(PlayerAction::Right);
            action_state.release(PlayerAction::Up);
            action_state.release(PlayerAction::Down);

            if pressed.contains(&game_controller::GameButton::Left) {
                action_state.press(PlayerAction::Left);
            }
            if pressed.contains(&game_controller::GameButton::Right) {
                action_state.press(PlayerAction::Right);
            }
            if pressed.contains(&game_controller::GameButton::Up) {
                action_state.press(PlayerAction::Up);
            }
            if pressed.contains(&game_controller::GameButton::Down) {
                action_state.press(PlayerAction::Down);
            }
            if pressed.contains(&game_controller::GameButton::ActionDown) {
                action_state.press(PlayerAction::ActionDown);
            } else {
                action_state.release(PlayerAction::ActionDown);
            }
            if pressed.contains(&game_controller::GameButton::ActionUp) {
                action_state.press(PlayerAction::ActionUp);
            } else {
                action_state.release(PlayerAction::ActionUp);
            }
            if pressed.contains(&game_controller::GameButton::ActionLeft) {
                action_state.press(PlayerAction::ActionLeft);
            } else {
                action_state.release(PlayerAction::ActionLeft);
            }
            if pressed.contains(&game_controller::GameButton::ActionRight) {
                action_state.press(PlayerAction::ActionRight);
            } else {
                action_state.release(PlayerAction::ActionRight);
            }
        }

        for (_, just_pressed) in controllers.just_pressed.iter() {
            if just_pressed.contains(&game_controller::GameButton::ActionUp) {
                action_state.release(PlayerAction::ActionUp);
                action_state.press(PlayerAction::ActionUp);
            }
            if just_pressed.contains(&game_controller::GameButton::ActionDown) {
                action_state.release(PlayerAction::ActionDown);
                action_state.press(PlayerAction::ActionDown);
            }
            if just_pressed.contains(&game_controller::GameButton::ActionRight) {
                action_state.release(PlayerAction::ActionRight);
                action_state.press(PlayerAction::ActionRight);
            }
            if just_pressed.contains(&game_controller::GameButton::ActionLeft) {
                action_state.release(PlayerAction::ActionLeft);
                action_state.press(PlayerAction::ActionLeft);
            }
        }
    }
}

pub enum Movement {
    Normal(direction::Direction),
}

fn handle_input(
    mut app_state: ResMut<State<AppState>>,
    players: Query<(Entity, &ActionState<PlayerAction>, &Transform, &Player)>,
    game_state: Res<game_state::GameState>,
    mut player_move_event_writer: EventWriter<PlayerMoveEvent>,
) {
    for (entity, action_state, transform, player) in players.iter() {
        //println!("T: {:?}", transform.translation);
        let mut direction = direction::Direction::NEUTRAL;

        for input_direction in PlayerAction::DIRECTIONS {
            if action_state.pressed(input_direction) {
                direction += input_direction.direction();
            }
        }

        if direction != direction::Direction::NEUTRAL {
            player_move_event_writer.send(PlayerMoveEvent {
                entity,
                movement: Movement::Normal(direction),
            });
        }

        if action_state.just_pressed(PlayerAction::ActionUp) {}
        if action_state.pressed(PlayerAction::ActionUp) {}

        if action_state.just_pressed(PlayerAction::ActionDown) {}

        if action_state.pressed(PlayerAction::ActionDown) {}

        if action_state.just_pressed(PlayerAction::ActionLeft) {}

        if action_state.pressed(PlayerAction::ActionLeft) {}

        if action_state.just_pressed(PlayerAction::ActionRight) {}

        if action_state.pressed(PlayerAction::ActionRight) {}
    }
}
