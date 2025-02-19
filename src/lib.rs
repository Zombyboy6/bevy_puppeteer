#![allow(dead_code)]
pub mod puppeteer;

use avian3d::prelude::{
    Collider, GravityScale, Position, RigidBody, ShapeCastConfig, SpatialQuery, SpatialQueryFilter,
};
use bevy::prelude::*;

use puppeteer::{GravityMultiplier, Puppeteer, PuppeteerInput};

const MAX_BOUNCES: u32 = 5;

pub struct PuppeteerPlugin;

impl Plugin for PuppeteerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_type::<Puppeteer>()
            .register_type::<KinematicPuppet>()
            .register_type::<PuppeteerInput>();
        app.add_systems(
            FixedUpdate,
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

#[derive(Clone, Debug, Default, PartialEq, Copy, Component)]
pub struct Grounded;

#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
#[require(
    Collider,
    RigidBody,
    Transform,
    Visibility,
    GravityScale,
    GravityMultiplier
)]
pub struct KinematicPuppet {
    pub skin_thickness: f32,
    pub step_move_distance: f32,
    pub step_height: f32,
    pub max_slope_angle: f32,

    pub gravity: f32,
    movement_vel: Vec3,
}

impl KinematicPuppet {
    fn move_to(&mut self, target: Vec3) {
        self.movement_vel = target;
    }
}

impl Default for KinematicPuppet {
    fn default() -> Self {
        Self {
            skin_thickness: 0.025,
            step_move_distance: 0.2,
            step_height: 0.5,
            max_slope_angle: 55.0,
            gravity: 0.0,
            movement_vel: Vec3::default(),
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
pub fn move_controller(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &KinematicPuppet,
        Has<Grounded>,
        &Collider,
        &mut Transform,
    )>,
    spatial_query: SpatialQuery,
) {
    for (entity, puppet, grounded, collider, mut transform) in query.iter_mut() {
        let gravity = Vec3::new(0.0, puppet.gravity, 0.0) * time.delta_secs();

        let mut effective_translation = collide_and_slide(
            transform.translation,
            puppet.movement_vel * Vec3::new(1.0, 0.0, 1.0) * time.delta_secs(),
            &spatial_query,
            &SpatialQueryFilter::default().with_excluded_entities([entity]),
            collider,
            puppet,
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
            puppet,
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
    puppet: &KinematicPuppet,
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

    let mut initial_vel = puppet.movement_vel;
    if gravity_pass {
        initial_vel = Vec3::new(0.0, puppet.gravity, 0.0);
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

#[cfg(test)]
mod test {
    use avian3d::{
        prelude::{Collider, ColliderHierarchyPlugin, Position, RigidBody},
        PhysicsPlugins,
    };
    use bevy::prelude::*;

    use crate::{
        puppeteer::{Puppeteer, PuppeteerInput},
        PuppeteerPlugin,
    };

    #[test]
    fn delta_sync() {
        let mut app = build_app();

        test_env(app.world_mut());
        let puppet_id = app
            .world_mut()
            .spawn((
                Puppeteer::default(),
                Collider::capsule(0.25, 1.20),
                RigidBody::Kinematic,
                Transform::from_xyz(0.0, 4.0, 0.0),
            ))
            .id();
        app.update();

        for _ in 0..5000 {
            app.update();
        }

        let pos1 = app.world().get::<Position>(puppet_id).cloned();

        let mut app = build_app();

        app.insert_resource(Time::<Fixed>::from_hz(128.0));

        test_env(app.world_mut());
        let puppet_id = app
            .world_mut()
            .spawn((
                Puppeteer::default(),
                Collider::capsule(0.25, 1.20),
                RigidBody::Kinematic,
                Transform::from_xyz(0.0, 4.0, 0.0),
            ))
            .id();

        app.update();
        for _ in 0..5000 {
            app.update();
        }

        let pos2 = app.world().get::<Position>(puppet_id).cloned();

        assert_eq!(pos1, pos2);
    }

    fn test_env(world: &mut World) {
        world.spawn((Collider::cuboid(5.0, 0.1, 5.0), RigidBody::Static));
    }

    fn move_puppet(mut query: Query<&mut PuppeteerInput>) {
        let mut puppet = query.single_mut();
        puppet.move_amount(Vec3::X);
    }

    fn build_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::scene::ScenePlugin::default(),
        ))
        .add_plugins(PuppeteerPlugin)
        .add_plugins((PhysicsPlugins::default()
            .build()
            .disable::<ColliderHierarchyPlugin>(),))
        .add_systems(FixedUpdate, move_puppet)
        .init_resource::<Assets<Mesh>>();
        app.finish();
        app
    }
}
