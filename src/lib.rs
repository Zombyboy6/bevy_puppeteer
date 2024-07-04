pub mod puppeteer;

use bevy::prelude::*;
use bevy_xpbd_3d::plugins::{
    collision::Collider,
    spatial_query::{self, SpatialQuery, SpatialQueryFilter},
};

const MAX_BOUNCES: u32 = 5;

pub struct ZCharacterControllerPlugin;

impl Plugin for ZCharacterControllerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Update,
            (
                check_if_grounded,
                puppeteer::movement,
                puppeteer::scale_gravity,
                puppeteer::update_coyote_time,
                puppeteer::update_jump_buffer,
                puppeteer::jumping,
                move_controller,
            )
                .chain(),
        );
    }
}

#[derive(Debug, Component, Default)]
pub struct PuppetInput {
    movement_vec: Vec3,
    pub gravity: f32,
}

impl PuppetInput {
    pub fn move_to(&mut self, vec: Vec3) {
        self.movement_vec = vec;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Copy, Component)]
pub struct Grounded;

#[derive(Debug, Component)]
pub struct KinematicPuppet {
    pub skin_thickness: f32,
    pub step_move_distance: f32,
    pub step_height: f32,
    pub max_slope_angle: f32,
}

impl Default for KinematicPuppet {
    fn default() -> Self {
        Self {
            skin_thickness: 0.025,
            step_move_distance: 0.2,
            step_height: 0.5,
            max_slope_angle: 55.0,
        }
    }
}

pub(crate) fn check_if_grounded(
    mut commands: Commands,
    mut controller_query: Query<(&KinematicPuppet, &mut Transform, &Collider, Entity)>,
    spatial_query: SpatialQuery,
) {
    for (controller, mut transform, collider, entity) in controller_query.iter_mut() {
        if let Some(hit) = spatial_query.cast_shape(
            collider,
            transform.translation,
            Quat::default(),
            Direction3d::NEG_Y,
            controller.skin_thickness * 2.0,
            true,
            SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            if hit.time_of_impact == 0.0 {
                transform.translation.y += controller.skin_thickness;
            }
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

pub fn move_controller(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &KinematicPuppet,
        &PuppetInput,
        Has<Grounded>,
        &Collider,
        &mut Transform,
    )>,
    spatial_query: SpatialQuery,
) {
    for (entity, controller, input, grounded, collider, mut transform) in query.iter_mut() {
        let gravity = Vec3::new(0.0, input.gravity, 0.0) * time.delta_seconds();
        println!("{:?}", input.gravity);

        let mut effective_translation = collide_and_slide(
            transform.translation,
            input.movement_vec * Vec3::new(1.0, 0.0, 1.0) * time.delta_seconds(),
            &spatial_query,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
            collider,
            controller,
            &input,
            grounded,
            0,
            false,
        );
        effective_translation += collide_and_slide(
            transform.translation + effective_translation,
            gravity,
            &spatial_query,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
            collider,
            controller,
            &input,
            grounded,
            0,
            true,
        );

        transform.translation += effective_translation;
    }
}

#[allow(clippy::complexity)]
fn collide_and_slide(
    pos: Vec3,
    vel: Vec3,
    spatial_query: &SpatialQuery,
    query_filter: &SpatialQueryFilter,
    collider: &Collider,
    controller: &KinematicPuppet,
    initial_input: &PuppetInput,
    grounded: bool,
    depth: u32,
    gravity_pass: bool,
) -> Vec3 {
    if vel.length() == 0.0 {
        return Vec3::ZERO;
    }
    if depth >= MAX_BOUNCES {
        return Vec3::ZERO;
    }

    let mut initial_vel = initial_input.movement_vec;
    if gravity_pass {
        initial_vel = Vec3::new(0.0, initial_input.gravity, 0.0);
    }

    if let Some(hit) = spatial_query.cast_shape(
        collider,
        pos,
        Quat::default(),
        Direction3d::new(vel.normalize_or_zero()).unwrap(),
        vel.length() + controller.skin_thickness,
        true,
        query_filter.to_owned(),
    ) {
        let mut effective_vel =
            vel.normalize_or_zero() * (hit.time_of_impact - controller.skin_thickness);
        let mut remaining_vel = vel - effective_vel;
        let angle = Vec3::Y.angle_between(hit.normal1).to_degrees();

        if effective_vel.length() <= controller.skin_thickness {
            effective_vel = Vec3::ZERO;
        }

        // Check for max slope
        if angle <= controller.max_slope_angle {
            if gravity_pass {
                return effective_vel;
            }
            remaining_vel = project_and_scale(remaining_vel, hit.normal1);
        } else {
            // Hit wall
            // Scale slide distance by angle of collision
            let scale = 1.0
                - Vec3::dot(
                    Vec3::new(hit.normal1.x, 0.0, hit.normal1.z).normalize_or_zero(),
                    -Vec3::new(initial_vel.x, 0.0, initial_vel.z).normalize_or_zero(),
                );

            if grounded && !gravity_pass {
                //Check step
                let mut step_height = controller.step_height;
                let mut step_vel = vel + (-hit.normal1 * controller.step_move_distance);

                // 1. Cast collision shape up a step-height
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos,
                    Quat::default(),
                    Direction3d::Y,
                    step_height + controller.skin_thickness,
                    true,
                    query_filter.to_owned(),
                ) {
                    step_height = step_hit.time_of_impact - controller.skin_thickness;
                }
                // 2. Cast collision shape along velocity direction
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos + (Vec3::Y * step_height),
                    Quat::default(),
                    Direction3d::new(step_vel.normalize_or_zero()).unwrap(),
                    step_vel.length() + controller.skin_thickness,
                    true,
                    query_filter.to_owned(),
                ) {
                    step_vel = vel.normalize_or_zero()
                        * (step_hit.time_of_impact - controller.skin_thickness);
                }
                if step_vel.length() <= controller.skin_thickness {
                    step_vel = Vec3::ZERO;
                }
                // 3. Cast collision shape down new vel.y - pos.y
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos + step_vel + (Vec3::Y * step_height),
                    Quat::default(),
                    Direction3d::NEG_Y,
                    step_height + controller.skin_thickness,
                    true,
                    query_filter.to_owned(),
                ) {
                    step_height -= step_hit.time_of_impact - controller.skin_thickness;
                    let step_angle = Vec3::Y.angle_between(step_hit.normal1).to_degrees();
                    if step_angle <= controller.max_slope_angle {
                        return Vec3::new(step_vel.x, 0.0, step_vel.z) + (Vec3::Y * step_height);
                    }
                }
                // Treat the collision normal as a flat wall to fix jitter when sliding along steep
                // angles
                remaining_vel = project_and_scale(
                    Vec3::new(remaining_vel.x, 0.0, remaining_vel.z),
                    Vec3::new(hit.normal1.x, 0.0, hit.normal1.z),
                ) * scale;
            } else {
                remaining_vel = project_and_scale(remaining_vel, hit.normal1) * scale;
            }
        }

        effective_vel
            + collide_and_slide(
                pos + effective_vel,
                remaining_vel,
                spatial_query,
                query_filter,
                collider,
                controller,
                initial_input,
                grounded,
                depth + 1,
                gravity_pass,
            )
    } else {
        vel
    }
}

fn project_onto_plane(rhs: Vec3, plane: Vec3) -> Vec3 {
    let sqr_mag = plane.dot(plane);
    if sqr_mag < f32::EPSILON {
        rhs
    } else {
        let dot = rhs.dot(plane);
        Vec3::new(
            rhs.x - plane.x * dot / sqr_mag,
            rhs.y - plane.y * dot / sqr_mag,
            rhs.z - plane.z * dot / sqr_mag,
        )
    }
}

fn project_and_scale(rhs: Vec3, plane: Vec3) -> Vec3 {
    project_onto_plane(rhs, plane).normalize_or_zero() * rhs.length()
}
