use bevy::{prelude::*, picking::prelude::MeshPickingPlugin};

const BOARD_SIZE_I: usize = 5;
const BOARD_SIZE_J: usize = 5;

#[derive(Component)]
struct Tile {
    i: usize,
    j: usize,
}

#[derive(Component)]
struct TileHighlight {
    timer: Timer,
    original_material: Handle<StandardMaterial>,
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, (setup_camera, setup_board).chain())
        .add_systems(Update, update_tile_highlights)
        .run();
}

fn setup_camera(mut commands: Commands) {
    // Position camera for isometric view
    let focus_point = Vec3::new(
        BOARD_SIZE_I as f32 / 2.0,
        0.0,
        BOARD_SIZE_J as f32 / 2.0,
    );
    
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

    // Spawn the robot in the middle of the field
    let center_x = BOARD_SIZE_I as f32 / 2.0;
    let center_z = BOARD_SIZE_J as f32 / 2.0;
    
    commands.spawn((
        SceneRoot(asset_server.load("robot.glb#Scene0")),
        Transform::from_xyz(center_x, 0.0, center_z)
            .with_scale(Vec3::splat(0.2)),
    ));

    // Create mesh and material for tiles
    let tile_mesh = meshes.add(Cuboid::new(1.0, 0.2, 1.0));
    let tile_material = materials.add(Color::srgb(0.3, 0.7, 0.4));
    let highlight_material = materials.add(Color::srgb(1.0, 1.0, 0.0));

    // Spawn the game board
    for j in 0..BOARD_SIZE_J {
        for i in 0..BOARD_SIZE_I {
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(tile_material.clone()),
                Transform::from_xyz(i as f32, -0.2, j as f32),
                Tile { i, j },
            ))
            .observe(on_tile_click(highlight_material.clone(), tile_material.clone()));
        }
    }
}

/// Returns an observer that handles tile clicks
fn on_tile_click(
    highlight_material: Handle<StandardMaterial>,
    original_material: Handle<StandardMaterial>,
) -> impl Fn(On<Pointer<Click>>, Query<(&Tile, &mut MeshMaterial3d<StandardMaterial>)>, Commands) {
    move |event, mut query, mut commands| {
        if let Ok((tile, mut material)) = query.get_mut(event.event_target()) {
            println!("Tile clicked at position: ({}, {})", tile.i, tile.j);
            
            // Change to highlight material
            material.0 = highlight_material.clone();
            
            // Add highlight component with timer
            commands.entity(event.event_target()).insert(TileHighlight {
                timer: Timer::from_seconds(0.5, TimerMode::Once),
                original_material: original_material.clone(),
            });
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
