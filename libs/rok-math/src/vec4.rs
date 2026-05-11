// vec4.rs
//
// rok-math library
//

use crate::simd::F32x4;

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Copy, Clone)]
pub struct Vec4 {
    v: F32x4,
}

impl Vec4 {
    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            v: F32x4::new(x, y, z, w),
        }
    }

    // Accessors

    #[inline]
    pub fn x(self) -> f32 {
        self.v.x()
    }

    #[inline]
    pub fn y(self) -> f32 {
        self.v.y()
    }

    #[inline]
    pub fn z(self) -> f32 {
        self.v.z()
    }

    #[inline]
    pub fn w(self) -> f32 {
        self.v.w()
    }

    #[inline]
    pub(crate) fn to_array(self) -> [f32; 4] {
        self.v.to_array()
    }

    // crate-private

    #[inline(always)]
    pub(crate) fn from_simd(v: F32x4) -> Self {
        Self { v }
    }

    #[inline(always)]
    pub(crate) fn to_simd(self) -> F32x4 {
        self.v
    }
}

// conversions

impl From<F32x4> for Vec4 {
    #[inline(always)]
    fn from(v: F32x4) -> Self {
        Self { v }
    }
}

impl From<Vec4> for F32x4 {
    #[inline]
    fn from(v: Vec4) -> Self {
        v.v
    }
}
