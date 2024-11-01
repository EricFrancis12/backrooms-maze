mod maze;
mod utils;

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_third_person_camera::*;
use maze::{calc_maze_dims, maze_from_seed};
use std::{
    borrow::Cow,
    env,
    f32::consts::PI,
    fmt::{Display, Formatter, Result},
};
use utils::dev::write_maze_to_html_file;

const CHUNK_SIZE: f32 = 32.0;
const CELL_SIZE: f32 = 4.0;

const PLAYER_SIZE: f32 = 1.0;
const DEFAULT_PLAYER_SPEED: f32 = 8.0;

const CAMERA_X: f32 = -2.0;
const CAMERA_Y: f32 = 2.5;
const CAMERA_Z: f32 = 5.0;

const SEED: u32 = 1234;
const HTML_FILE_OUTPUT_PATH: &str = "maze.html";

#[derive(Debug)]
enum ArgName {
    Html,
}

impl Display for ArgName {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ArgName::Html => write!(f, "html"),
        }
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Speed(f32);

#[derive(Component)]
struct Chunk;

#[derive(Clone, Component, Debug, Default)]
pub struct Cell {
    wall_top: bool,
    wall_bottom: bool,
    wall_left: bool,
    wall_right: bool,
}

fn spawn_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk_bundle = (
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(CHUNK_SIZE, CHUNK_SIZE)),
            material: materials.add(Color::linear_rgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        Chunk,
        Name::new("Chunk"),
    );

    commands.spawn(chunk_bundle).with_children(|parent| {
        // One maze is created per chunk
        let (height, width) = calc_maze_dims(CHUNK_SIZE, CELL_SIZE);
        let maze = &maze_from_seed(SEED, height, width);

        for (x, row) in maze.iter().enumerate() {
            for (z, cell) in row.iter().enumerate() {
                let cell_bundle = (
                    PbrBundle {
                        mesh: meshes.add(Plane3d::default().mesh().size(CELL_SIZE, CELL_SIZE)),
                        material: materials.add(Color::linear_rgba(0.55, 0.0, 0.0, 1.0)),
                        transform: Transform::from_xyz(
                            calc_transform_pos(x),
                            0.0,
                            calc_transform_pos(z),
                        ),
                        ..default()
                    },
                    cell.clone(),
                    Name::new(format!("Cell ({},{})", x, z)),
                );
                parent.spawn(cell_bundle).with_children(|grandparent| {
                    // Top wall
                    if cell.wall_top {
                        let mut transform =
                            Transform::from_xyz(CELL_SIZE / 2.0, CELL_SIZE / 2.0, 0.0);
                        let z_90_deg_rotation = Quat::from_rotation_z(PI / 2.0);
                        transform.rotate(z_90_deg_rotation);

                        grandparent.spawn(new_cell_wall_bundle(
                            "Top Wall",
                            transform,
                            &mut meshes,
                            &mut materials,
                        ));
                    }

                    // Left wall
                    if cell.wall_left {
                        let mut transform =
                            Transform::from_xyz(0.0, CELL_SIZE / 2.0, CELL_SIZE / 2.0);
                        let x_270_deg_rotation = Quat::from_rotation_x(PI * 3.0 / 2.0);
                        transform.rotate(x_270_deg_rotation);

                        grandparent.spawn(new_cell_wall_bundle(
                            "Left Wall",
                            transform,
                            &mut meshes,
                            &mut materials,
                        ));
                    }

                    // Bottom wall
                    if cell.wall_bottom {
                        let mut transform =
                            Transform::from_xyz(-CELL_SIZE / 2.0, CELL_SIZE / 2.0, 0.0);
                        let z_270_deg_rotation = Quat::from_rotation_z(PI * 3.0 / 2.0);
                        transform.rotate(z_270_deg_rotation);

                        grandparent.spawn(new_cell_wall_bundle(
                            "Bottom Wall",
                            transform,
                            &mut meshes,
                            &mut materials,
                        ));
                    }

                    // Right wall
                    if cell.wall_right {
                        let mut transform =
                            Transform::from_xyz(0.0, CELL_SIZE / 2.0, -CELL_SIZE / 2.0);
                        let x_90_deg_rotation = Quat::from_rotation_x(PI / 2.0);
                        transform.rotate(x_90_deg_rotation);

                        grandparent.spawn(new_cell_wall_bundle(
                            "Right Wall",
                            transform,
                            &mut meshes,
                            &mut materials,
                        ));
                    }
                });
            }
        }
    });
}

fn new_cell_wall_bundle(
    name: impl Into<Cow<'static, str>>,
    transform: Transform,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> impl Bundle {
    (
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(CELL_SIZE, CELL_SIZE)),
            material: materials.add(Color::linear_rgba(0.15, 0.0, 0.55, 1.0)),
            transform,
            ..default()
        },
        Name::new(name),
    )
}

fn calc_transform_pos(index: usize) -> f32 {
    let num_cells_per_chunk = (CHUNK_SIZE / CELL_SIZE) as usize;
    let mut positions = vec![CELL_SIZE / 2.0, -CELL_SIZE / 2.0];
    while positions.len() < num_cells_per_chunk {
        positions.insert(0, positions[0] + CELL_SIZE);
        positions.push(positions.last().unwrap() - CELL_SIZE);
    }

    positions.get(index).unwrap().to_owned()
}

fn spawn_camera(mut commands: Commands) {
    let camera_bundle = (
        Camera3dBundle {
            transform: Transform::from_xyz(CAMERA_X, CAMERA_Y, CAMERA_Z)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        ThirdPersonCamera::default(),
        Name::new("Camera"),
    );

    commands.spawn(camera_bundle);
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_bundle = (
        PbrBundle {
            mesh: meshes.add(Cuboid::new(PLAYER_SIZE, PLAYER_SIZE, PLAYER_SIZE)),
            material: materials.add(Color::linear_rgba(0.0, 0.0, 0.3, 1.0)),
            transform: Transform::from_xyz(0.0, PLAYER_SIZE / 2.0, 0.0),
            ..default()
        },
        Player,
        ThirdPersonCameraTarget,
        Speed(DEFAULT_PLAYER_SPEED),
        Name::new("Player"),
    );

    commands.spawn(player_bundle);
}

fn player_movement(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player_query: Query<(&mut Transform, &Speed), With<Player>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Player>)>,
) {
    for (mut player_transform, player_speed) in player_query.iter_mut() {
        let camera_transform = match camera_query.get_single() {
            Ok(ct) => ct,
            Err(err) => Err(format!("Error retrieving camera: {}", err)).unwrap(),
        };

        let mut direction = Vec3::default();

        // Forward
        if keys.pressed(KeyCode::KeyW) {
            let d = camera_transform.forward();
            direction.x += d.x;
            direction.z += d.z;
        }
        // Back
        if keys.pressed(KeyCode::KeyS) {
            let d = camera_transform.back();
            direction.x += d.x;
            direction.z += d.z;
        }
        // Left
        if keys.pressed(KeyCode::KeyA) {
            let d = camera_transform.left();
            direction.x += d.x;
            direction.z += d.z;
        }
        // Right
        if keys.pressed(KeyCode::KeyD) {
            let d = camera_transform.right();
            direction.x += d.x;
            direction.z += d.z;
        }

        let movement = direction.normalize_or_zero() * player_speed.0 * time.delta_seconds();
        player_transform.translation += movement;

        if direction.length_squared() > 0.0 {
            player_transform.look_to(direction, Vec3::Y);
        }
    }
}

fn main() {
    assert_eq!(
        CHUNK_SIZE % CELL_SIZE,
        0.0,
        "expected chunk size ({}) to be divisible by cell size ({})",
        CHUNK_SIZE,
        CELL_SIZE,
    );

    let args: Vec<String> = env::args().collect();

    if args.contains(&ArgName::Html.to_string()) {
        let (height, width) = calc_maze_dims(CHUNK_SIZE, CELL_SIZE);
        let maze = &maze_from_seed(SEED, height, width);
        write_maze_to_html_file(maze, HTML_FILE_OUTPUT_PATH).unwrap();
    }

    App::new()
        .add_plugins((
            DefaultPlugins,
            ThirdPersonCameraPlugin,
            WorldInspectorPlugin::new(),
        ))
        .add_systems(Startup, (spawn_chunks, spawn_camera, spawn_player))
        .add_systems(Update, player_movement)
        .run();
}
