# ```bevy_puppeteer```
[![Following released Bevy versions](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://bevy.org/learn/quick-start/plugin-development/#main-branch-tracking)
[![docs.rs](https://docs.rs/bevy_puppeteer/badge.svg)](https://docs.rs/bevy_puppeteer)
[![crates.io](https://img.shields.io/crates/v/bevy_puppeteer)](https://crates.io/crates/bevy_puppeteer)



A 3D kinematic character controller for the [Bevy game engine](https://bevy.org/), built on top of [Avian physics](https://github.com/Jondolf/avian).

---

## Usage

This plugin provides two main components: **Puppet** and **Puppeteer**.

- **Puppet** handles low-level collision and sliding logic.  
- **Puppeteer** is a higher-level character controller that manages a Puppet.

---

### Puppet

A **Puppet** is a simple entity that can move and interact with the world.  
It supports the following behaviors out of the box:

- Collision with objects  
- Sliding along walls  
- Stepping over obstacles  
- Sliding off slopes  

You can move a Puppet directly with the `move_to()` function.

---

### Puppeteer

**Puppeteer** builds on top of Puppet to provide a full-featured character controller.  

Current features include:

- Movement  
  - Acceleration, deceleration, turn speed  
  - Separate air acceleration, air deceleration, air turn speed  
- Gravity  
- Jumping  
  - Jump cutoff  
  - Air jumps  
- Coyote time  
- Jump buffer  

---

## Compatibility

| Bevy | bevy_puppeteer |
|------|-----------|
| 0.16 | 1.0       |
