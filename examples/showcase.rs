use avian3d::{
    prelude::{Collider, ColliderConstructorHierarchy, GravityScale, RigidBody},
    PhysicsPlugins,
};
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin},
    egui,
};
use puppeteer::{
    puppeteer::{Puppeteer, PuppeteerInput},
    PuppeteerPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PuppeteerPlugin)
        .add_plugins((
            PhysicsPlugins::default(), /* PhysicsDebugPlugin::default()*/
            EguiPlugin,
            bevy_inspector_egui::DefaultInspectorConfigPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (player_look, player_move, ui, mouse_lock))
        .run();
}

#[derive(Component, Default)]
pub struct Player;

#[derive(Component, Default)]
pub struct PlayerHead {
    pub height_offset: f32,
    pub yaw: f32,
    pub pitch: f32,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut windows_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    windows_query.single_mut().cursor_options.grab_mode = CursorGrabMode::Locked;
    windows_query.single_mut().cursor_options.visible = false;
    commands.spawn((
        SceneRoot(
            asset_server.load("test_level.gltf#Scene0"),
            //transform: Transform::from_rotation(Quat::from_rotation_y(-std::f32::consts::PI * 0.5)),
        ),
        ColliderConstructorHierarchy::new(Some(
            avian3d::prelude::ColliderConstructor::TrimeshFromMesh,
        )),
        RigidBody::Static,
    ));

    // player
    let _player = commands.spawn((
        Player,
        Puppeteer::default(),
        Collider::capsule(0.25, 1.80),
        RigidBody::Kinematic,
        Transform::from_xyz(0.0, 2.5, 0.0),
    ));

    // Player Head
    commands.spawn((
        PlayerHead {
            height_offset: 0.9,
            ..default()
        },
        Camera3d::default(),
    ));
}

fn ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single(world)
    else {
        return;
    };

    let mut egui_context = egui_context.clone();

    egui::Window::new("UI").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let Ok(puppeteer) = world
                .query_filtered::<Entity, With<Puppeteer>>()
                .get_single(world)
            else {
                return;
            };

            bevy_inspector_egui::bevy_inspector::ui_for_entity(world, puppeteer, ui);
        });
    });
}

fn mouse_lock(mut query: Query<&mut Window, With<PrimaryWindow>>, keys: Res<ButtonInput<KeyCode>>) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Ok(mut window) = query.get_single_mut() else {
        return;
    };

    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    } else {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}
pub fn player_look(
    mut player_head_query: Query<(&mut PlayerHead, &mut Transform), Without<Player>>,
    player_query: Query<&Transform, With<Player>>,
    mut mouse_motion_event: EventReader<MouseMotion>,
) {
    let sensibility = 0.75;
    for (mut head, mut head_transform) in player_head_query.iter_mut() {
        for mouse in mouse_motion_event.read() {
            head.pitch -= (0.1 * mouse.delta.y * sensibility).to_radians();
            head.yaw -= (0.1 * mouse.delta.x * sensibility).to_radians();

            head.pitch = head.pitch.clamp(-1.54, 1.54);

            let new_rotation_y = Quat::from_axis_angle(Vec3::Y, head.yaw);
            let new_rotation_x = Quat::from_axis_angle(Vec3::X, head.pitch);
            head_transform.rotation = new_rotation_y * new_rotation_x;
        }
        head_transform.translation =
            player_query.single().translation + (Vec3::Y * head.height_offset);
    }
}

pub fn player_move(
    player_head_query: Query<&PlayerHead>,
    mut player_query: Query<(&mut PuppeteerInput, &mut Puppeteer)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vec3::new(horizontal as f32, 0.0, vertical as f32).clamp_length_max(1.0);

    let head = player_head_query.single();
    let (mut input, _puppeteer) = player_query.single_mut();

    let local_z = Mat2::from_cols(
        [head.yaw.cos(), -head.yaw.sin()].into(),
        [head.yaw.sin(), head.yaw.cos()].into(),
    )
    .mul_vec2(Vec2::Y);
    let forward = -Vec3::new(local_z.x, 0., local_z.y);
    let right = Vec3::new(local_z.y, 0., -local_z.x);

    let mut move_vector = Vec3::ZERO;
    move_vector += forward * direction.z;
    move_vector += right * direction.x;
    move_vector = move_vector.normalize_or_zero();

    if keyboard_input.just_pressed(KeyCode::Space) {
        input.start_jump();
    }
    if keyboard_input.just_released(KeyCode::Space) {
        input.stop_jump();
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        input.speed_multiplier = 2.0
    } else {
        input.speed_multiplier = 1.0
    }

    input.move_amount(move_vector);
    //println!("{:?}", move_vector);
}
