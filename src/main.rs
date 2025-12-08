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
    waypoint_queue: Vec<(i32, i32)>,
}

#[derive(Component)]
struct WaypointList;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, (setup_camera, setup_board, setup_ui).chain())
        .add_systems(Update, (update_tile_highlights, move_robot, update_waypoint_ui))
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
    commands.spawn((
        Robot {
            current_i: 0,
            current_j: 0,
            path: Vec::new(),
            move_speed: 2.0,
            waypoint_queue: Vec::new(),
        },
        SceneRoot(asset_server.load("robot.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0)
            .with_scale(Vec3::splat(0.2)),
    ));

    // Create mesh and material for tiles
    let tile_mesh = meshes.add(Cuboid::new(1.0, 0.2, 1.0));
    let tile_material = materials.add(Color::srgb(0.3, 0.7, 0.4));
    let highlight_material = materials.add(Color::srgb(1.0, 1.0, 0.0));

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
}

/// Returns an observer that handles tile clicks
fn on_tile_click(
    highlight_material: Handle<StandardMaterial>,
    original_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Tile, &mut MeshMaterial3d<StandardMaterial>)>, Query<&mut Robot>, Commands) {
    move |event, mut tile_query, mut robot_query, mut commands| {
        if let Ok((tile, mut material)) = tile_query.get_mut(event.event_target()) {
            println!("Tile clicked at position: ({}, {})", tile.i, tile.j);
            
            // Change to highlight material
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(TileHighlight {
                timer: Timer::from_seconds(0.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
            
            // Add waypoint to robot's queue
            if let Ok(mut robot) = robot_query.single_mut() {
                robot.waypoint_queue.push((tile.i, tile.j));
                println!("Added waypoint to queue: ({}, {}). Queue size: {}", tile.i, tile.j, robot.waypoint_queue.len());
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

fn move_robot(
    mut robot_query: Query<(&mut Robot, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut robot, mut transform) in robot_query.iter_mut() {
        // If no current path but waypoints in queue, calculate path to next waypoint
        if robot.path.is_empty() && !robot.waypoint_queue.is_empty() {
            let start = (robot.current_i, robot.current_j);
            let goal = robot.waypoint_queue[0];
            
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
                let reached_waypoint = robot.waypoint_queue[0];
                if robot.current_i == reached_waypoint.0 && robot.current_j == reached_waypoint.1 {
                    println!("Robot reached waypoint: ({}, {})", robot.current_i, robot.current_j);
                    robot.waypoint_queue.remove(0);
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
        Text::new("Waypoints:"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            right: Val::Px(5.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
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
    robot_query: Query<&Robot>,
    mut waypoint_text_query: Query<&mut Text, With<WaypointList>>,
) {
    let Ok(robot) = robot_query.single() else {
        return;
    };
    
    let Ok(mut text) = waypoint_text_query.single_mut() else {
        return;
    };
    
    // Update text to show all waypoints
    if robot.waypoint_queue.is_empty() {
        text.0 = "Waypoints: (none)".to_string();
    } else {
        let waypoint_list: Vec<String> = robot.waypoint_queue
            .iter()
            .map(|(i, j)| format!("({}, {})", i, j))
            .collect();
        text.0 = format!("Waypoints:\n{}", waypoint_list.join("\n"));
    }
}
