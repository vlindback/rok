// transform.rs

use rok_math::mat4x4::Mat4x4;
use rok_math::quaternion::Quat;
use rok_math::vec3::Vec3;

#[derive(Copy, Clone)]
pub struct Transform {
    pub rotation: Quat,
    pub position: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub const fn identity() -> Self {
        Self {
            rotation: Quat::identity(),
            position: Vec3::new(0., 0., 0.),
            scale: Vec3::new(1., 1., 1.),
        }
    }

    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Self::identity()
        }
    }

    pub fn to_matrix(&self) -> Mat4x4 {
        Mat4x4::from_translation(self.position) * self.rotation.to_mat4x4()
    }
}
