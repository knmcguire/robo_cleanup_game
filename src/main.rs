use bevy::prelude::*;

const BOARD_SIZE_I: usize = 5;
const BOARD_SIZE_J: usize = 5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, setup_board).chain())
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

    // Spawn the game board
    for j in 0..BOARD_SIZE_J {
        for i in 0..BOARD_SIZE_I {
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(tile_material.clone()),
                Transform::from_xyz(i as f32, -0.2, j as f32),
            ));
        }
    }
}
