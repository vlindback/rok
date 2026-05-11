// lib.rs
//
// rok-math library
//

pub mod mat4x4;
pub mod quaternion;

pub mod lerp;
pub mod simd;

pub mod vec2;
pub mod vec3;
pub mod vec4;

// geometry

pub mod plane;

// re-exports:

pub use lerp::Lerp;

mod mat4x4_tests;
