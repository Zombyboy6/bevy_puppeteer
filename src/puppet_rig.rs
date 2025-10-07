use std::time::Duration;

use bevy::{math::ops::sin, prelude::*, time::Stopwatch};

use crate::{puppet::Grounded, puppeteer::Puppeteer};

#[derive(Clone, Copy, Component, Debug, PartialEq, Reflect)]
#[relationship(relationship_target = PuppetRigs)]
pub struct RelatedPuppet(Entity);

#[derive(Clone, Component, Debug, PartialEq, Reflect)]
#[reflect(Debug, Component, Default, PartialEq)]
#[require(LastPosition)]
pub struct PuppetRig {
    /// Offset relative to the puppet's position.
    /// If None, the rig will calculate the offset based on the initial transform.
    pub offset: Option<Vec3>,
    pub smoothing: f32,

    pub fov_acceleration_multiplier: f32,
    pub fov_acceleration_smoothing: f32,
    pub fov: f32,

    pub yaw: f32,
    pub pitch: f32,

    pub timer: Stopwatch,
    pub bobbing_offset: Vec3,
    pub vertical_bobbing_amplitude: f32,
    pub horizontal_bobbing_amplitude: f32,
    pub bobbing_frequency: f32,
}

impl Default for PuppetRig {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            yaw: Default::default(),
            pitch: Default::default(),
            smoothing: 50.0,
            fov_acceleration_multiplier: 7.0,
            fov: 60.0_f32.to_radians(),
            fov_acceleration_smoothing: 10.0,
            timer: Stopwatch::new(),
            bobbing_offset: Vec3::ZERO,
            vertical_bobbing_amplitude: 0.05,
            horizontal_bobbing_amplitude: 0.05,
            bobbing_frequency: 1.0,
        }
    }
}

#[derive(Clone, Component, Debug, PartialEq, Reflect, Default)]
#[reflect(Debug, Component, Default, PartialEq)]
#[relationship_target(relationship = RelatedPuppet)]
pub struct PuppetRigs(Vec<Entity>);

#[derive(Clone, Copy, Component, Debug, PartialEq, Reflect, Default, Deref, DerefMut)]
#[reflect(Debug, Component, Default, PartialEq)]
pub struct LastPosition(pub Vec3);

pub(crate) fn bobbing(
    mut rig_query: Query<(&mut PuppetRig, &RelatedPuppet, &Transform)>,
    puppeteer_query: Query<(&LastPosition, &Transform), With<Grounded>>,
) {
    for (mut rig, related_puppet, transform) in rig_query.iter_mut() {
        if let Ok((last_position, puppet_transform)) = puppeteer_query.get(related_puppet.0) {
            let velocity = puppet_transform.translation - last_position.0;

            let vel_scaled = velocity.length();
            if vel_scaled == 0.0 {
                rig.timer.reset();
                continue;
            }

            rig.timer.tick(Duration::from_secs_f32(vel_scaled));
        }

        let right = transform.right().to_owned();
        let up = transform.up().to_owned();
        let bobbing = sin(rig.timer.elapsed_secs() * 0.5 * rig.bobbing_frequency);
        let bobbing_up = sin(rig.timer.elapsed_secs() * rig.bobbing_frequency);

        let y_amp = rig.vertical_bobbing_amplitude;
        let x_amp = rig.horizontal_bobbing_amplitude;

        rig.bobbing_offset = up * bobbing_up * y_amp + right * bobbing * x_amp;
    }
}

pub(crate) fn apply_bobbing_offset(
    mut rig_query: Query<(&mut PuppetRig, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut rig, mut transform) in rig_query.iter_mut() {
        transform.translation += rig.bobbing_offset;
        if rig.timer.elapsed_secs() == 0.0 {
            let smooth_offset_reset =
                ((Vec3::ZERO) - rig.bobbing_offset) * (1.0 - (-10.0 * time.delta_secs()).exp());

            rig.bobbing_offset += smooth_offset_reset;
        }
    }
}

pub(crate) fn fov(
    mut rig_query: Query<(
        &PuppetRig,
        &RelatedPuppet,
        &LastPosition,
        &Transform,
        &mut Projection,
    )>,
    puppeteer_query: Query<&Puppeteer>,
    time: Res<Time>,
) {
    for (rig, related_puppet, last_position, transform, mut projection) in rig_query.iter_mut() {
        let velocity = transform.translation - last_position.0;

        let dot = velocity.normalize_or_zero().dot(*transform.forward());
        let puppeteer = puppeteer_query.get(related_puppet.0).unwrap();

        let vel_percentage = velocity.length() / puppeteer.max_speed / time.delta_secs();

        if let Projection::Perspective(perspective) = &mut *projection {
            let smooth_fov = ((rig.fov
                + (vel_percentage * rig.fov_acceleration_multiplier * dot.max(0.0)).to_radians())
                - perspective.fov)
                * (1.0 - (-rig.fov_acceleration_smoothing * time.delta_secs()).exp());

            perspective.fov += smooth_fov;
        }
    }
}
pub(crate) fn update_last_position(mut rig_query: Query<(&mut LastPosition, &Transform)>) {
    for (mut last_position, transform) in rig_query.iter_mut() {
        last_position.0 = transform.translation;
    }
}

pub(crate) fn sync_rig(
    mut rig_query: Query<(Entity, &mut PuppetRig, &RelatedPuppet)>,
    mut transform_query: Query<&mut Transform>,
    time: Res<Time>,
) {
    for (rig_entity, mut rig, related_puppet) in rig_query.iter_mut() {
        let rig_translation = transform_query.get(rig_entity).unwrap().translation;
        let Some(offset) = rig.offset else {
            rig.offset = Some(rig_translation);
            continue;
        };

        let puppet_transform = transform_query.get(related_puppet.0).unwrap().translation;

        let smooth_pos = (puppet_transform + offset - rig_translation)
            * (1.0 - (-rig.smoothing * time.delta_secs()).exp());

        let mut rig_transform = transform_query.get_mut(rig_entity).unwrap();
        rig_transform.translation += smooth_pos;

        let new_rotation_y = Quat::from_axis_angle(Vec3::Y, rig.yaw);
        let new_rotation_x = Quat::from_axis_angle(Vec3::X, rig.pitch);
        rig_transform.rotation = new_rotation_y * new_rotation_x;
    }
}
