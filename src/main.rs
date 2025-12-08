use bevy::{prelude::*, picking::prelude::MeshPickingPlugin};
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
struct Robot {
    current_i: i32,
    current_j: i32,
    path: Vec<(i32, i32)>,
    move_speed: f32,
    waypoint_queue: Vec<WaypointTask>,
    wait_timer: Timer,
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum WaypointType {
    MoveTo,
    PickUp,
}

#[derive(Clone, Copy, Debug)]
struct WaypointTask {
    position: (i32, i32),
    task_type: WaypointType,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, (setup_camera, setup_board, setup_ui).chain())
        .add_systems(Update, (update_tile_highlights, move_robot, pickup_balls, dispose_balls, update_waypoint_ui, handle_waypoint_button_click))
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
        },
        SceneRoot(asset_server.load("robot.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_scale(Vec3::splat(0.2)),
    ));

    // Create mesh and material for tiles
    let tile_mesh = meshes.add(Cuboid::new(1.0, 0.2, 1.0));
    let tile_material = materials.add(Color::srgb(0.3, 0.7, 0.4));
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
            .observe(on_tile_click(highlight_material.clone(), tile_material.clone()));
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
        ));
    }
    
    // Spawn drop zone cylinder at tile (-2, -2)
    let cylinder_mesh = meshes.add(Cylinder::new(0.25, 0.25));
    let cylinder_material = materials.add(Color::srgb(0.8, 0.3, 0.3));
    
    commands.spawn((
        DropZone,
        Mesh3d(cylinder_mesh),
        MeshMaterial3d(cylinder_material),
        Transform::from_xyz(-2.0, 0.125, -2.0),
    ));
}

/// Returns an observer that handles tile clicks
fn on_tile_click(
    highlight_material: Handle<StandardMaterial>,
    original_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Tile, &mut MeshMaterial3d<StandardMaterial>)>, Query<&mut Robot>, Query<&Ball>, Commands) {
    move |event, mut tile_query, mut robot_query, ball_query, mut commands| {
        if let Ok((tile, mut material)) = tile_query.get_mut(event.event_target()) {
            // Check if there's a ball on this tile
            let has_ball = ball_query.iter().any(|ball| {
                ball.tile_i == tile.i && ball.tile_j == tile.j
            });
            
            let task_type = if has_ball {
                WaypointType::PickUp
            } else {
                WaypointType::MoveTo
            };
            
            let task_name = match task_type {
                WaypointType::PickUp => "pick up",
                WaypointType::MoveTo => "go to",
            };
            
            println!("Tile clicked at position: ({}, {}) - {}", tile.i, tile.j, task_name);
            
            // Change to highlight material
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(TileHighlight {
                timer: Timer::from_seconds(0.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
            
            // Add waypoint to robot's queue
            if let Ok(mut robot) = robot_query.single_mut() {
                robot.waypoint_queue.push(WaypointTask {
                    position: (tile.i, tile.j),
                    task_type,
                });
                println!("Added waypoint to queue: {} ({}, {}). Queue size: {}", task_name, tile.i, tile.j, robot.waypoint_queue.len());
            }
        }
    }
}

fn update_tile_highlights(
    mut commands: Commands,
    mut tiles: Query<(Entity, &mut TileHighlight, &mut MeshMaterial3d<StandardMaterial>)>,
    time: Res<Time>,
) {
    for (entity, mut highlight, mut material) in tiles.iter_mut() {
        highlight.timer.tick(time.delta());
        
        if highlight.timer.is_finished() {
            // Restore original material
            material.0 = highlight.original_material.clone();
            commands.entity(entity).remove::<TileHighlight>();
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
    robot_query: Query<&Transform, With<Robot>>,
    drop_zone_query: Query<&Transform, With<DropZone>>,
    picked_balls_query: Query<Entity, (With<Ball>, With<PickedUpBall>)>,
) {
    let Ok(robot_transform) = robot_query.single() else {
        return;
    };
    
    let Ok(drop_zone_transform) = drop_zone_query.single() else {
        return;
    };
    
    // Check if robot is near the drop zone
    let distance = robot_transform.translation.distance(drop_zone_transform.translation);
    
    if distance < 1.0 {
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

fn move_robot(
    mut commands: Commands,
    mut robot_query: Query<(&mut Robot, &mut Transform)>,
    time: Res<Time>,
    mut ball_query: Query<(Entity, &Ball, &mut Transform), (Without<Robot>, Without<PickedUpBall>)>,
) {
    for (mut robot, mut transform) in robot_query.iter_mut() {
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
                                println!("Picked up ball from ({}, {})", robot.current_i, robot.current_j);
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
            
            // Rotate to face movement direction
            if direction.length_squared() > 0.001 {
                let target_rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
                transform.rotation = transform.rotation.slerp(target_rotation, time.delta_secs() * 10.0);
            }
        }
    }
}

fn setup_ui(mut commands: Commands) {
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
