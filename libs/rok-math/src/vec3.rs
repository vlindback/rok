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
        let length = self.length();
        if length != 0. {
            Self {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length,
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
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalizes the Vec3 producing a unit vector.
    pub fn normalize(&mut self) {
        let length = self.length();
        if length != 0. {
            self.x /= length;
            self.y /= length;
            self.z /= length;
        } else {
            self.x = 0.;
            self.y = 0.;
            self.z = 0.;
        }
    }

    /// Returns a copy of this Vec3 as a unit vector (length 1)
    pub fn as_unit(self) -> Self {
        let mut copy = self;
        copy.normalize();
        copy
    }

    #[inline]
    pub fn dot_product(self, other: Vec3) -> f32 {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z)
    }

    /// Returns a Vec3 that is perpendicular to this and other.
    #[inline]
    pub fn cross_product(self, other: Vec3) -> Vec3 {
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
