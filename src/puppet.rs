use avian3d::prelude::{
    Collider, GravityScale, RigidBody, ShapeCastConfig, SpatialQuery, SpatialQueryFilter,
};
use bevy::{
    log,
    math::{VectorSpace, ops::log10},
    prelude::*,
};

use crate::{MAX_BOUNCES, PuppeteerSet, puppeteer::GravityMultiplier};

pub struct PuppetPlugin;
impl Plugin for PuppetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Puppet>().register_type::<Grounded>();
        app.add_systems(
            FixedPostUpdate,
            (
                check_if_grounded.in_set(PuppeteerSet::Prepare),
                move_puppet.in_set(PuppeteerSet::Move),
            ),
        );
    }
}

/// A puppet can interact with physics objects and simulate movement
///
/// The puppet can collide with objects and slide along surfaces while maintaining
/// physical constraints like step height and maximum slope angle.
///
#[derive(Reflect, Clone, Copy, Component, Debug, PartialEq)]
#[reflect(Debug, Component, Default, PartialEq)]
#[require(
    Collider::capsule(0.25, 1.20),
    RigidBody::Kinematic,
    Transform,
    GravityScale,
    GravityMultiplier
)]
pub struct Puppet {
    /// The amount of extra distance added to collision checks
    /// to prevent tunneling in collision detection.
    /// Usually a small value like 0.01 is enough.
    pub skin_thickness: f32,

    /// The distance moved when stepping up a step. When this distance is too small and your collider base
    /// isn't flat then the collider will land on the edge of the step and slide off it
    pub step_move_distance: f32,

    /// The height of a step that the puppet can step up
    pub step_height: f32,

    /// The maximum angle of a slope in degrees that the puppet can walk up / stand on
    pub max_slope_angle: f32,

    /// The relative position the puppet tries to move to in the next iteration.
    /// This is reset to zero when the puppet has moved.
    /// Use [Puppet::move_to] to update the target position.
    pub target_position: Vec3,
}

impl Puppet {
    /// Try to move the puppet to its relative target position.
    /// Applies gravity, collision detection and surface sliding.
    /// This method should be called in [`FixedUpdate`].
    pub fn move_to(&mut self, vec: Vec3) {
        self.target_position = vec;
    }
}
impl Default for Puppet {
    fn default() -> Self {
        Self {
            skin_thickness: 0.025,
            step_move_distance: 0.2,
            step_height: 0.5,
            max_slope_angle: 55.0,
            target_position: Vec3::ZERO,
        }
    }
}

/// Marker component for a puppet that is currently grounded.
#[derive(Clone, Debug, Default, PartialEq, Copy, Component, Reflect)]
#[reflect(Debug, Component, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub struct Grounded;

pub(crate) fn check_if_grounded(
    mut commands: Commands,
    mut controller_query: Query<(&Puppet, &mut Transform, &Collider, Entity)>,
    spatial_query: SpatialQuery,
) {
    for (controller, mut transform, collider, entity) in controller_query.iter_mut() {
        if let Some(hit) = spatial_query.cast_shape(
            collider,
            transform.translation,
            Quat::default(),
            Dir3::NEG_Y,
            &ShapeCastConfig::from_max_distance(controller.skin_thickness * 2.0),
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
        ) {
            if hit.distance == 0.0 {
                transform.translation.y += controller.skin_thickness;
            }
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

#[allow(clippy::complexity)]
pub fn move_puppet(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Puppet,
        Has<Grounded>,
        &Collider,
        &mut Transform,
    )>,
    spatial_query: SpatialQuery,
) {
    for (entity, puppet, grounded, collider, mut transform) in query.iter_mut() {
        let gravity = Vec3::new(0.0, puppet.target_position.y, 0.0);

        let mut effective_translation = collide_and_slide(
            transform.translation,
            puppet.target_position * Vec3::new(1.0, 0.0, 1.0) * time.delta_secs(),
            &spatial_query,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
            collider,
            &puppet,
            grounded,
            0,
            false,
        );
        effective_translation += collide_and_slide(
            transform.translation + effective_translation,
            gravity * time.delta_secs(),
            &spatial_query,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
            collider,
            &puppet,
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
    puppet: &Puppet,
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

    let mut initial_vel = puppet.target_position;
    if gravity_pass {
        initial_vel = Vec3::new(0.0, puppet.target_position.y, 0.0);
    }

    if let Some(hit) = spatial_query.cast_shape(
        collider,
        pos,
        Quat::default(),
        Dir3::new(vel.normalize()).unwrap(),
        &ShapeCastConfig::from_max_distance(vel.length() + puppet.skin_thickness),
        query_filter,
    ) {
        let mut effective_vel = vel.normalize_or_zero() * (hit.distance - puppet.skin_thickness);
        let mut remaining_vel = vel - effective_vel;
        let angle = Vec3::Y.angle_between(hit.normal1).to_degrees();

        if effective_vel.length() <= puppet.skin_thickness {
            effective_vel = Vec3::ZERO;
        }

        // Check for max slope
        if angle <= puppet.max_slope_angle {
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
                let mut step_height = puppet.step_height;
                let mut step_vel = vel + (-hit.normal1 * puppet.step_move_distance);

                // 1. Cast collision shape up a step-height
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos,
                    Quat::default(),
                    Dir3::Y,
                    &ShapeCastConfig::from_max_distance(step_height + puppet.skin_thickness),
                    query_filter,
                ) {
                    step_height = step_hit.distance - puppet.skin_thickness;
                }
                // 2. Cast collision shape along velocity direction
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos + (Vec3::Y * step_height),
                    Quat::default(),
                    Dir3::new(step_vel.normalize_or_zero()).unwrap(),
                    &ShapeCastConfig::from_max_distance(step_vel.length() + puppet.skin_thickness),
                    query_filter,
                ) {
                    step_vel =
                        vel.normalize_or_zero() * (step_hit.distance - puppet.skin_thickness);
                }
                if step_vel.length() <= puppet.skin_thickness {
                    step_vel = Vec3::ZERO;
                }
                // 3. Cast collision shape down new vel.y - pos.y
                if let Some(step_hit) = spatial_query.cast_shape(
                    collider,
                    pos + step_vel + (Vec3::Y * step_height),
                    Quat::default(),
                    Dir3::NEG_Y,
                    &ShapeCastConfig::from_max_distance(step_height + puppet.skin_thickness),
                    query_filter,
                ) {
                    step_height -= step_hit.distance - puppet.skin_thickness;
                    let step_angle = Vec3::Y.angle_between(step_hit.normal1).to_degrees();
                    if step_angle <= puppet.max_slope_angle {
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
                puppet,
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
