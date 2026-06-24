use bevy::{prelude::*, picking::prelude::MeshPickingPlugin, audio::Volume};
use std::collections::{BinaryHeap, HashMap, HashSet};

const BOARD_SIZE_I: usize = 5;
const BOARD_SIZE_J: usize = 5;

#[derive(Component)]
struct Tile {
    i: i32,
    j: i32,
}

#[derive(Component)]
struct TileHighlight {
    timer: Timer,
    original_material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct ObjectHighlight {
    timer: Timer,
    original_material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Robot {
    current_i: i32,
    current_j: i32,
    path: Vec<(i32, i32)>,
    move_speed: f32,
    waypoint_queue: Vec<WaypointTask>,
    wait_timer: Timer,
    battery: f32, // 0.0 to 100.0
    is_charging: bool,
    charge_timer: Timer,
    vacuum_sound: Option<Entity>,
    sound_volume: f32,
}

#[derive(Component)]
struct WaypointList;

#[derive(Component)]
struct WaypointButton {
    index: usize,
}

#[derive(Component)]
struct Ball {
    tile_i: i32,
    tile_j: i32,
}

#[derive(Component)]
struct PickedUpBall;

#[derive(Component)]
struct DropZone;

#[derive(Component)]
struct ChargingStation;

#[derive(Component)]
struct BatteryBar;

#[derive(Component)]
struct BatteryBarFill;

#[derive(Component)]
struct CleanedTile;

#[derive(Component)]
struct CleanlinessText;

#[derive(Component)]
struct VacuumSound {
    target_volume: f32,
}

#[derive(Resource)]
struct Cleanliness {
    cleaned_count: usize,
    total_tiles: usize,
    balls_collected: usize,
    total_balls: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WaypointType {
    MoveTo,
    PickUp,
    DropAway,
}

#[derive(Clone, Copy, Debug)]
struct WaypointTask {
    position: (i32, i32),
    task_type: WaypointType,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Robo Cleanup Game".into(),
                    resolution: (640, 360).into(),
                    ..default()
                }),
                ..default()
            }),
            MeshPickingPlugin,
        ))
        .insert_resource(Cleanliness {
            cleaned_count: 0,
            total_tiles: (BOARD_SIZE_I * BOARD_SIZE_J),
            balls_collected: 0,
            total_balls: 3, // We spawn 3 balls total
        })
        .add_systems(Startup, (setup_camera, setup_board, setup_ui).chain())
        .add_systems(Update, (update_tile_highlights, update_object_highlights, move_robot, clean_tiles, pickup_balls, dispose_balls, update_waypoint_ui, update_cleanliness_ui))
        .add_systems(Update, (update_battery_bar, handle_waypoint_button_click, update_vacuum_sound))
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Position camera for isometric view (centered at 0,0)
    let focus_point = Vec3::new(0.0, 0.0, 0.0);
    
    // Calculate isometric camera position (45 degrees around Y, ~35.264 degrees elevation)
    let distance = 30.0;
    let angle_y = std::f32::consts::PI / 4.0; // 45 degrees for isometric
    let angle_x = std::f32::consts::FRAC_PI_4.atan(); // ~35.264 degrees
    
    let camera_pos = Vec3::new(
        focus_point.x + distance * angle_x.cos() * angle_y.cos(),
        focus_point.y + distance * angle_x.sin(),
        focus_point.z + distance * angle_x.cos() * angle_y.sin(),
    );
    
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 0.01,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(camera_pos)
            .looking_at(focus_point, Vec3::Y),
    ));
}

fn setup_board(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Spawn a light
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            shadows_enabled: true,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0),
    ));

    // Spawn the robot at (0, 0)
    let mut wait_timer = Timer::from_seconds(2.0, TimerMode::Once);
    wait_timer.tick(std::time::Duration::from_secs(3)); // Start as finished
    
    commands.spawn((
        Robot {
            current_i: 0,
            current_j: 0,
            path: Vec::new(),
            move_speed: 2.0,
            waypoint_queue: Vec::new(),
            wait_timer,
            battery: 100.0,
            is_charging: false,
            charge_timer: Timer::from_seconds(5.0, TimerMode::Once),
            vacuum_sound: None,
            sound_volume: 0.0,
        },
        SceneRoot(asset_server.load("robot.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_scale(Vec3::splat(0.2)),
    ));

    // Create mesh and material for tiles
    let tile_mesh = meshes.add(Cuboid::new(1.0, 0.2, 1.0));
    let tile_material = materials.add(Color::srgb(0.2, 0.5, 0.3)); // Darker dirty tiles
    let clean_tile_material = materials.add(Color::srgb(0.4, 0.8, 0.5)); // Lighter clean tiles
    let highlight_material = materials.add(Color::srgb(1.0, 1.0, 0.0));
    
    // Create mesh and material for balls
    let ball_mesh = meshes.add(Sphere::new(0.075));
    let ball_material = materials.add(Color::WHITE);

    // Spawn the game board centered at (0, 0)
    let half_i = (BOARD_SIZE_I / 2) as i32;
    let half_j = (BOARD_SIZE_J / 2) as i32;
    
    for j in 0..BOARD_SIZE_J {
        for i in 0..BOARD_SIZE_I {
            let tile_i = i as i32 - half_i;
            let tile_j = j as i32 - half_j;
            
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(tile_material.clone()),
                Transform::from_xyz(tile_i as f32, -0.2, tile_j as f32),
                Tile { i: tile_i, j: tile_j },
            ))
            .observe(on_tile_click(highlight_material.clone(), tile_material.clone(), clean_tile_material.clone()));
        }
    }
    
    // Spawn 3 balls total on random tiles (not on robot's starting position or drop zone)
    let ball_positions = [
        (1, 1),
        (-2, 1),
        (2, -1),
    ];
    
    for (tile_i, tile_j) in ball_positions {
        commands.spawn((
            Ball { tile_i, tile_j },
            Mesh3d(ball_mesh.clone()),
            MeshMaterial3d(ball_material.clone()),
            Transform::from_xyz(tile_i as f32, 0.075, tile_j as f32),
        ))
        .observe(on_ball_click(ball_material.clone()));
    }
    
    // Spawn drop zone cylinder at tile (-2, -2)
    let cylinder_mesh = meshes.add(Cylinder::new(0.25, 0.25));
    let cylinder_material = materials.add(Color::srgb(0.8, 0.3, 0.3));
    
    commands.spawn((
        DropZone,
        Mesh3d(cylinder_mesh.clone()),
        MeshMaterial3d(cylinder_material.clone()),
        Transform::from_xyz(-2.0, 0.125, -2.0),
    ))
    .observe(on_dropzone_click(cylinder_material.clone()));
    
    // Spawn charging station cylinder at tile (2, -2)
    let charging_mesh = meshes.add(Cylinder::new(0.25, 0.25));
    let charging_material = materials.add(Color::srgb(0.3, 0.3, 0.8));
    
    commands.spawn((
        ChargingStation,
        Mesh3d(charging_mesh),
        MeshMaterial3d(charging_material),
        Transform::from_xyz(2.0, 0.125, -2.0),
    ));
    
    // Spawn 3D battery bar above robot spawn point
    // Background bar (dark)
    let bar_bg_mesh = meshes.add(Cuboid::new(1.0, 0.1, 0.05));
    let bar_bg_material = materials.add(Color::srgba(0.2, 0.2, 0.2, 0.8));
    
    commands.spawn((
        BatteryBar,
        Mesh3d(bar_bg_mesh),
        MeshMaterial3d(bar_bg_material),
        Transform::from_xyz(0.0, 1.2, 0.0), // Above robot
    ));
    
    // Fill bar (green, will shrink with battery level)
    let bar_fill_mesh = meshes.add(Cuboid::new(1.0, 0.08, 0.04));
    let bar_fill_material = materials.add(Color::srgb(0.3, 0.8, 0.3));
    
    commands.spawn((
        BatteryBarFill,
        Mesh3d(bar_fill_mesh),
        MeshMaterial3d(bar_fill_material),
        Transform::from_xyz(0.0, 1.2, 0.0), // Above robot
    ));
}

/// Returns an observer that handles tile clicks
fn on_tile_click(
    highlight_material: Handle<StandardMaterial>,
    original_material: Handle<StandardMaterial>,
    _clean_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Tile, &mut MeshMaterial3d<StandardMaterial>)>, Query<&mut Robot>, Commands) {
    move |event, mut tile_query, mut robot_query, mut commands| {
        if let Ok((tile, mut material)) = tile_query.get_mut(event.event_target()) {
            println!("Tile clicked at position: ({}, {}) - go to", tile.i, tile.j);
            
            // Change to highlight material
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(TileHighlight {
                timer: Timer::from_seconds(0.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
            
            // Add MoveTo waypoint to robot's queue
            if let Ok(mut robot) = robot_query.single_mut() {
                robot.waypoint_queue.push(WaypointTask {
                    position: (tile.i, tile.j),
                    task_type: WaypointType::MoveTo,
                });
                println!("Added waypoint to queue: go to ({}, {}). Queue size: {}", tile.i, tile.j, robot.waypoint_queue.len());
            }
        }
    }
}

/// Returns an observer that handles ball clicks
fn on_ball_click(
    original_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Ball, &mut MeshMaterial3d<StandardMaterial>)>, Query<&mut Robot>, Commands, ResMut<Assets<StandardMaterial>>) {
    move |event, mut ball_query, mut robot_query, mut commands, mut materials| {
        if let Ok((ball, mut material)) = ball_query.get_mut(event.event_target()) {
            println!("Ball clicked at position: ({}, {}) - pick up", ball.tile_i, ball.tile_j);
            
            // Create highlight material (bright yellow)
            let highlight_material = materials.add(Color::srgb(2.0, 2.0, 0.0));
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(ObjectHighlight {
                timer: Timer::from_seconds(1.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
            
            // Add PickUp waypoint to robot's queue
            if let Ok(mut robot) = robot_query.single_mut() {
                robot.waypoint_queue.push(WaypointTask {
                    position: (ball.tile_i, ball.tile_j),
                    task_type: WaypointType::PickUp,
                });
                println!("Added waypoint to queue: pick up ({}, {}). Queue size: {}", ball.tile_i, ball.tile_j, robot.waypoint_queue.len());
            }
        }
    }
}

/// Returns an observer that handles drop zone clicks
fn on_dropzone_click(
    original_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Transform, &mut MeshMaterial3d<StandardMaterial>), With<DropZone>>, Query<&mut Robot>, Commands, ResMut<Assets<StandardMaterial>>) {
    move |event, mut dropzone_query, mut robot_query, mut commands, mut materials| {
        if let Ok((dropzone_transform, mut material)) = dropzone_query.get_mut(event.event_target()) {
            let drop_i = dropzone_transform.translation.x.round() as i32;
            let drop_j = dropzone_transform.translation.z.round() as i32;
            
            println!("Drop zone clicked at position: ({}, {}) - drop away", drop_i, drop_j);
            
            // Create highlight material (bright yellow)
            let highlight_material = materials.add(Color::srgb(2.0, 2.0, 0.0));
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(ObjectHighlight {
                timer: Timer::from_seconds(1.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
            
            // Add DropAway waypoint to robot's queue
            if let Ok(mut robot) = robot_query.single_mut() {
                robot.waypoint_queue.push(WaypointTask {
                    position: (drop_i, drop_j),
                    task_type: WaypointType::DropAway,
                });
                println!("Added waypoint to queue: drop away ({}, {}). Queue size: {}", drop_i, drop_j, robot.waypoint_queue.len());
            }
        }
    }
}

fn update_tile_highlights(
    mut commands: Commands,
    mut tiles: Query<(Entity, &mut TileHighlight, &mut MeshMaterial3d<StandardMaterial>, Option<&CleanedTile>)>,
    time: Res<Time>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut highlight, mut material, is_cleaned) in tiles.iter_mut() {
        highlight.timer.tick(time.delta());
        
        if highlight.timer.is_finished() {
            // Restore to appropriate material based on whether tile is cleaned
            if is_cleaned.is_some() {
                // Tile is cleaned, restore to clean color
                material.0 = material_assets.add(Color::srgb(0.4, 0.8, 0.5));
            } else {
                // Tile is dirty, restore to original dirty color
                material.0 = highlight.original_material.clone();
            }
            commands.entity(entity).remove::<TileHighlight>();
        }
    }
}

fn update_object_highlights(
    mut commands: Commands,
    mut objects: Query<(Entity, &mut ObjectHighlight, &mut MeshMaterial3d<StandardMaterial>)>,
    time: Res<Time>,
) {
    for (entity, mut highlight, mut material) in objects.iter_mut() {
        highlight.timer.tick(time.delta());
        
        if highlight.timer.is_finished() {
            // Restore original material
            material.0 = highlight.original_material.clone();
            commands.entity(entity).remove::<ObjectHighlight>();
        }
    }
}

fn clean_tiles(
    mut commands: Commands,
    robot_query: Query<&Robot>,
    mut tile_query: Query<(Entity, &Tile, &mut MeshMaterial3d<StandardMaterial>), Without<CleanedTile>>,
    mut cleanliness: ResMut<Cleanliness>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(robot) = robot_query.single() else {
        return;
    };
    
    // Check each uncleaned tile
    for (tile_entity, tile, mut material) in tile_query.iter_mut() {
        if tile.i == robot.current_i && tile.j == robot.current_j {
            // Mark tile as cleaned and change to lighter color
            let clean_material = material_assets.add(Color::srgb(0.4, 0.8, 0.5));
            material.0 = clean_material;
            commands.entity(tile_entity).insert(CleanedTile);
            cleanliness.cleaned_count += 1;
            println!("Cleaned tile ({}, {}). Progress: {}/{}", tile.i, tile.j, cleanliness.cleaned_count, cleanliness.total_tiles);
        }
    }
}

fn pickup_balls(
    robot_query: Query<&Transform, With<Robot>>,
    mut picked_balls_query: Query<&mut Transform, (With<Ball>, With<PickedUpBall>, Without<Robot>)>,
) {
    let Ok(robot_transform) = robot_query.single() else {
        return;
    };
    
    // Update positions of picked up balls to float above robot
    let mut height_offset = 0.8;
    for mut ball_transform in picked_balls_query.iter_mut() {
        ball_transform.translation.x = robot_transform.translation.x;
        ball_transform.translation.y = height_offset;
        ball_transform.translation.z = robot_transform.translation.z;
        height_offset += 0.2; // Stack balls on top of each other (reduced for smaller balls)
    }
}

fn dispose_balls(
    mut commands: Commands,
    robot_query: Query<&Robot>,
    picked_balls_query: Query<Entity, (With<Ball>, With<PickedUpBall>)>,
) {
    let Ok(robot) = robot_query.single() else {
        return;
    };
    
    // Check if robot is on the drop zone tile (-2, -2) and has finished waiting
    if robot.current_i == -2 && robot.current_j == -2 && robot.wait_timer.is_finished() {
        // Check if the last completed task was a DropAway
        if robot.waypoint_queue.is_empty() || (robot.path.is_empty() && !robot.waypoint_queue.is_empty()) {
            // Dispose all picked up balls
            let ball_count = picked_balls_query.iter().count();
            if ball_count > 0 {
                for ball_entity in picked_balls_query.iter() {
                    commands.entity(ball_entity).despawn();
                }
                println!("Disposed {} ball(s) at drop zone", ball_count);
            }
        }
    }
}

fn move_robot(
    mut commands: Commands,
    mut robot_query: Query<(&mut Robot, &mut Transform)>,
    time: Res<Time>,
    mut ball_query: Query<(Entity, &Ball, &mut Transform), (Without<Robot>, Without<PickedUpBall>)>,
    mut cleanliness: ResMut<Cleanliness>,
) {
    for (mut robot, mut transform) in robot_query.iter_mut() {
        // Handle charging
        if robot.is_charging {
            robot.charge_timer.tick(time.delta());
            
            // Gradually charge the battery over 5 seconds
            let charge_rate = 100.0 / 5.0; // 20% per second
            robot.battery = (robot.battery + charge_rate * time.delta_secs()).min(100.0);
            
            if robot.charge_timer.just_finished() {
                robot.battery = 100.0;
                robot.is_charging = false;
                println!("Robot fully charged! Battery: 100%");
            }
            continue;
        }
        
        // Check if battery is depleted
        if robot.battery <= 0.0 && !robot.is_charging {
            // Clear current queue and path, go to charging station
            robot.waypoint_queue.clear();
            robot.path.clear();
            
            // Calculate path to charging station at (2, -2)
            let start = (robot.current_i, robot.current_j);
            let charging_pos = (2, -2);
            
            if let Some(path) = find_path(start, charging_pos) {
                robot.path = path;
                println!("Battery depleted! Returning to charging station at (2, -2)");
            }
        }
        
        // Check if at charging station with depleted battery
        if robot.current_i == 2 && robot.current_j == -2 && robot.battery <= 0.0 {
            robot.is_charging = true;
            robot.charge_timer.reset();
            robot.path.clear();
            println!("Charging... (5 seconds)");
            continue;
        }
        // If waiting at a waypoint, don't do anything else
        if !robot.wait_timer.is_finished() {
            robot.wait_timer.tick(time.delta());
            if robot.wait_timer.just_finished() {
                if !robot.waypoint_queue.is_empty() {
                    let current_waypoint = robot.waypoint_queue[0];
                    
                    // If this was a pickup task, pick up one ball from this tile
                    if current_waypoint.task_type == WaypointType::PickUp {
                        // Find a ball on this tile and pick it up
                        for (ball_entity, ball, _) in ball_query.iter_mut() {
                            if ball.tile_i == robot.current_i && ball.tile_j == robot.current_j {
                                commands.entity(ball_entity).insert(PickedUpBall);
                                cleanliness.balls_collected += 1;
                                println!("Picked up ball from ({}, {}). Balls collected: {}/{}", robot.current_i, robot.current_j, cleanliness.balls_collected, cleanliness.total_balls);
                                break; // Only pick up one ball
                            }
                        }
                    }
                    
                    println!("Robot finished waiting at waypoint: ({}, {})", robot.current_i, robot.current_j);
                    robot.waypoint_queue.remove(0);
                }
            }
            continue;
        }
        
        // If no current path but waypoints in queue, calculate path to next waypoint
        if robot.path.is_empty() && !robot.waypoint_queue.is_empty() {
            let start = (robot.current_i, robot.current_j);
            let goal = robot.waypoint_queue[0].position;
            
            if let Some(path) = find_path(start, goal) {
                println!("Calculating path to waypoint: {:?}", goal);
                robot.path = path;
            } else {
                // If no path found, remove the waypoint
                robot.waypoint_queue.remove(0);
            }
        }
        
        if robot.path.is_empty() {
            continue;
        }
        
        let target = robot.path[0];
        let target_pos = Vec3::new(target.0 as f32, 0.0, target.1 as f32);
        let mut current_pos = transform.translation;
        current_pos.y = 0.0; // Ignore Y for distance calculation
        
        let direction = (target_pos - current_pos).normalize_or_zero();
        let distance = current_pos.distance(target_pos);
        
        if distance < 0.1 {
            // Reached waypoint
            robot.current_i = target.0;
            robot.current_j = target.1;
            robot.path.remove(0);
            
            // Check if reached the final destination in the current waypoint
            if robot.path.is_empty() && !robot.waypoint_queue.is_empty() {
                let reached_waypoint = robot.waypoint_queue[0].position;
                if robot.current_i == reached_waypoint.0 && robot.current_j == reached_waypoint.1 {
                    println!("Robot reached waypoint: ({}, {}), waiting 2 seconds...", robot.current_i, robot.current_j);
                    robot.wait_timer.reset();
                }
            }
        } else {
            // Move towards waypoint
            let movement = direction * robot.move_speed * time.delta_secs();
            transform.translation.x += movement.x;
            transform.translation.z += movement.z;
            
            // Deplete battery while moving (10% per unit of distance)
            let distance_moved = movement.length();
            robot.battery = (robot.battery - distance_moved * 10.0).max(0.0);
            
            // Rotate to face movement direction
            if direction.length_squared() > 0.001 {
                let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
                transform.rotation = transform.rotation.slerp(target_rotation, time.delta_secs() * 10.0);
            }
        }
    }
}

fn setup_ui(mut commands: Commands) {
    // Tasks panel on the right
    commands.spawn((
        WaypointList,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            right: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
        BorderColor::all(Color::srgb(0.6, 0.6, 0.6)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Tasks"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
    
    // Cleanliness panel on the left
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            padding: UiRect::all(Val::Px(10.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
        BorderColor::all(Color::srgb(0.6, 0.6, 0.6)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Cleanliness"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        parent.spawn((
            CleanlinessText,
            Text::new("0%"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
        ));
    });
}

#[derive(Clone, Eq, PartialEq)]
struct PathNode {
    pos: (i32, i32),
    f_score: i32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn find_path(start: (i32, i32), goal: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    let half_i = (BOARD_SIZE_I / 2) as i32;
    let half_j = (BOARD_SIZE_J / 2) as i32;
    
    let mut open_set = BinaryHeap::<PathNode>::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut closed_set = HashSet::new();
    
    g_score.insert(start, 0);
    open_set.push(PathNode {
        pos: start,
        f_score: heuristic(start, goal),
    });
    
    while let Some(current_node) = open_set.pop() {
        let current = current_node.pos;
        
        if current == goal {
            return Some(reconstruct_path(&came_from, current));
        }
        
        closed_set.insert(current);
        
        // Check 4 neighboring tiles
        let neighbors = [
            (current.0 + 1, current.1),
            (current.0 - 1, current.1),
            (current.0, current.1 + 1),
            (current.0, current.1 - 1),
        ];
        
        for neighbor in neighbors {
            // Check if neighbor is within bounds (inclusive of boundaries)
            if neighbor.0 < -half_i || neighbor.0 > half_i || 
               neighbor.1 < -half_j || neighbor.1 > half_j {
                continue;
            }
            
            if closed_set.contains(&neighbor) {
                continue;
            }
            
            let tentative_g_score = g_score.get(&current).unwrap_or(&i32::MAX) + 1;
            
            if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                came_from.insert(neighbor, current);
                g_score.insert(neighbor, tentative_g_score);
                
                let f_score = tentative_g_score + heuristic(neighbor, goal);
                open_set.push(PathNode {
                    pos: neighbor,
                    f_score,
                });
            }
        }
    }
    
    None
}

fn heuristic(pos: (i32, i32), goal: (i32, i32)) -> i32 {
    (pos.0 - goal.0).abs() + (pos.1 - goal.1).abs()
}

fn reconstruct_path(came_from: &HashMap<(i32, i32), (i32, i32)>, mut current: (i32, i32)) -> Vec<(i32, i32)> {
    let mut path = vec![current];
    
    while let Some(&prev) = came_from.get(&current) {
        current = prev;
        path.push(current);
    }
    
    path.reverse();
    path.remove(0); // Remove starting position
    path
}

fn update_waypoint_ui(
    mut commands: Commands,
    robot_query: Query<&Robot>,
    waypoint_list_query: Query<Entity, With<WaypointList>>,
    waypoint_buttons_query: Query<Entity, With<WaypointButton>>,
) {
    let Ok(robot) = robot_query.single() else {
        return;
    };
    
    let Ok(waypoint_list_entity) = waypoint_list_query.single() else {
        return;
    };
    
    // Clear existing buttons
    for button_entity in waypoint_buttons_query.iter() {
        commands.entity(button_entity).despawn();
    }
    
    // Spawn new buttons for each waypoint
    commands.entity(waypoint_list_entity).with_children(|parent| {
        for (index, waypoint_task) in robot.waypoint_queue.iter().enumerate() {
            let button_color = if index == 0 {
                // Check if robot is at the waypoint (waiting) or still traveling
                if !robot.wait_timer.is_finished() {
                    // Robot is waiting at this waypoint
                    Color::srgb(0.8, 0.5, 0.2) // Orange for waiting
                } else {
                    // Robot is still traveling to this waypoint
                    Color::srgb(0.9, 0.9, 0.2) // Yellow for traveling
                }
            } else {
                Color::srgb(0.2, 0.2, 0.8) // Blue for queued waypoints
            };
            
            let task_text = match waypoint_task.task_type {
                WaypointType::PickUp => format!("pick up ({}, {})", waypoint_task.position.0, waypoint_task.position.1),
                WaypointType::MoveTo => format!("go to ({}, {})", waypoint_task.position.0, waypoint_task.position.1),
                WaypointType::DropAway => format!("drop away ({}, {})", waypoint_task.position.0, waypoint_task.position.1),
            };
            
            parent.spawn((
                WaypointButton { index },
                Button,
                Node {
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(button_color),
            )).with_children(|button_parent| {
                button_parent.spawn((
                    Text::new(task_text),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        }
    });
}

fn update_cleanliness_ui(
    cleanliness: Res<Cleanliness>,
    mut text_query: Query<&mut Text, With<CleanlinessText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    
    // Calculate total cleanliness: tiles + balls
    let total_items = cleanliness.total_tiles + cleanliness.total_balls;
    let cleaned_items = cleanliness.cleaned_count + cleanliness.balls_collected;
    let percentage = (cleaned_items as f32 / total_items as f32 * 100.0) as usize;
    
    text.0 = format!("{}%\nTiles: {}/{}\nBalls: {}/{}", 
        percentage, 
        cleanliness.cleaned_count, cleanliness.total_tiles,
        cleanliness.balls_collected, cleanliness.total_balls
    );
}

fn update_battery_bar(
    robot_query: Query<&Robot>,
    robot_transform_query: Query<&Transform, With<Robot>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut battery_bg_query: Query<&mut Transform, (With<BatteryBar>, Without<Robot>, Without<Camera3d>, Without<BatteryBarFill>)>,
    mut battery_fill_query: Query<(&mut Transform, &mut MeshMaterial3d<StandardMaterial>), (With<BatteryBarFill>, Without<Robot>, Without<Camera3d>, Without<BatteryBar>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(robot) = robot_query.single() else {
        return;
    };
    
    let Ok(robot_transform) = robot_transform_query.single() else {
        return;
    };
    
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    
    // Position battery bar background above robot
    if let Ok(mut bg_transform) = battery_bg_query.single_mut() {
        bg_transform.translation = robot_transform.translation + Vec3::new(0.0, 1.2, 0.0);
        
        // Make it face the camera
        let to_camera = (camera_transform.translation - bg_transform.translation).normalize();
        bg_transform.rotation = Quat::from_rotation_y(to_camera.x.atan2(to_camera.z));
    }
    
    // Position and scale battery bar fill
    if let Ok((mut fill_transform, mut fill_material)) = battery_fill_query.single_mut() {
        let battery_percent = robot.battery / 100.0;
        
        // Position above robot at same location as background
        fill_transform.translation = robot_transform.translation + Vec3::new(0.0, 1.2, 0.0);
        
        // Scale based on battery level
        fill_transform.scale = Vec3::new(battery_percent, 1.0, 1.0);
        
        // Make it face the camera
        let to_camera = (camera_transform.translation - fill_transform.translation).normalize();
        fill_transform.rotation = Quat::from_rotation_y(to_camera.x.atan2(to_camera.z));
        
        // Update color based on battery level and charging state
        let color = if robot.is_charging {
            Color::srgb(0.8, 0.8, 0.3) // Yellow when charging
        } else if robot.battery > 50.0 {
            Color::srgb(0.3, 0.8, 0.3) // Green when high
        } else if robot.battery > 25.0 {
            Color::srgb(0.8, 0.6, 0.0) // Orange when medium
        } else {
            Color::srgb(0.8, 0.2, 0.2) // Red when low
        };
        
        fill_material.0 = materials.add(color);
    }
}

fn handle_waypoint_button_click(
    mut robot_query: Query<&mut Robot>,
    button_query: Query<(&WaypointButton, &Interaction), Changed<Interaction>>,
) {
    let Ok(mut robot) = robot_query.single_mut() else {
        return;
    };
    
    for (waypoint_button, interaction) in button_query.iter() {
        if *interaction == Interaction::Pressed {
            if waypoint_button.index == 0 {
                // Clicking the current waypoint skips the wait timer
                if !robot.wait_timer.is_finished() {
                    println!("Skipping wait timer for current waypoint");
                    robot.wait_timer = Timer::from_seconds(2.0, TimerMode::Once);
                    robot.wait_timer.tick(std::time::Duration::from_secs(3)); // Mark as finished
                } else if !robot.waypoint_queue.is_empty() {
                    // If not waiting, remove the current waypoint
                    let removed = robot.waypoint_queue.remove(0);
                    println!("Removed current waypoint ({}, {}) from queue", removed.position.0, removed.position.1);
                    robot.path.clear(); // Clear the current path
                }
            } else if waypoint_button.index < robot.waypoint_queue.len() {
                // Clicking other buttons removes them from the queue
                let removed = robot.waypoint_queue.remove(waypoint_button.index);
                println!("Removed waypoint ({}, {}) from queue", removed.position.0, removed.position.1);
            }
        }
    }
}

fn update_vacuum_sound(
    mut commands: Commands,
    mut robot_query: Query<&mut Robot>,
    mut vacuum_sound_query: Query<(&mut VacuumSound, &mut AudioSink)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    let Ok(mut robot) = robot_query.single_mut() else {
        return;
    };
    
    // Check if robot is moving (has a non-empty path and not charging)
    let is_moving = !robot.path.is_empty() && !robot.is_charging;
    
    let target_volume = if is_moving { 0.5 } else { 0.0 };
    let fade_speed = 1.5; // Volume units per second
    
    // Gradually adjust volume
    if robot.sound_volume < target_volume {
        robot.sound_volume = (robot.sound_volume + fade_speed * time.delta_secs()).min(target_volume);
    } else if robot.sound_volume > target_volume {
        robot.sound_volume = (robot.sound_volume - fade_speed * time.delta_secs()).max(target_volume);
    }
    
    // If sound entity exists
    if let Some(sound_entity) = robot.vacuum_sound {
        if let Ok((mut vacuum_sound, mut audio_sink)) = vacuum_sound_query.get_mut(sound_entity) {
            // Update the volume
            audio_sink.set_volume(Volume::Linear(robot.sound_volume));
            vacuum_sound.target_volume = target_volume;
            
            // If faded out completely, despawn
            if robot.sound_volume <= 0.0 && target_volume == 0.0 {
                commands.entity(sound_entity).despawn();
                robot.vacuum_sound = None;
            }
        }
    } else {
        // No sound entity exists
        if is_moving {
            // Robot started moving - spawn the sound with volume 0, it will fade in
            println!("Starting vacuum sound");
            robot.sound_volume = 0.0;
            let sound_entity = commands.spawn((
                VacuumSound { target_volume },
                AudioPlayer::new(asset_server.load("robot_vacuum.mp3")),
                PlaybackSettings::LOOP,
            )).id();
            robot.vacuum_sound = Some(sound_entity);
        }
    }
}
