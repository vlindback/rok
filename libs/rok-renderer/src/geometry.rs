// geometry.rs
//

use crate::backend::vertex::Vertex;
use rok_math::{vec2::Vec2, vec3::Vec3};

/// 24-vertex unit cube: 4 verts per face so each can carry its own UV,
/// normal, and tangent. Wound CCW-outward.
pub fn cube() -> ([Vertex; 24], [u16; 36]) {
    let h = 0.5;
    let vertices = [
        Vertex {
            position: Vec3::new(-h, -h, h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, -h, h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, h, h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(-h, h, h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        // -Z back, n = (0,0,-1)
        Vertex {
            position: Vec3::new(h, -h, -h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            tangent: Vec3::new(-1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(-h, -h, -h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            tangent: Vec3::new(-1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(-h, h, -h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            tangent: Vec3::new(-1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, h, -h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(0.0, 0.0, -1.0),
            tangent: Vec3::new(-1.0, 0.0, 0.0),
        },
        // +X right, n = (1,0,0)
        Vertex {
            position: Vec3::new(h, -h, h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, -1.0),
        },
        Vertex {
            position: Vec3::new(h, -h, -h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, -1.0),
        },
        Vertex {
            position: Vec3::new(h, h, -h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, -1.0),
        },
        Vertex {
            position: Vec3::new(h, h, h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, -1.0),
        },
        // -X left, n = (-1,0,0)
        Vertex {
            position: Vec3::new(-h, -h, -h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
        },
        Vertex {
            position: Vec3::new(-h, -h, h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
        },
        Vertex {
            position: Vec3::new(-h, h, h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
        },
        Vertex {
            position: Vec3::new(-h, h, -h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            tangent: Vec3::new(0.0, 0.0, 1.0),
        },
        // +Y top, n = (0,1,0)
        Vertex {
            position: Vec3::new(-h, h, h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, h, h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, h, -h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(-h, h, -h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        // -Y bottom, n = (0,-1,0)
        Vertex {
            position: Vec3::new(-h, -h, -h),
            uv: Vec2::new(0.0, 1.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, -h, -h),
            uv: Vec2::new(1.0, 1.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(h, -h, h),
            uv: Vec2::new(1.0, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(-h, -h, h),
            uv: Vec2::new(0.0, 0.0),
            normal: Vec3::new(0.0, -1.0, 0.0),
            tangent: Vec3::new(1.0, 0.0, 0.0),
        },
    ];

    let indices: [u16; 36] = [
        0, 1, 2, 2, 3, 0, // +Z
        4, 5, 6, 6, 7, 4, // -Z
        8, 9, 10, 10, 11, 8, // +X
        12, 13, 14, 14, 15, 12, // -X
        16, 17, 18, 18, 19, 16, // +Y
        20, 21, 22, 22, 23, 20, // -Y
    ];

    (vertices, indices)
}
