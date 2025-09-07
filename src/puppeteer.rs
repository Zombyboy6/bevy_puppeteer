use std::time::Duration;

use avian3d::prelude::GravityScale;
use bevy::prelude::*;

use crate::puppet::{Grounded, Puppet};

#[derive(Component, Reflect)]
#[require(Puppet, PuppeteerInput)]
pub struct Puppeteer {
    pub acceleration: f32,
    pub deceleration: f32,
    pub air_acceleration: f32,
    pub air_deceleration: f32,
    pub air_turn_speed: f32,
    pub max_speed: f32,
    pub turn_speed: f32,
    pub gravity: f32,

    pub jump_height: f32,
    pub time_to_jump_apex: f32,
    pub downward_movement_multiplier: f32,
    pub max_air_jumps: u32,
    pub jump_cutoff: f32,

    pub coyote_time: Duration,
    pub jump_buffer: Duration,
}

impl Default for Puppeteer {
    fn default() -> Self {
        Self {
            acceleration: 0.7,
            deceleration: 0.7,
            air_acceleration: 0.6,
            air_deceleration: 0.1,
            air_turn_speed: 0.1,
            max_speed: 7.0,
            turn_speed: 2.0,
            gravity: 9.81,
            jump_height: 1.0,
            time_to_jump_apex: 0.3,
            downward_movement_multiplier: 1.0,
            max_air_jumps: 0,
            jump_cutoff: 1.5,

            coyote_time: Duration::from_millis(150),
            jump_buffer: Duration::from_millis(150),
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
//#[require(Puppeteer)]
pub struct PuppeteerInput {
    pub move_direction: Vec3,
    pub speed_multiplier: f32,
    jump_start: bool,
    jump_canceled: bool,
}

impl PuppeteerInput {
    /// Move in direction (-1.0 to 1.0)
    pub fn move_amount(&mut self, direction: Vec3) {
        self.move_direction = direction;
    }

    /// Start jumping until chanceld (make sure to call ´stop_jump´)
    pub fn start_jump(&mut self) {
        self.jump_start = true;
    }

    /// Stop jumping
    pub fn stop_jump(&mut self) {
        self.jump_canceled = true;
    }
}

/// Component indicating that the entity is jumping with a timer defining the duration of the jump
#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(storage = "SparseSet")]
pub struct Jumping;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DominantCollider;

#[derive(Component, Deref, DerefMut, Reflect)]
#[component(storage = "SparseSet")]
pub struct AirJumpCount(pub u32);

#[derive(Component, Deref, DerefMut, Reflect)]
#[component(storage = "SparseSet")]
pub struct CoyoteTime(pub Timer);

#[derive(Component, Deref, DerefMut, Reflect)]
#[component(storage = "SparseSet")]
pub struct JumpBuffer(pub Timer);

#[derive(Component, Default, Deref, Reflect)]
pub struct GravityMultiplier(pub f32);

pub fn movement(
    mut query: Query<(
        &Puppeteer,
        &mut PuppeteerInput,
        &mut Puppet,
        Has<Grounded>,
        &GravityScale,
    )>,
    time: Res<Time>,
) {
    for (controller, mut move_action, mut puppet, is_grounded, gravity_scale) in &mut query {
        let acceleration = if is_grounded {
            controller.acceleration
        } else {
            controller.air_acceleration
        };
        let deceleration = if is_grounded {
            controller.deceleration
        } else {
            controller.air_deceleration
        };
        let turn_speed = if is_grounded {
            controller.turn_speed
        } else {
            controller.air_turn_speed
        };

        let desired_velocity =
            move_action.move_direction * controller.max_speed * move_action.speed_multiplier;

        let max_speed_change = if move_action.move_direction.length() > 0.1 {
            if puppet.target_position.length() < 0.1 {
                acceleration
            } else {
                acceleration.lerp(
                    turn_speed,
                    (-move_action
                        .move_direction
                        .normalize()
                        .dot(puppet.target_position.normalize())
                        + 1.0)
                        * 0.5,
                )
            }
        } else {
            deceleration
        };

        puppet.target_position =
            move_towards(puppet.target_position, desired_velocity, max_speed_change);

        // apply gravity
        if !is_grounded {
            puppet.gravity_velocity -= controller.gravity * **gravity_scale * time.delta_secs();
        }

        move_action.move_direction = Vec3::ZERO;
    }
}
fn move_towards(current: Vec3, target: Vec3, max_distance_delta: f32) -> Vec3 {
    if (target - current).xz().length() <= max_distance_delta {
        return target;
    }
    current + (target - current).normalize_or_zero() * max_distance_delta
}

pub fn scale_gravity(mut query: Query<(&Puppeteer, &GravityMultiplier, &mut GravityScale)>) {
    for (puppeteer, gravity_multiplier, mut gravity_scale) in &mut query {
        let new_gravity = (-2.0 * puppeteer.jump_height)
            / (puppeteer.time_to_jump_apex * puppeteer.time_to_jump_apex);

        **gravity_scale = (new_gravity / -puppeteer.gravity) * **gravity_multiplier;
    }
}

#[allow(clippy::complexity)]
pub fn jumping(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Puppeteer,
        &mut PuppeteerInput,
        &mut Puppet,
        Has<Grounded>,
        Has<Jumping>,
        &mut GravityMultiplier,
        &GravityScale,
        Option<&mut AirJumpCount>,
        Option<&mut CoyoteTime>,
        Has<JumpBuffer>,
    )>,
) {
    for (
        entity,
        puppeteer,
        mut input,
        mut puppet_input,
        is_grounded,
        is_jumping,
        mut gravity_multiplier,
        gravity_scale,
        air_jump_count,
        coyote_time,
        has_jump_buffer,
    ) in &mut query
    {
        if input.jump_canceled {
            commands.entity(entity).remove::<Jumping>();
            input.jump_canceled = false;
        }
        if input.jump_start {
            commands.entity(entity).insert(Jumping);

            if is_grounded || coyote_time.is_some_and(|t| !t.finished()) {
                commands.entity(entity).insert(JumpBuffer(Timer::new(
                    puppeteer.jump_buffer,
                    TimerMode::Once,
                )));
                commands.entity(entity).remove::<AirJumpCount>();
            } else if puppeteer.max_air_jumps > 0 {
                if let Some(mut jumps) = air_jump_count {
                    if **jumps >= puppeteer.max_air_jumps {
                        if !has_jump_buffer {
                            commands.entity(entity).insert(JumpBuffer(Timer::new(
                                puppeteer.jump_buffer,
                                TimerMode::Once,
                            )));
                        }
                        continue;
                    }
                    **jumps += 1;
                } else {
                    commands.entity(entity).insert(AirJumpCount(1));
                }
            } else {
                if !has_jump_buffer {
                    commands.entity(entity).insert(JumpBuffer(Timer::new(
                        puppeteer.jump_buffer,
                        TimerMode::Once,
                    )));
                }
                continue;
            }
            input.jump_start = false;

            let mut timer = Timer::new(puppeteer.jump_buffer, TimerMode::Once);
            timer.tick(puppeteer.coyote_time);
            commands.entity(entity).insert(CoyoteTime(timer));

            let mut jump_speed =
                (-2.0 * -puppeteer.gravity * **gravity_scale * puppeteer.jump_height).sqrt();

            if puppet_input.gravity_velocity > 0.0 {
                jump_speed = (jump_speed - puppet_input.gravity_velocity).max(0.0);
            } else if puppet_input.gravity_velocity < 0.0 {
                jump_speed += puppet_input.gravity_velocity.abs();
            }

            puppet_input.gravity_velocity += jump_speed;
        }

        if puppet_input.target_position.y > 0.01 {
            if is_jumping {
                gravity_multiplier.0 = 1.0;
            } else {
                gravity_multiplier.0 = puppeteer.jump_cutoff;
            }
        } else if puppet_input.gravity_velocity < -0.01 {
            gravity_multiplier.0 = puppeteer.downward_movement_multiplier;
        } else {
            gravity_multiplier.0 = 1.0;
        }
    }
}

#[allow(clippy::complexity)]
pub fn update_coyote_time(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &Puppeteer,
        Has<Jumping>,
        Has<Grounded>,
        Option<&mut CoyoteTime>,
    )>,
) {
    for (entity, controller, is_jumping, is_grounded, coyote_time) in query.iter_mut() {
        if !is_jumping && !is_grounded {
            if let Some(mut coyote_time) = coyote_time {
                coyote_time.0.tick(time.delta());
            } else {
                commands.entity(entity).insert(CoyoteTime(Timer::new(
                    controller.coyote_time,
                    TimerMode::Once,
                )));
            }
        } else if is_grounded {
            commands.entity(entity).remove::<CoyoteTime>();
        }
    }
}

pub fn update_jump_buffer(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PuppeteerInput, Option<&mut JumpBuffer>)>,
) {
    for (entity, mut input, jump_buffer) in query.iter_mut() {
        if let Some(mut jump_buffer) = jump_buffer {
            jump_buffer.0.tick(time.delta());
            if jump_buffer.finished() || !input.jump_start {
                input.jump_start = false;
                commands.entity(entity).remove::<JumpBuffer>();
            }
        }
    }
}
