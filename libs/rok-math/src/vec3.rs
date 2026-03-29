// vec3.rs

use std::ops::{Add, Mul};

pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T>
where
    T: Copy + Add<Output = T> + Mul<Output = T>,
{
    pub fn dot_product(self, other: Vector3<T>) -> T {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z)
    }
}
