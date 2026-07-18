// vec4.rs
//
// rok-math library
//

use crate::{Lerp, simd::F32x4, vec3::Vec3};

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Copy, Clone)]
pub struct Vec4 {
    v: F32x4,
}

// Traits

impl Add for Vec4 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Vec4 {
            v: self.v.add(other.v),
        }
    }
}

impl AddAssign for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.v.add_assign(rhs.v);
    }
}

impl AddAssign<&Vec4> for Vec4 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.v.add_assign(rhs.v);
    }
}

impl Sub for Vec4 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        Vec4 {
            v: self.v.sub(other.v),
        }
    }
}

impl SubAssign for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.v.sub_assign(rhs.v);
    }
}

impl SubAssign<&Vec4> for Vec4 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        self.v.sub_assign(rhs.v);
    }
}

impl Div for Vec4 {
    type Output = Self;

    #[inline]
    fn div(self, other: Self) -> Self::Output {
        Vec4 {
            v: self.v.div(other.v),
        }
    }
}

impl DivAssign<f32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        let scalar_vector = F32x4::splat(scalar);
        self.v.div_assign(scalar_vector);
    }
}

impl DivAssign for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.v.div_assign(rhs.v);
    }
}

impl DivAssign<&Vec4> for Vec4 {
    #[inline]
    fn div_assign(&mut self, rhs: &Self) {
        self.v.div_assign(rhs.v);
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self::Output {
        let scalar_vector = F32x4::splat(scalar);
        Vec4 {
            v: self.v.div(scalar_vector),
        }
    }
}

impl Mul for Vec4 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        Vec4 {
            v: self.v.mul(other.v),
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self::Output {
        let scalar_vector = F32x4::splat(scalar);
        Vec4 {
            v: self.v.mul(scalar_vector),
        }
    }
}

impl MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        let scalar_vector = F32x4::splat(scalar);
        self.v.mul_assign(scalar_vector);
    }
}

impl MulAssign for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.v.mul_assign(rhs.v);
    }
}

impl MulAssign<&Vec4> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.v.mul_assign(rhs.v);
    }
}

impl Neg for Vec4 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        let scalar_vector = F32x4::splat(-0.);
        Vec4 {
            v: self.v.bit_xor(scalar_vector),
        }
    }
}

impl Lerp<f32> for Vec4 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t_vec = F32x4::splat(t);
        let one_vec = F32x4::splat(1.);

        // (1.0 - t)
        let one_minus_t = one_vec.sub(t_vec);

        // (1.0 - t) * a
        let term1 = one_minus_t.mul(self.v);

        // t * b
        let term2 = t_vec.mul(other.v);

        Vec4 {
            v: term1.add(term2),
        }
    }
}

// Convert from array: [f32; 4] -> Vec4
impl From<[f32; 4]> for Vec4 {
    #[inline]
    fn from(arr: [f32; 4]) -> Self {
        Vec4 {
            v: F32x4::load(&arr),
        }
    }
}

// Convert to array: Vec4 -> [f32; 4]
impl From<Vec4> for [f32; 4] {
    #[inline]
    fn from(v: Vec4) -> Self {
        v.v.to_array()
    }
}

// Implementation block

impl Vec4 {
    /// Below this squared length a vector is treated as zero during
    /// normalization (length cutoff = sqrt = 1e-4).
    const NORMALIZE_EPS: f32 = 1e-8;

    // Construction

    #[inline]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            v: F32x4::new(x, y, z, w),
        }
    }

    #[inline]
    pub fn from_vec3(v3: Vec3, w: f32) -> Self {
        Self {
            v: F32x4::new(v3.x(), v3.y(), v3.z(), w),
        }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self { v: F32x4::splat(v) }
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

    // Setters

    #[inline]
    pub fn set_x(&mut self, x: f32) {
        self.v = self.v.insert::<0x00>(x);
    }
    #[inline]
    pub fn set_y(&mut self, y: f32) {
        self.v = self.v.insert::<0x10>(y);
    }
    #[inline]
    pub fn set_z(&mut self, z: f32) {
        self.v = self.v.insert::<0x20>(z);
    }
    #[inline]
    pub fn set_w(&mut self, w: f32) {
        self.v = self.v.insert::<0x30>(w);
    }

    // Utility

    #[inline]
    pub(crate) fn to_array(self) -> [f32; 4] {
        self.v.to_array()
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.v.dot::<0xF1>(other.v).x()
    }

    #[inline]
    pub fn zero() -> Self {
        Self { v: F32x4::zero() }
    }

    #[inline]
    pub fn unit_x() -> Self {
        Self {
            v: F32x4::new(1., 0., 0., 0.),
        }
    }

    #[inline]
    pub fn unit_y() -> Self {
        Self {
            v: F32x4::new(0., 1., 0., 0.),
        }
    }

    #[inline]
    pub fn unit_z() -> Self {
        Self {
            v: F32x4::new(0., 0., 1., 0.),
        }
    }

    #[inline]
    pub fn unit_w() -> Self {
        Self {
            v: F32x4::new(0., 0., 0., 1.),
        }
    }

    pub fn normalized(self) -> Self {
        let len_sq = self.length_squared();
        // Guard on len_sq, so the effective length cutoff is sqrt(1e-8) = 1e-4.
        if len_sq > Self::NORMALIZE_EPS {
            let inv_len = 1.0 / len_sq.sqrt();
            Self {
                v: F32x4::mul(self.v, F32x4::splat(inv_len)),
            }
        } else {
            Self::zero()
        }
    }

    #[inline]
    pub fn normalize(&mut self) {
        let len_sq = self.length_squared();
        if len_sq > Self::NORMALIZE_EPS {
            let inv_len = 1.0 / len_sq.sqrt();
            self.v.mul_assign(F32x4::splat(inv_len));
        } else {
            self.v = F32x4::zero()
        }
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn distance(self, other: Vec4) -> f32 {
        (self - other).length()
    }

    #[inline]
    pub fn distance_squared(self, other: Vec4) -> f32 {
        (self - other).length_squared()
    }

    #[inline]
    pub fn is_nan(self) -> bool {
        // A NaN lane fails self == self, so a full mask means "no NaN present".
        self.v.cmpeq(self.v).movemask() != 0b1111
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        // finite - finite == 0 exactly; (inf - inf) and (nan - nan) are NaN.
        // So every lane compares equal to 0 iff every lane was finite.
        let d = self.v.sub(self.v);
        d.cmpeq(F32x4::splat(0.0)).movemask() == 0b1111
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        // f32::min semantics: where `other` is NaN, keep self.
        // _mm_min_ps is correct when self is NaN (returns other) but returns
        // NaN when other is NaN, so patch those lanes back to self.
        let other_not_nan = other.v.cmpeq(other.v); // set where other is NOT NaN
        let m = self.v.min(other.v);
        Self {
            v: F32x4::blendv(self.v, m, other_not_nan),
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        let other_not_nan = other.v.cmpeq(other.v);
        let m = self.v.max(other.v);
        Self {
            v: F32x4::blendv(self.v, m, other_not_nan),
        }
    }

    /// Per-component clamp into the box [min, max].
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        debug_assert!(
            min.x() <= max.x() && min.y() <= max.y() && min.z() <= max.z() && min.w() <= max.w(),
            "clamp: min must be <= max per component"
        );
        self.max(min).min(max)
    }

    /// Vector projection of `self` onto `onto`: (dot(self,onto) / dot(onto,onto)) * onto.
    /// Returns zero if `onto` is degenerate (same epsilon policy as `normalize`).
    #[inline]
    pub fn project_onto(self, onto: Self) -> Self {
        let onto_len_sq = onto.length_squared();
        if onto_len_sq > Self::NORMALIZE_EPS {
            let scale = self.dot(onto) / onto_len_sq;
            onto * scale
        } else {
            Self::zero()
        }
    }

    /// Projection onto a *unit* `onto`. Skips the divide — caller guarantees ‖onto‖ = 1.
    #[inline]
    pub fn project_onto_normalized(self, onto: Self) -> Self {
        onto * self.dot(onto)
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

// PartialEq

impl PartialEq for Vec4 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (self.v.cmpeq(other.v)).movemask() == 0b1111
    }
}

#[test]
fn min_max_nan_matches_scalar_semantics() {
    let n = f32::NAN;
    // NaN in the second operand: f32::min keeps the first (non-NaN) operand.
    let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vec4::new(n, n, n, n);
    assert_eq!(a.min(b), a); // other is NaN everywhere -> keep self
    assert_eq!(a.max(b), a);

    // NaN in the first operand: _mm_min_ps already returns the second here,
    // which matches f32::min (non-NaN wins).
    assert_eq!(b.min(a), a);
    assert_eq!(b.max(a), a);
}
