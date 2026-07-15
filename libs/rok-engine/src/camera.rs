// camera.rs
//

use rok_abi::input::ScanCode;
use rok_math::mat4x4::Mat4x4;
use rok_math::vec3::Vec3;

use crate::input::InputState;

pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,   // radians, horizontal orbit
    pub pitch: f32, // radians, vertical orbit
    pub distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,

    // Tuning.
    pub rotate_sensitivity: f32, // radians per raw mouse count
    pub zoom_speed: f32,         // distance units per second
}

impl OrbitCamera {
    pub fn new() -> Self {
        Self {
            target: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            distance: 3.0,     // matches the old hardcoded eye at (0,0,3)
            min_distance: 1.5, // cube half-extent is 0.5, stay outside it
            max_distance: 10.0,
            rotate_sensitivity: 0.003,
            zoom_speed: 5.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(
            self.target.x() + self.distance * cp * sy,
            self.target.y() + self.distance * sp,
            self.target.z() + self.distance * cp * cy,
        )
    }

    pub fn view(&self) -> Mat4x4 {
        Mat4x4::look_at(self.eye(), self.target, Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn update(&mut self, input: &InputState, dt: f32) {
        // Mouse delta -> orbit. Total rotation tracks total mouse movement,
        // so this is inherently frame-rate independent (no dt needed).
        // Flip either sign if it feels inverted.
        self.yaw -= input.mouse.delta_x as f32 * self.rotate_sensitivity;
        self.pitch -= input.mouse.delta_y as f32 * self.rotate_sensitivity;

        // Stop just short of the poles so the +Y up vector never flips.
        let limit = 89f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);

        // Zoom on Q/E, time-based, so held-key speed is frame-rate independent.
        if input.keyboard.key_down(ScanCode::E) {
            self.distance -= self.zoom_speed * dt; // E in
        }
        if input.keyboard.key_down(ScanCode::Q) {
            self.distance += self.zoom_speed * dt; // Q out
        }
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new()
    }
}
