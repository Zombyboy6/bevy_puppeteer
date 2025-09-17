use std::f32::consts::PI;

use avian3d::prelude::{AngularDamping, Collider, Friction, LinearDamping, Mass, RigidBody};
use bevy::{
    light::{CascadeShadowConfigBuilder, NotShadowCaster},
    math::primitives::Sphere,
    prelude::*,
};

pub fn spawn_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 100.0,
        ..default()
    }
    .build();
    // Sun
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
        cascade_shadow_config,
    ));
    // Sky
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Srgba::hex("87ceeb").unwrap().into(),
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(200.0)),
        NotShadowCaster,
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 0.5, -5.0),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(asset_server.load("tile.png"))),
        Collider::sphere(0.5),
        Mass(1.0),
        Friction::new(0.4),
        LinearDamping(1.5),
        AngularDamping(1.5),
        RigidBody::Dynamic,
    ));
    commands.spawn((
        Transform::from_xyz(0.0, 1.5, -5.0),
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(asset_server.load("tile.png"))),
        Collider::cuboid(4.0, 1.0, 1.0),
        Mass(1.0),
        Friction::new(0.4),
        LinearDamping(1.5),
        AngularDamping(1.5),
        RigidBody::Dynamic,
    ));

    let mut cube = |transform: Transform,
                    size: Vec3|
     -> (
        Mesh3d,
        MeshMaterial3d<StandardMaterial>,
        Transform,
        Collider,
        RigidBody,
    ) {
        (
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(materials.add(asset_server.load("tile.png"))),
            transform,
            Collider::cuboid(size.x, size.y, size.z),
            RigidBody::Static,
        )
    };

    // floor
    commands.spawn(cube(
        Transform::from_xyz(0.0, -0.05, -25.0),
        Vec3::new(15.0, 0.1, 60.0),
    ));

    // slopes
    for x in 0..14 {
        commands.spawn(cube(
            Transform::from_xyz(9.0, -1.5 + x as f32 * 0.15, -6.5 + -x as f32 * 3.0).with_rotation(
                Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, (x as f32 * 5.0).to_radians()),
            ),
            Vec3::new(3.0, 3.0, 3.0),
        ));
    }
    // steps
    for x in 0..20 {
        commands.spawn(cube(
            Transform::from_xyz(
                x as f32 * 1.0 + 8.0,
                (x as f32 * 0.05) * x as f32 / 2.0,
                0.0,
            ),
            Vec3::new(1.0, (x as f32 * 0.05) * x as f32, 10.0),
        ));
    }

    // obstacles
    for _ in 0..100 {
        let origin = Vec3::new(0.0, 0.0, -15.0);
        let x = rand::random_range(-5.0..5.0);
        let z = rand::random_range(-5.0..5.0);

        commands.spawn(cube(
            Transform::from_xyz(x + origin.x, origin.y, z + origin.z).with_rotation(
                Quat::from_euler(
                    EulerRot::XYZ,
                    rand::random_range(0.0..PI),
                    rand::random_range(0.0..PI),
                    rand::random_range(0.0..PI),
                ),
            ),
            Vec3::new(0.5, 0.5, 0.5),
        ));
    }
    // spinning platform
    commands.spawn((
        cube(
            Transform::from_xyz(-5.0, 2.0, -40.0),
            Vec3::new(2.0, 1.0, 13.0),
        ),
        Rotate(0.5),
    ));

    commands.spawn((
        cube(
            Transform::from_xyz(-5.0, 2.0, -40.0),
            Vec3::new(2.0, 1.0, 13.0),
        ),
        Move(Vec3::new(0.4, 0.0, 0.0)),
    ));

    let mut cylinder = |transform: Transform,
                        size: Vec3|
     -> (
        Mesh3d,
        MeshMaterial3d<StandardMaterial>,
        Transform,
        Collider,
        RigidBody,
    ) {
        (
            Mesh3d(meshes.add(Cylinder::new(size.x, size.y))),
            MeshMaterial3d(materials.add(asset_server.load("tile.png"))),
            transform,
            Collider::cylinder(size.x, size.y),
            RigidBody::Static,
        )
    };

    commands.spawn(cylinder(
        Transform::from_xyz(-5.0, 0.5, -5.0),
        Vec3::new(2.0, 1.0, 1.0),
    ));
    commands.spawn((
        cylinder(
            Transform::from_xyz(0.0, 0.0, 10.0),
            Vec3::new(5.0, 1.0, 5.0),
        ),
        Rotate(0.1),
    ));
    commands.spawn((
        cylinder(
            Transform::from_xyz(-5.0, 1.0, -13.0),
            Vec3::new(2.0, 2.0, 1.0),
        ),
        Rotate(1.0),
    ));
    commands.spawn(cylinder(
        Transform::from_xyz(-5.0, 1.5, -20.0),
        Vec3::new(2.0, 3.0, 1.0),
    ));
    commands.spawn(cylinder(
        Transform::from_xyz(-5.0, 1.5, -30.0),
        Vec3::new(2.0, 3.0, 1.0),
    ));
}

#[derive(Component)]
pub struct Rotate(f32);

pub fn rotate(mut query: Query<(&mut Transform, &Rotate)>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform
            .0
            .rotate_local_y(transform.1.0 * time.delta_secs() * PI);
    }
}

#[derive(Component)]
pub struct Move(Vec3);

pub fn move_platform(mut query: Query<(&mut Transform, &Move)>, time: Res<Time>) {
    for (mut transform, move_component) in query.iter_mut() {
        transform.translation += move_component.0 * time.delta_secs();
    }
}
