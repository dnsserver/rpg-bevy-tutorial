use bevy::{
    prelude::*,
    //render::{camera::Camera2d},
    sprite::collide_aabb::collide,
};
use bevy_inspector_egui::Inspectable;

use crate::{
    ascii::{AsciiSheet},
    combat::CombatStats,
    fadeout::create_fadeout,
    graphics::{CharacterSheet, FacingDirection, FrameAnimation, PlayerGraphics},
    tilemap::{EncounterSpawner, TileCollider},
    GameState, TILE_SIZE,
};

pub struct PlayerPlugin;

struct DangerousGrounds(bool);

#[derive(Component, Inspectable)]
pub struct Player {
    pub active: bool,
    pub exp: usize,
    pub max_steps: u32,
    pub step: u32,
}

impl Player {
    pub fn give_exp(&mut self, exp: usize, stats: &mut CombatStats) -> bool {
        self.exp += exp;
        if self.exp >= 50 {
            stats.health += 2;
            stats.max_health += 2;
            stats.attack += 1;
            stats.defense += 1;
            self.exp -= 50;
            return true;
        }
        false
    }
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DangerousGrounds>()
            .add_system_set(SystemSet::on_resume(GameState::Overworld).with_system(show_player))
            .add_system_set(SystemSet::on_pause(GameState::Overworld).with_system(hide_player))
            .add_system_set(
                SystemSet::on_update(GameState::Overworld)
                    .with_system(player_encounter_checking.after(player_movement))
                    .with_system(camera_follow.after(player_movement))
                    .with_system(player_movement),
            )
            .add_system_set(SystemSet::on_enter(GameState::Overworld).with_system(spawn_player));
    }
}

fn hide_player(
    mut player_query: Query<&mut Visibility, With<Player>>,
    children_query: Query<&Children, With<Player>>,
    mut child_visibility_query: Query<&mut Visibility, Without<Player>>,
) {
    let mut player_vis = player_query.single_mut();
    player_vis.is_visible = false;

    if let Ok(children) = children_query.get_single() {
        for child in children.iter() {
            if let Ok(mut child_vis) = child_visibility_query.get_mut(*child) {
                child_vis.is_visible = false;
            }
        }
    }
}

fn show_player(
    mut player_query: Query<(&mut Player, &mut Visibility)>,
    children_query: Query<&Children, With<Player>>,
    mut child_visibility_query: Query<&mut Visibility, Without<Player>>,
) {
    let (mut player, mut player_vis) = player_query.single_mut();
    player.active = true;
    player_vis.is_visible = true;

    if let Ok(children) = children_query.get_single() {
        for child in children.iter() {
            if let Ok(mut child_vis) = child_visibility_query.get_mut(*child) {
                child_vis.is_visible = true;
            }
        }
    }
}

fn player_encounter_checking(
    player_query: Query<&Transform, With<Player>>,
    encounter_query: Query<&Transform, (With<EncounterSpawner>, Without<Player>)>,
    mut dangerous_grounds: EventWriter<DangerousGrounds>,
) {
    let player_transform = player_query.single();
    let player_translation = player_transform.translation;

    if encounter_query
            .iter()
            .any(|&transform| wall_collision_check(player_translation, transform.translation))
    {
        dangerous_grounds.send(DangerousGrounds(true));
    }else{
        dangerous_grounds.send(DangerousGrounds(false));
    }
}

fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (Without<Player>, With<Camera2d>)>,
) {
    let player_transform = player_query.single();
    let mut camera_transform = camera_query.single_mut();

    camera_transform.translation.x = player_transform.translation.x;
    camera_transform.translation.y = player_transform.translation.y;
}

fn player_movement(
    mut commands: Commands,
    mut player_query: Query<(&mut Player, &mut Transform, &mut PlayerGraphics)>,
    wall_query: Query<&Transform, (With<TileCollider>, Without<Player>)>,
    keyboard: Res<Input<KeyCode>>,
    mut dangerous_grounds: EventReader<DangerousGrounds>,

    ascii: Res<AsciiSheet>,
    //time: Res<Time>,
) {
    let (mut player, mut transform, mut graphics) = player_query.single_mut();

    if !player.active {
        return;
    }

    let danger = dangerous_grounds.iter().any(|b| b.0);
        

    let mut delta_movement = Vec3::new(0.,0.,0.);
    if keyboard.just_pressed(KeyCode::W){
        delta_movement.y += TILE_SIZE;
    }else if keyboard.just_pressed(KeyCode::S){
        delta_movement.y -= TILE_SIZE;
    }else if keyboard.just_pressed(KeyCode::A){
        delta_movement.x -= TILE_SIZE;
    }else if keyboard.just_pressed(KeyCode::D){
        delta_movement.x += TILE_SIZE;
    }

    let target = transform.translation + delta_movement;
    if !wall_query
        .iter()
        .any(|&transform| wall_collision_check(target, transform.translation))
    {
        if delta_movement.y != 0.0 || delta_movement.x != 0.0 {
            if danger {
                player.step += 1;
            }

            if player.step > player.max_steps {
                player.active = false;
                player.step = 0;
                create_fadeout(&mut commands, Some(GameState::Combat), &ascii);
                return;
            }
            if delta_movement.y > 0.0 {
                graphics.facing = FacingDirection::Up;
            } else if delta_movement.y < 0.0 {
                graphics.facing = FacingDirection::Down;
            }else if delta_movement.x > 0.0 {
                graphics.facing = FacingDirection::Right;
            } else {
                graphics.facing = FacingDirection::Left;
            }
        }
        transform.translation = target;
    }
}

fn wall_collision_check(target_player_pos: Vec3, wall_translation: Vec3) -> bool {
    let collision = collide(
        target_player_pos,
        Vec2::splat(TILE_SIZE * 0.9),
        wall_translation,
        Vec2::splat(TILE_SIZE),
    );
    collision.is_some()
}

fn spawn_player(mut commands: Commands, characters: Res<CharacterSheet>) {
    commands
        .spawn_bundle(SpriteSheetBundle {
            sprite: TextureAtlasSprite {
                index: characters.player_down[0],
                custom_size: Some(Vec2::splat(TILE_SIZE)),
                ..default()
            },
            transform: Transform::from_xyz(2.0 * TILE_SIZE, -2.0 * TILE_SIZE, 900.0),
            texture_atlas: characters.handle.clone(),
            ..default()
        })
        .insert(FrameAnimation {
            timer: Timer::from_seconds(0.2, true),
            frames: characters.player_down.to_vec(),
            current_frame: 0,
        })
        .insert(PlayerGraphics {
            facing: FacingDirection::Down,
        })
        .insert(Name::new("Player"))
        .insert(Player {
            active: true,
            exp: 0,
            step: 0,
            max_steps: 5,
        })
        .insert(CombatStats {
            health: 10,
            max_health: 10,
            attack: 2,
            defense: 1,
        });
}
