// vec3.rs
//
// rok-math library
//

use crate::Lerp;
use crate::vec4::Vec4;

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

// Traits

impl Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Vec3 {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl AddAssign<&Vec3> for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        Vec3 {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl SubAssign<&Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: &Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Div for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, other: Self) -> Self::Output {
        Vec3 {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        self.x /= scalar;
        self.y /= scalar;
        self.z /= scalar;
    }
}

impl DivAssign for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl DivAssign<&Vec3> for Vec3 {
    fn div_assign(&mut self, rhs: &Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, scalar: f32) -> Self::Output {
        Vec3 {
            x: self.x / scalar,
            y: self.y / scalar,
            z: self.z / scalar,
        }
    }
}

impl Mul for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        Vec3 {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self::Output {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }
}

impl MulAssign for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl MulAssign<&Vec3> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl Neg for Vec3 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Vec3 {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Lerp<f32> for Vec3 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

// Convert from array: [f32; 3] -> Vec3<f32>
impl From<[f32; 3]> for Vec3 {
    fn from(arr: [f32; 3]) -> Self {
        let [x, y, z] = arr;
        Vec3 { x, y, z }
    }
}

// Convert to array: Vec3<f32> -> [f32; 3]
impl From<Vec3> for [f32; 3] {
    fn from(v: Vec3) -> Self {
        [v.x, v.y, v.z]
    }
}

// Implementation block

impl Vec3 {
    /// Below this squared length a vector is treated as zero during
    /// normalization (length cutoff = sqrt = 1e-4).
    const NORMALIZE_EPS: f32 = 1e-8;

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.y
    }

    #[inline]
    pub fn z(&self) -> f32 {
        self.z
    }

    #[inline]
    pub fn set_x(&mut self, x: f32) {
        self.x = x
    }

    #[inline]
    pub fn set_y(&mut self, y: f32) {
        self.y = y
    }

    #[inline]
    pub fn set_z(&mut self, z: f32) {
        self.z = z
    }

    #[inline]
    pub fn zero() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 0.,
        }
    }

    #[inline]
    pub fn unit_x() -> Self {
        Self {
            x: 1.,
            y: 0.,
            z: 0.,
        }
    }

    #[inline]
    pub fn unit_y() -> Self {
        Self {
            x: 0.,
            y: 1.,
            z: 0.,
        }
    }

    #[inline]
    pub fn unit_z() -> Self {
        Self {
            x: 0.,
            y: 0.,
            z: 1.,
        }
    }

    pub fn normalized(self) -> Self {
        let len_sq = self.length_squared();
        // Guard on len_sq, so the effective length cutoff is sqrt(1e-8) = 1e-4.
        if len_sq > Self::NORMALIZE_EPS {
            let inv_len = 1.0 / len_sq.sqrt();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            }
        } else {
            Self {
                x: 0.,
                y: 0.,
                z: 0.,
            }
        }
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline]
    pub fn distance(self, other: Vec3) -> f32 {
        (self - other).length()
    }

    #[inline]
    pub fn distance_squared(self, other: Vec3) -> f32 {
        (self - other).length_squared()
    }

    /// Normalizes the Vec3 producing a unit vector.
    pub fn normalize(&mut self) {
        let len_sq = self.length_squared();
        if len_sq > Self::NORMALIZE_EPS {
            let inv_len = 1.0 / len_sq.sqrt();
            self.x *= inv_len;
            self.y *= inv_len;
            self.z *= inv_len;
        } else {
            self.x = 0.;
            self.y = 0.;
            self.z = 0.;
        }
    }

    #[inline]
    pub fn dot(self, other: Vec3) -> f32 {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z)
    }

    /// Returns a Vec3 that is perpendicular to this and other.
    #[inline]
    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    #[inline]
    pub fn splat(val: f32) -> Self {
        Vec3 {
            x: val,
            y: val,
            z: val,
        }
    }

    /// Reflects `self` across the plane with unit `normal`.
    ///
    /// self = parallel + perp, parallel = (self·n) n  (n assumed unit).
    /// Reflection flips only the parallel part:
    ///   r = self - 2 (self·n) n
    ///
    /// Precondition: `normal` is normalized. We deliberately do NOT normalize
    /// here — that hides caller bugs and buries a sqrt in a hot path.
    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * (2.0 * self.dot(normal))
    }

    /// Vector projection of `self` onto `other`: the part of `self`
    /// lying along `other`. Returns a vector parallel to `other`.
    ///
    ///   proj = (self·other / |other|²) other
    ///
    /// `other` need NOT be unit. Undefined if `other` is the zero vector.
    #[inline]
    pub fn project_onto(self, other: Self) -> Self {
        debug_assert!(other.length_squared() > 0.0, "project_onto: zero vector");
        other * (self.dot(other) / other.dot(other))
    }

    /// Same, but skips the divide when `other` is already unit length.
    #[inline]
    pub fn project_onto_normalized(self, other: Self) -> Self {
        other * self.dot(other)
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x().min(other.x()),
            self.y().min(other.y()),
            self.z().min(other.z()),
        )
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x().max(other.x()),
            self.y().max(other.y()),
            self.z().max(other.z()),
        )
    }

    /// Per-component clamp into the box [min, max].
    ///
    /// Implemented as max(min).min(max). Note: NaN components of `self`
    /// collapse to `min` here (min/max treat NaN as "missing"), which is a
    /// useful sanitizing property. f32::clamp would instead PROPAGATE NaN.
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        debug_assert!(
            min.x() <= max.x() && min.y() <= max.y() && min.z() <= max.z(),
            "clamp: min must be <= max per component"
        );
        self.max(min).min(max)
    }

    /// True if ANY component is NaN (the vector is contaminated).
    #[inline]
    pub fn is_nan(self) -> bool {
        self.x().is_nan() || self.y().is_nan() || self.z().is_nan()
    }

    /// True if ALL components are finite (no NaN, no ±inf).
    #[inline]
    pub fn is_finite(self) -> bool {
        self.x().is_finite() && self.y().is_finite() && self.z().is_finite()
    }
}

impl From<Vec4> for Vec3 {
    #[inline]
    fn from(v: Vec4) -> Self {
        let arr = v.to_array();
        Vec3 {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }
}
