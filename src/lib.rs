#![allow(dead_code)]
pub mod puppet;
pub mod puppeteer;

use avian3d::prelude::PhysicsSet;
use bevy::prelude::*;

use puppet::PuppetPlugin;
use puppeteer::{Jumping, Puppeteer, PuppeteerInput};

const MAX_BOUNCES: u32 = 5;

pub struct PuppeteerPlugin;

impl Plugin for PuppeteerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_type::<Puppeteer>()
            .register_type::<PuppeteerInput>()
            .register_type::<Jumping>();
        app.add_plugins(PuppetPlugin);
        app.configure_sets(
            FixedPostUpdate,
            (
                PuppeteerSet::Prepare,
                PuppeteerSet::Compute,
                PuppeteerSet::Move,
            )
                .chain()
                .before(PhysicsSet::Prepare),
        );

        app.add_systems(
            FixedUpdate,
            (
                puppeteer::movement,
                puppeteer::scale_gravity,
                puppeteer::update_coyote_time,
                puppeteer::update_jump_buffer,
                puppeteer::jumping,
            )
                .chain()
                .in_set(PuppeteerSet::Compute),
        );
    }
}
/// System set for puppeteer systems.
/// This runs in [`FixedPostUpdate`] before [`PhysicsSet::Prepare`]
///
/// 1. `Prepare`: Check if puppets are grounded and prepare/initialize components
/// 2. `Compute`: Compute movement, used by [`Puppeteer`]. This can be used to implement custom movement logic.
/// 3. `Move`: Move puppets with respect to collisions and other factors.
///     Note that the actual transform is updated by the physics engine.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PuppeteerSet {
    /// Check if puppets are grounded and prepare/initialize components
    Prepare,
    /// Compute movement, used by [`Puppeteer`]
    Compute,
    /// Move puppets with respect to collisions and other factors
    Move,
}
