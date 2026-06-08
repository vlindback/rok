// vec2.rs
//
// rok-math library
//

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::Lerp;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0., y: 0. }
    }

    #[inline]
    pub const fn unit_x() -> Self {
        Self { x: 1., y: 0. }
    }

    #[inline]
    pub const fn unit_y() -> Self {
        Self { x: 0., y: 1. }
    }

    #[inline]
    pub fn x(self) -> f32 {
        self.x
    }

    #[inline]
    pub fn y(self) -> f32 {
        self.y
    }

    #[inline]
    pub fn set_x(&mut self, x: f32) {
        self.x = x;
    }

    #[inline]
    pub fn set_y(&mut self, y: f32) {
        self.y = y;
    }

    #[inline]
    pub fn set(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    #[inline]
    pub fn dot_product(self, other: Vec2) -> f32 {
        (self.x * other.x) + (self.y * other.y)
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn distance(self, other: Vec2) -> f32 {
        (self - other).length()
    }

    #[inline]
    pub fn distance_squared(self, other: Vec2) -> f32 {
        (self - other).length_squared()
    }

    #[inline]
    pub fn normalize(&mut self) {
        let length = self.length();
        if length != 0. {
            self.x /= length;
            self.y /= length;
        } else {
            self.x = 0.;
            self.y = 0.;
        }
    }

    pub fn normalized(self) -> Self {
        let length = self.length();
        if length != 0. {
            Self {
                x: self.x / length,
                y: self.y / length,
            }
        } else {
            Self { x: 0., y: 0. }
        }
    }

    pub fn as_unit(self) -> Self {
        let mut copy = self;
        copy.normalize();
        copy
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self::Output {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl AddAssign<&Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self::Output {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl SubAssign<&Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: &Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Div for Vec2 {
    type Output = Self;

    #[inline]
    fn div(self, other: Self) -> Self::Output {
        Vec2 {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        self.x /= scalar;
        self.y /= scalar;
    }
}

impl DivAssign for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl DivAssign<&Vec2> for Vec2 {
    fn div_assign(&mut self, rhs: &Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl Mul for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self::Output {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self::Output {
        Vec2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
    }
}

impl MulAssign for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl MulAssign<&Vec2> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Neg for Vec2 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Lerp<f32> for Vec2 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        Vec2 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

// Convert from array: [f32; 2] -> Vec2<f32>
impl From<[f32; 2]> for Vec2 {
    #[inline]
    fn from(arr: [f32; 2]) -> Self {
        let [x, y] = arr;
        Vec2 { x, y }
    }
}

// Convert to array: Vec2<f32> -> [f32; 2]
impl From<Vec2> for [f32; 2] {
    #[inline]
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}
