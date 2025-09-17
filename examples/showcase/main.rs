mod map;

use std::any::Any;

use avian3d::{
    PhysicsPlugins,
    prelude::{Collider, RigidBody},
};
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput, mouse::MouseMotion},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use bevy_inspector_egui::{
    DefaultInspectorConfigPlugin,
    bevy_egui::{EguiContext, EguiPlugin},
    egui,
    restricted_world_view::RestrictedWorldView,
};
use puppeteer::{
    PuppeteerPlugin,
    puppet_rig::{PuppetRig, PuppetRigs},
    puppeteer::{Puppeteer, PuppeteerInput},
};

use crate::map::{move_platform, rotate, spawn_map};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PuppeteerPlugin)
        .add_plugins((
            PhysicsPlugins::default(), /* PhysicsDebugPlugin::default()*/
            DefaultInspectorConfigPlugin,
            EguiPlugin::default(),
            //WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_map)
        .add_systems(Update, (player_look, ui, mouse_lock))
        .add_systems(Update, (rotate, move_platform))
        .add_systems(FixedUpdate, player_move)
        .run();
}

#[derive(Component, Default)]
pub struct Player;

fn setup(
    mut commands: Commands,
    mut windows_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) -> Result {
    windows_query.single_mut()?.grab_mode = CursorGrabMode::Locked;
    windows_query.single_mut()?.visible = false;

    // player
    let _player = commands.spawn((
        Player,
        Puppeteer::default(),
        Collider::capsule(0.25, 1.80),
        RigidBody::Kinematic,
        Transform::from_xyz(0.0, 5.5, 0.0),
        related!(
            PuppetRigs[(
                PuppetRig {
                    offset: Some(Vec3::new(0.0, 0.9, 0.0)),
                    ..default()
                },
                Camera3d::default(),
            )]
        ),
    ));

    Ok(())
}

fn ui(world: &mut World) {
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .single(world)
    else {
        return;
    };

    let mut egui_context = egui_context.clone();

    egui::Window::new("Settings").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let Ok((entity, puppeteer, rigs)) = &world
                .query::<(Entity, &Puppeteer, &PuppetRigs)>()
                .single(world)
            else {
                return;
            };

            let type_id = puppeteer.to_owned().type_id();
            let Some(rig_entity) = rigs.collection().first().cloned() else {
                return;
            };
            let Ok(rig) = &world.query::<&PuppetRig>().get(world, rig_entity) else {
                return;
            };

            let type_id_rig = rig.to_owned().type_id();

            let type_registry = world.resource::<AppTypeRegistry>().0.clone();
            let type_registry = type_registry.read();
            let mut world = RestrictedWorldView::from(world);
            let (mut component_world, world) = world.split_off_component((*entity, type_id));

            let mut puppeteer = component_world
                .get_entity_component_reflect(*entity, type_id, &type_registry)
                .unwrap();

            egui::CollapsingHeader::new("Puppeteer").show(ui, |ui| unsafe {
                bevy_inspector_egui::bevy_inspector::ui_for_value(
                    puppeteer.downcast_mut::<Puppeteer>().unwrap(),
                    ui,
                    world.world().world_mut(),
                );
            });

            let mut world = RestrictedWorldView::from(world);
            let (mut component_world, world) = world.split_off_component((rig_entity, type_id_rig));

            let mut rig = component_world
                .get_entity_component_reflect(rig_entity, type_id_rig, &type_registry)
                .unwrap();

            egui::CollapsingHeader::new("Puppet rig").show(ui, |ui| unsafe {
                bevy_inspector_egui::bevy_inspector::ui_for_value(
                    rig.downcast_mut::<PuppetRig>().unwrap(),
                    ui,
                    world.world().world_mut(),
                );
            });
        });
    });
}

fn mouse_lock(
    mut query: Query<&mut CursorOptions, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    let Ok(mut cursor_options) = query.single_mut() else {
        return;
    };

    if cursor_options.grab_mode != CursorGrabMode::Locked {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
    } else {
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
    }
}
pub fn player_look(
    mut player_head_query: Query<&mut PuppetRig, Without<Player>>,
    mut mouse_motion_event: MessageReader<MouseMotion>,
    window: Single<&CursorOptions, With<PrimaryWindow>>,
) -> Result {
    let sensibility = 0.75;
    for mut head in player_head_query.iter_mut() {
        for mouse in mouse_motion_event.read() {
            if window.grab_mode == CursorGrabMode::None {
                continue;
            }
            head.pitch -= (0.1 * mouse.delta.y * sensibility).to_radians();
            head.yaw -= (0.1 * mouse.delta.x * sensibility).to_radians();

            head.pitch = head.pitch.clamp(-1.54, 1.54);
        }
    }
    Ok(())
}

pub fn player_move(
    player_head_query: Query<&PuppetRig>,
    mut player_query: Query<(&mut PuppeteerInput, &mut Puppeteer)>,
    mut keyboard_input: Local<ButtonInput<KeyCode>>,
    mut keyboard_input_events: MessageReader<KeyboardInput>,
) -> Result {
    keyboard_input.clear();
    for event in keyboard_input_events.read() {
        let key_code = event.key_code;
        match event.state {
            ButtonState::Pressed => keyboard_input.press(key_code),
            ButtonState::Released => keyboard_input.release(key_code),
        }
    }

    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vec3::new(horizontal as f32, 0.0, vertical as f32).clamp_length_max(1.0);

    let head = player_head_query.single()?;
    let (mut input, _puppeteer) = player_query.single_mut()?;

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
    Ok(())
}
