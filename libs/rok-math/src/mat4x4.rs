// mat4x4
//
// rok-math library
//

use crate::quaternion::Quat;
use crate::simd::{F32x4, shuffle_mask};
use crate::vec3::Vec3;
use crate::vec4::Vec4;

use std::ops::{Mul, MulAssign};

/// A column major 4x4 Matrix (F32)
/// ```
/// [1 0 0 tx]
/// [0 1 0 ty]
/// [0 0 1 tz]
/// [0 0 0 1 ]
/// ```
#[derive(Debug, Copy, Clone)]
pub struct Mat4x4 {
    pub(crate) cols: [F32x4; 4],
}

impl Mat4x4 {
    pub fn new() -> Self {
        Self {
            cols: [
                F32x4::new(1., 0., 0., 0.),
                F32x4::new(0., 1., 0., 0.),
                F32x4::new(0., 0., 1., 0.),
                F32x4::new(0., 0., 0., 1.),
            ],
        }
    }

    #[inline]
    pub fn identity() -> Self {
        Self::new()
    }

    pub fn zero() -> Self {
        Self {
            cols: [F32x4::zero(), F32x4::zero(), F32x4::zero(), F32x4::zero()],
        }
    }

    /// Creates a Mat4x4 from a scale at origin.
    pub fn from_scale(sx: f32, sy: f32, sz: f32) -> Self {
        Self {
            cols: [
                F32x4::new(sx, 0.0, 0.0, 0.0),
                F32x4::new(0.0, sy, 0.0, 0.0),
                F32x4::new(0.0, 0.0, sz, 0.0),
                F32x4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    /// Creates a Mat4x4 from a 3D position.
    pub fn from_translation(t: Vec3) -> Self {
        Self {
            cols: [
                F32x4::new(1.0, 0.0, 0.0, 0.0),
                F32x4::new(0.0, 1.0, 0.0, 0.0),
                F32x4::new(0.0, 0.0, 1.0, 0.0),
                F32x4::new(t.x(), t.y(), t.z(), 1.0),
            ],
        }
    }

    pub fn from_rotation_x(angle_rad: f32) -> Self {
        let (s, c) = angle_rad.sin_cos();
        Self {
            cols: [
                F32x4::new(1.0, 0.0, 0.0, 0.0),
                F32x4::new(0.0, c, s, 0.0),
                F32x4::new(0.0, -s, c, 0.0),
                F32x4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    pub fn from_rotation_y(angle_rad: f32) -> Self {
        let (s, c) = angle_rad.sin_cos();
        Self {
            cols: [
                F32x4::new(c, 0.0, -s, 0.0),
                F32x4::new(0.0, 1.0, 0.0, 0.0),
                F32x4::new(s, 0.0, c, 0.0),
                F32x4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    pub fn from_rotation_z(angle_rad: f32) -> Self {
        let (s, c) = angle_rad.sin_cos();
        Self {
            cols: [
                F32x4::new(c, s, 0.0, 0.0),
                F32x4::new(-s, c, 0.0, 0.0),
                F32x4::new(0.0, 0.0, 1.0, 0.0),
                F32x4::new(0.0, 0.0, 0.0, 1.0),
            ],
        }
    }

    /// Construct from four column vectors.
    pub fn from_cols(c0: Vec4, c1: Vec4, c2: Vec4, c3: Vec4) -> Self {
        // Vec4 wraps F32x4 — assume into-conversion or direct field access.
        Self {
            cols: [c0.into(), c1.into(), c2.into(), c3.into()],
        }
    }

    /// Construct from 16 f32s in column-major order.
    /// `m00` is column 0 row 0; `m01` is column 0 row 1; etc.
    /// Reads as: column 0 first, then column 1, then column 2, then column 3.
    #[allow(clippy::too_many_arguments)]
    pub fn from_cols_array(
        m00: f32,
        m01: f32,
        m02: f32,
        m03: f32,
        m10: f32,
        m11: f32,
        m12: f32,
        m13: f32,
        m20: f32,
        m21: f32,
        m22: f32,
        m23: f32,
        m30: f32,
        m31: f32,
        m32: f32,
        m33: f32,
    ) -> Self {
        Self {
            cols: [
                F32x4::new(m00, m01, m02, m03),
                F32x4::new(m10, m11, m12, m13),
                F32x4::new(m20, m21, m22, m23),
                F32x4::new(m30, m31, m32, m33),
            ],
        }
    }

    /// Construct from a `[f32; 16]` slice in column-major order.
    pub fn from_cols_slice(m: &[f32; 16]) -> Self {
        Self {
            cols: [
                F32x4::new(m[0], m[1], m[2], m[3]),
                F32x4::new(m[4], m[5], m[6], m[7]),
                F32x4::new(m[8], m[9], m[10], m[11]),
                F32x4::new(m[12], m[13], m[14], m[15]),
            ],
        }
    }

    /// Creates a Mat4x4 from a Translation, Rotation (Quaternion), and Scale.
    /// This is significantly faster than multiplying T * R * S matrices.
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let x = rotation.x();
        let y = rotation.y();
        let z = rotation.z();
        let w = rotation.w();

        // Pre-compute to avoid redundant multiplications
        let x2 = x + x;
        let y2 = y + y;
        let z2 = z + z;

        let xx = x * x2;
        let xy = x * y2;
        let xz = x * z2;

        let yy = y * y2;
        let yz = y * z2;
        let zz = z * z2;

        let wx = w * x2;
        let wy = w * y2;
        let wz = w * z2;

        let sx = scale.x();
        let sy = scale.y();
        let sz = scale.z();

        Self {
            cols: [
                // Column 0
                F32x4::new((1.0 - (yy + zz)) * sx, (xy + wz) * sx, (xz - wy) * sx, 0.0),
                // Column 1
                F32x4::new((xy - wz) * sy, (1.0 - (xx + zz)) * sy, (yz + wx) * sy, 0.0),
                // Column 2
                F32x4::new((xz + wy) * sz, (yz - wx) * sz, (1.0 - (xx + yy)) * sz, 0.0),
                // Column 3 (Translation)
                F32x4::new(translation.x(), translation.y(), translation.z(), 1.0),
            ],
        }
    }

    pub fn to_cols_array(&self) -> [f32; 16] {
        let c0 = self.cols[0].to_array();
        let c1 = self.cols[1].to_array();
        let c2 = self.cols[2].to_array();
        let c3 = self.cols[3].to_array();
        [
            c0[0], c0[1], c0[2], c0[3], c1[0], c1[1], c1[2], c1[3], c2[0], c2[1], c2[2], c2[3],
            c3[0], c3[1], c3[2], c3[3],
        ]
    }

    #[inline]
    pub fn x_axis(&self) -> Vec4 {
        Vec4::from_simd(self.cols[0])
    }
    #[inline]
    pub fn y_axis(&self) -> Vec4 {
        Vec4::from_simd(self.cols[1])
    }
    #[inline]
    pub fn z_axis(&self) -> Vec4 {
        Vec4::from_simd(self.cols[2])
    }
    #[inline]
    pub fn w_axis(&self) -> Vec4 {
        Vec4::from_simd(self.cols[3])
    }

    pub fn determinant(self) -> f32 {
        let col = self.cols;

        let cof_a = (col[2].z() * col[3].w()) - (col[3].z() * col[2].w());
        let cof_b = (col[2].y() * col[3].w()) - (col[3].y() * col[2].w());
        let cof_c = (col[2].y() * col[3].z()) - (col[3].y() * col[2].z());
        let cof_d = (col[2].x() * col[3].w()) - (col[3].x() * col[2].w());
        let cof_e = (col[2].x() * col[3].z()) - (col[3].x() * col[2].z());
        let cof_f = (col[2].x() * col[3].y()) - (col[3].x() * col[2].y());

        let minor00 = col[1].y() * cof_a - col[1].z() * cof_b + col[1].w() * cof_c;
        let minor10 = col[1].x() * cof_a - col[1].z() * cof_d + col[1].w() * cof_e;
        let minor20 = col[1].x() * cof_b - col[1].y() * cof_d + col[1].w() * cof_f;
        let minor30 = col[1].x() * cof_c - col[1].y() * cof_e + col[1].z() * cof_f;

        let det = col[0].x() * minor00 - col[0].y() * minor10 + col[0].z() * minor20
            - col[0].w() * minor30;
        det
    }

    /// Returns a transposed copy of this matrix.
    pub fn transpose(&self) -> Self {
        // 1: Unpack lower and upper halves
        let tmp0 = F32x4::unpack_lo(self.cols[0], self.cols[1]);
        let tmp1 = F32x4::unpack_lo(self.cols[2], self.cols[3]);
        let tmp2 = F32x4::unpack_hi(self.cols[0], self.cols[1]);
        let tmp3 = F32x4::unpack_hi(self.cols[2], self.cols[3]);
        // 2: Shuffle to get the final transposed columns.
        Self {
            cols: [
                F32x4::shuffle::<{ shuffle_mask(1, 0, 1, 0) }>(tmp0, tmp1), // [x0, x1, x2, x3]
                F32x4::shuffle::<{ shuffle_mask(3, 2, 3, 2) }>(tmp0, tmp1), // [y0, y1, y2, y3]
                F32x4::shuffle::<{ shuffle_mask(1, 0, 1, 0) }>(tmp2, tmp3), // [z0, z1, z2, z3]
                F32x4::shuffle::<{ shuffle_mask(3, 2, 3, 2) }>(tmp2, tmp3), // [w0, w1, w2, w3]
            ],
        }
    }

    //     pub fn t(self) {
    //         let col = self.cols;

    //         let cof_a = (col[2].z() * col[3].w()) - (col[3].z() * col[2].w());
    //         let cof_b = (col[2].y() * col[3].w()) - (col[3].y() * col[2].w());
    //         let cof_c = (col[2].y() * col[3].z()) - (col[3].y() * col[2].z());
    //         let cof_d = (col[2].x() * col[3].w()) - (col[3].x() * col[2].w());

    //         let cof_e = (col[2].x() * col[3].z()) - (col[3].x() * col[2].z());
    //         let cof_f = (col[2].x() * col[3].y()) - (col[3].x() * col[2].y());
    //         let cof_j = (col[2].x() * col[3].w()) - (col[2].w() * col[3].x());
    //         let cof_k = (col[2].x() * col[3].z()) - (col[2].z() * col[3].x());

    //         let cof_m = (col[1].z() * col[3].w()) - (col[1].w() * col[3].z());
    //         let cof_n = (col[1].y() * col[3].w()) - (col[1].w() * col[3].y());
    //         let cof_o = (col[1].y() * col[3].z()) - (col[1].z() * col[3].y());
    //         let cof_p = (col[1].x() * col[3].w()) - (col[1].w() * col[3].x());

    //         let cof_q = (col[1].x() * col[3].z()) - (col[1].z() * col[3].x());
    //         let cof_r = (col[1].x() * col[3].y()) - (col[1].y() * col[3].x());
    //         let cof_s = (col[1].z() * col[2].w()) - (col[1].w() * col[2].z());
    //         let cof_t = (col[1].y() * col[2].w()) - (col[1].w() * col[2].y());

    //         let cof_u = (col[1].y() * col[2].z()) - (col[1].z() * col[2].y());
    //         let cof_v = (col[1].x() * col[2].w()) - (col[1].w() * col[2].x());
    //         let cof_w = (col[1].x() * col[2].z()) - (col[1].z() * col[2].x());
    //         let cof_x = (col[1].x() * col[2].y()) - (col[1].y() * col[2].x());
    //     }

    pub fn inverse(self) -> Option<Self> {
        let c0 = self.cols[0];
        let c1 = self.cols[1];
        let c2 = self.cols[2];
        let c3 = self.cols[3];

        // Calculate 2x2 determinants for pairs of columns
        let (d_a_23, d_b_23) = Self::calc_d(c2, c3);
        let (d_a_13, d_b_13) = Self::calc_d(c1, c3);
        let (d_a_12, d_b_12) = Self::calc_d(c1, c2);

        // Compute the cofactor rows (which form the columns of the adjugate)
        let adj_0 = Self::cross4(c1, d_a_23, d_b_23);
        let adj_1 = Self::cross4(c0, d_a_23, d_b_23);
        let adj_2 = Self::cross4(c0, d_a_13, d_b_13);
        let adj_3 = Self::cross4(c0, d_a_12, d_b_12);

        // Apply checkerboard signs via bitwise XOR
        let flip_pnpn = F32x4::new(0.0, -0.0, 0.0, -0.0);
        let flip_npnp = F32x4::new(-0.0, 0.0, -0.0, 0.0);

        let adj_0_signed = adj_0.bit_xor(flip_pnpn);
        let adj_1_signed = adj_1.bit_xor(flip_npnp);
        let adj_2_signed = adj_2.bit_xor(flip_pnpn);
        let adj_3_signed = adj_3.bit_xor(flip_npnp);

        // Calculate determinant via dot product of col0 and adj_0
        let det_vec = c0.dot::<0xFF>(adj_0_signed);
        let det = det_vec.x();

        const EPS: f32 = 1e-6;
        if det.abs() < EPS {
            return None;
        }

        // Multiply by inverse determinant
        let inv_det = F32x4::splat(1.0).div(det_vec);

        let r0 = adj_0_signed.mul(inv_det);
        let r1 = adj_1_signed.mul(inv_det);
        let r2 = adj_2_signed.mul(inv_det);
        let r3 = adj_3_signed.mul(inv_det);

        // Transpose to form the final inverse columns
        let (inv_c0, inv_c1, inv_c2, inv_c3) = Self::transpose4(r0, r1, r2, r3);

        Some(Mat4x4::from_cols(
            Vec4::from(inv_c0),
            Vec4::from(inv_c1),
            Vec4::from(inv_c2),
            Vec4::from(inv_c3),
        ))
    }

    #[inline(always)]
    fn calc_d(a: F32x4, b: F32x4) -> (F32x4, F32x4) {
        // d_a calculates [zw, yw, yz, yz]
        let a_l1 = F32x4::shuffle::<{ shuffle_mask(1, 1, 1, 2) }>(a, a);
        let b_r1 = F32x4::shuffle::<{ shuffle_mask(2, 2, 3, 3) }>(b, b);
        let b_l2 = F32x4::shuffle::<{ shuffle_mask(1, 1, 1, 2) }>(b, b);
        let a_r2 = F32x4::shuffle::<{ shuffle_mask(2, 2, 3, 3) }>(a, a);
        let d_a = a_l1.mul(b_r1).sub(b_l2.mul(a_r2));

        // d_b calculates [xw, xz, xy, xy]
        let a_l3 = F32x4::shuffle::<{ shuffle_mask(0, 0, 0, 0) }>(a, a);
        let b_r3 = F32x4::shuffle::<{ shuffle_mask(1, 1, 2, 3) }>(b, b);
        let b_l4 = F32x4::shuffle::<{ shuffle_mask(0, 0, 0, 0) }>(b, b);
        let a_r4 = F32x4::shuffle::<{ shuffle_mask(1, 1, 2, 3) }>(a, a);
        let d_b = a_l3.mul(b_r3).sub(b_l4.mul(a_r4));

        (d_a, d_b)
    }

    /// Computes a generalized 4D cross-product to generate an adjugate row
    #[inline(always)]
    fn cross4(c: F32x4, d_a: F32x4, d_b: F32x4) -> F32x4 {
        let v_x = F32x4::shuffle::<{ shuffle_mask(0, 0, 0, 1) }>(c, c);
        let v_y = F32x4::shuffle::<{ shuffle_mask(1, 1, 2, 2) }>(c, c);
        let v_z = F32x4::shuffle::<{ shuffle_mask(2, 3, 3, 3) }>(c, c);

        let w_x = F32x4::shuffle::<{ shuffle_mask(2, 1, 0, 0) }>(d_a, d_a);

        let t0 = F32x4::shuffle::<{ shuffle_mask(0, 0, 1, 1) }>(d_a, d_b);
        let w_y = F32x4::shuffle::<{ shuffle_mask(1, 0, 2, 0) }>(t0, d_b);

        let t1 = F32x4::shuffle::<{ shuffle_mask(2, 1, 2, 2) }>(d_a, d_b);
        let w_z = F32x4::shuffle::<{ shuffle_mask(3, 3, 2, 0) }>(t1, t1);

        v_x.mul(w_x).sub(v_y.mul(w_y)).add(v_z.mul(w_z))
    }

    /// Fast 4x4 matrix transpose using standard unpacking
    #[inline(always)]
    fn transpose4(r0: F32x4, r1: F32x4, r2: F32x4, r3: F32x4) -> (F32x4, F32x4, F32x4, F32x4) {
        let tmp0 = F32x4::unpack_lo(r0, r1);
        let tmp1 = F32x4::unpack_hi(r0, r1);
        let tmp2 = F32x4::unpack_lo(r2, r3);
        let tmp3 = F32x4::unpack_hi(r2, r3);

        let c0 = F32x4::shuffle::<{ shuffle_mask(1, 0, 1, 0) }>(tmp0, tmp2);
        let c1 = F32x4::shuffle::<{ shuffle_mask(3, 2, 3, 2) }>(tmp0, tmp2);
        let c2 = F32x4::shuffle::<{ shuffle_mask(1, 0, 1, 0) }>(tmp1, tmp3);
        let c3 = F32x4::shuffle::<{ shuffle_mask(3, 2, 3, 2) }>(tmp1, tmp3);

        (c0, c1, c2, c3)
    }

    /// Transform a position. w = 1, so translation applies.
    /// Assumes an AFFINE matrix (no perspective divide). For a projection
    /// matrix you must divide by the resulting w yourself.
    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        let r = self.cols[0] * F32x4::splat(p.x())
            + self.cols[1] * F32x4::splat(p.y())
            + self.cols[2] * F32x4::splat(p.z())
            + self.cols[3]; // implicit * 1.0
        Vec3::new(r.x(), r.y(), r.z())
    }

    /// Transform a direction. w = 0, so translation is ignored
    /// only the upper-left 3x3 (the linear part) acts.
    #[inline]
    pub fn transform_vector(&self, d: Vec3) -> Vec3 {
        let r = self.cols[0] * F32x4::splat(d.x())
            + self.cols[1] * F32x4::splat(d.y())
            + self.cols[2] * F32x4::splat(d.z());
        Vec3::new(r.x(), r.y(), r.z())
    }

    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalized(); // forward
        let r = f.cross(up).normalized(); // right
        let u = r.cross(f); // re-orthogonalized up

        Self {
            cols: [
                F32x4::new(r.x(), u.x(), -f.x(), 0.0),
                F32x4::new(r.y(), u.y(), -f.y(), 0.0),
                F32x4::new(r.z(), u.z(), -f.z(), 0.0),
                F32x4::new(-r.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
            ],
        }
    }

    /// RH view space, [0,1] depth, REVERSE-Z (near->1, far->0).
    /// Y is NOT flipped here — handled by negative viewport height in the renderer.
    /// Renderer must clear depth to 0.0 and use COMPARE_OP_GREATER.
    pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y_rad * 0.5).tan();
        Self {
            cols: [
                F32x4::new(f / aspect, 0.0, 0.0, 0.0),
                F32x4::new(0.0, f, 0.0, 0.0),
                F32x4::new(0.0, 0.0, near / (far - near), -1.0),
                F32x4::new(0.0, 0.0, (near * far) / (far - near), 0.0),
            ],
        }
    }

    /// Original hand written naive inverse, kept for testing purposes.
    pub(crate) fn inverse_naive(self) -> Option<Self> {
        //
        // The inverse A⁻¹ is defined by: A * A⁻¹ = I
        //
        // We compute it via the adjugate (classical adjoint) method:
        //
        //   A⁻¹ = adj(A) / det(A)
        //
        // where adj(A) is the transpose of the cofactor matrix. Each cofactor
        // C[i][j] is the signed determinant of the 3×3 minor you get by
        // deleting row i and column j of A. The sign follows a checkerboard
        // pattern (+ - + - / - + - + / ...).
        //
        // So: 16 cofactors, each a 3x3 determinant. We compute each 3×3
        // determinant by expanding along a row, which gives 3 weighted 2×2
        // sub-determinants. The 2x2s have lots of overlap across the 16
        // cofactors - the same column pair shows up repeatedly - so the
        // folded form below lifts those out and reuses them.
        //
        // Singular matrices (det == 0) have no inverse; we return None.

        let col = self.cols;
        let cof_a = (col[2].z() * col[3].w()) - (col[3].z() * col[2].w());
        let cof_b = (col[2].y() * col[3].w()) - (col[3].y() * col[2].w());
        let cof_c = (col[2].y() * col[3].z()) - (col[3].y() * col[2].z());
        let minor00 = col[1].y() * cof_a - col[1].z() * cof_b + col[1].w() * cof_c;
        let cof_d = (col[2].x() * col[3].w()) - (col[3].x() * col[2].w());
        let cof_e = (col[2].x() * col[3].z()) - (col[3].x() * col[2].z());
        let minor10 = col[1].x() * cof_a - col[1].z() * cof_d + col[1].w() * cof_e;
        let cof_f = (col[2].x() * col[3].y()) - (col[3].x() * col[2].y());
        let minor20 = col[1].x() * cof_b - col[1].y() * cof_d + col[1].w() * cof_f;

        let minor30 = col[1].x() * cof_c - col[1].y() * cof_e + col[1].z() * cof_f;
        let det = col[0].x() * minor00 - col[0].y() * minor10 + col[0].z() * minor20
            - col[0].w() * minor30;

        // Guard against singular inverses.
        const EPS: f32 = 1e-6;
        if det.abs() < EPS {
            return None;
        }

        let minor01 = col[0].y() * cof_a - col[0].z() * cof_b + col[0].w() * cof_c;
        let cof_j = (col[2].x() * col[3].w()) - (col[2].w() * col[3].x());
        let cof_k = (col[2].x() * col[3].z()) - (col[2].z() * col[3].x());
        let minor11 = col[0].x() * cof_a - col[0].z() * cof_j + col[0].w() * cof_k;
        let minor21 = col[0].x() * cof_b - col[0].y() * cof_j + col[0].w() * cof_f;
        let minor31 = col[0].x() * cof_c - col[0].y() * cof_k + col[0].z() * cof_f;
        let cof_m = (col[1].z() * col[3].w()) - (col[1].w() * col[3].z());
        let cof_n = (col[1].y() * col[3].w()) - (col[1].w() * col[3].y());
        let cof_o = (col[1].y() * col[3].z()) - (col[1].z() * col[3].y());
        let minor02 = col[0].y() * cof_m - col[0].z() * cof_n + col[0].w() * cof_o;
        let cof_p = (col[1].x() * col[3].w()) - (col[1].w() * col[3].x());
        let cof_q = (col[1].x() * col[3].z()) - (col[1].z() * col[3].x());
        let minor12 = col[0].x() * cof_m - col[0].z() * cof_p + col[0].w() * cof_q;
        let cof_r = (col[1].x() * col[3].y()) - (col[1].y() * col[3].x());
        let minor22 = col[0].x() * cof_n - col[0].y() * cof_p + col[0].w() * cof_r;
        let minor32 = col[0].x() * cof_o - col[0].y() * cof_q + col[0].z() * cof_r;
        let cof_s = (col[1].z() * col[2].w()) - (col[1].w() * col[2].z());
        let cof_t = (col[1].y() * col[2].w()) - (col[1].w() * col[2].y());
        let cof_u = (col[1].y() * col[2].z()) - (col[1].z() * col[2].y());
        let minor03 = col[0].y() * cof_s - col[0].z() * cof_t + col[0].w() * cof_u;
        let cof_v = (col[1].x() * col[2].w()) - (col[1].w() * col[2].x());
        let cof_w = (col[1].x() * col[2].z()) - (col[1].z() * col[2].x());
        let minor13 = col[0].x() * cof_s - col[0].z() * cof_v + col[0].w() * cof_w;
        let cof_x = (col[1].x() * col[2].y()) - (col[1].y() * col[2].x());
        let minor23 = col[0].x() * cof_t - col[0].y() * cof_v + col[0].w() * cof_x;
        let minor33 = col[0].x() * cof_u - col[0].y() * cof_w + col[0].z() * cof_x;

        let inv_det = 1.0 / det;

        // Maybe better to Vec4 * scalar (SIMD) rather then 4 individual *?

        let c0 = Vec4::new(
            minor00 * inv_det,
            -minor01 * inv_det,
            minor02 * inv_det,
            -minor03 * inv_det,
        );
        let c1 = Vec4::new(
            -minor10 * inv_det,
            minor11 * inv_det,
            -minor12 * inv_det,
            minor13 * inv_det,
        );
        let c2 = Vec4::new(
            minor20 * inv_det,
            -minor21 * inv_det,
            minor22 * inv_det,
            -minor23 * inv_det,
        );
        let c3 = Vec4::new(
            -minor30 * inv_det,
            minor31 * inv_det,
            -minor32 * inv_det,
            minor33 * inv_det,
        );

        let m = Mat4x4::from_cols(c0, c1, c2, c3);

        return Some(m);
    }

    /// Inverse of a RIGID transform: rotation + translation only, NO scale/shear.
    /// Bottom row must be [0,0,0,1] and the 3x3 must be orthonormal.
    ///
    /// Uses R⁻¹ = Rᵀ (true ONLY for orthonormal R), so it's far cheaper and more
    /// precise than the general `inverse()` - a transpose plus one mat-vec, no
    /// determinant, no adjugate. If the matrix has scale this is WRONG; use the
    /// general `inverse()` instead.
    pub fn inverse_rigid(self) -> Self {
        let c0 = self.cols[0];
        let c1 = self.cols[1];
        let c2 = self.cols[2];
        let t = self.cols[3]; // translation in xyz

        // Transpose the 3x3: the rows of R become the columns of Rᵀ.
        // (gather each component across the three original columns)
        let r0 = F32x4::new(c0.x(), c1.x(), c2.x(), 0.0);
        let r1 = F32x4::new(c0.y(), c1.y(), c2.y(), 0.0);
        let r2 = F32x4::new(c0.z(), c1.z(), c2.z(), 0.0);

        // New translation = -Rᵀ · t.
        // Component i = -dot(column i of original R, t).
        let tx = -(c0.x() * t.x() + c0.y() * t.y() + c0.z() * t.z());
        let ty = -(c1.x() * t.x() + c1.y() * t.y() + c1.z() * t.z());
        let tz = -(c2.x() * t.x() + c2.y() * t.y() + c2.z() * t.z());

        Self {
            cols: [r0, r1, r2, F32x4::new(tx, ty, tz, 1.0)],
        }
    }
}

// Operations (* /)

impl Mul for Mat4x4 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        let lhs = &self.cols;
        let mul_col = |c: F32x4| -> F32x4 {
            lhs[0] * c.splat_x()
                + lhs[1] * c.splat_y()
                + lhs[2] * c.splat_z()
                + lhs[3] * c.splat_w()
        };
        Self {
            cols: [
                mul_col(rhs.cols[0]),
                mul_col(rhs.cols[1]),
                mul_col(rhs.cols[2]),
                mul_col(rhs.cols[3]),
            ],
        }
    }
}

impl MulAssign for Mat4x4 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Mul<Vec4> for Mat4x4 {
    type Output = Vec4;

    #[inline]
    fn mul(self, v: Vec4) -> Vec4 {
        let v: F32x4 = v.into();
        let r = self.cols[0] * v.splat_x()
            + self.cols[1] * v.splat_y()
            + self.cols[2] * v.splat_z()
            + self.cols[3] * v.splat_w();
        Vec4::from_simd(r)
    }
}

impl Default for Mat4x4 {
    #[inline]
    fn default() -> Self {
        Self::new() // identity, not zero
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_det() {
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 1.0, 6.0, 7.0),
            Vec4::new(8.0, 9.0, 1.0, 10.0),
            Vec4::new(11.0, 12.0, 13.0, 1.0),
        );

        println!("{}", m.transpose().determinant());
    }

    #[test]
    fn det_identity() {
        assert!((Mat4x4::new().determinant() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn det_zero_matrix() {
        assert_eq!(Mat4x4::zero().determinant(), 0.0);
    }

    #[test]
    fn det_uniform_scale() {
        // 4x4 with diag (2, 2, 2, 1) — graphics-style uniform 3D scale, w untouched.
        // det = 2 * 2 * 2 * 1 = 8.
        let m = Mat4x4::from_scale(2.0, 2.0, 2.0);
        assert!((m.determinant() - 8.0).abs() < 1e-5);
    }

    #[test]
    fn det_rotation() {
        // Pure rotation has det = 1.
        let m = Mat4x4::from_rotation_y(0.7);
        assert!((m.determinant() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn det_singular() {
        // Two identical columns -> det = 0.
        let mut m = Mat4x4::new();
        m.cols[1] = m.cols[0];
        assert!(m.determinant().abs() < 1e-5);
    }

    #[test]
    fn det_known_value() {
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 1.0, 6.0, 7.0),
            Vec4::new(8.0, 9.0, 1.0, 10.0),
            Vec4::new(11.0, 12.0, 13.0, 1.0),
        );
        // Non-singular. det = -3042 (hand-expanded along row 0).
        assert!((m.determinant() - (-3042.0)).abs() < 1e-3);
    }

    fn approx_eq(a: &Mat4x4, b: &Mat4x4, eps: f32) -> bool {
        for j in 0..4 {
            let (ac, bc) = (a.cols[j].to_array(), b.cols[j].to_array());
            for i in 0..4 {
                if (ac[i] - bc[i]).abs() > eps {
                    return false;
                }
            }
        }
        true
    }

    // xorshift32 so the differential test is reproducible without pulling in `rand`.
    fn xorshift(s: &mut u32) -> u32 {
        let mut x = *s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *s = x;
        x
    }
    fn rand_f32(s: &mut u32) -> f32 {
        (xorshift(s) as f32 / u32::MAX as f32) * 2.0 - 1.0 // ~[-1,1)
    }
    fn rand_invertible(s: &mut u32) -> Mat4x4 {
        let mut m = Mat4x4::zero();
        for j in 0..4 {
            let mut l = [0.0f32; 4];
            for i in 0..4 {
                l[i] = rand_f32(s);
            }
            l[j] += 4.0; // strong diagonal -> comfortably non-singular, well-conditioned
            m.cols[j] = F32x4::new(l[0], l[1], l[2], l[3]);
        }
        m
    }

    #[test]
    fn fast_identity() {
        assert!(approx_eq(
            &Mat4x4::new().inverse().unwrap(),
            &Mat4x4::new(),
            1e-6
        ));
    }

    #[test]
    fn fast_scale() {
        let m = Mat4x4::from_scale(2.0, 4.0, 8.0);
        let expected = Mat4x4::from_scale(0.5, 0.25, 0.125);
        assert!(approx_eq(&m.inverse().unwrap(), &expected, 1e-6));
    }

    #[test]
    fn fast_singular_returns_none() {
        let mut m = Mat4x4::new();
        m.cols[1] = m.cols[0];
        assert!(m.inverse().is_none());
    }

    #[test]
    fn fast_roundtrip_is_identity() {
        let mut s: u32 = 0x9E37_79B9;
        let id = Mat4x4::new();
        for _ in 0..1000 {
            let m = rand_invertible(&mut s);
            let inv = m.inverse().unwrap();
            assert!(approx_eq(&(m * inv), &id, 1e-3), "M * M^-1 != I");
            assert!(approx_eq(&(inv * m), &id, 1e-3), "M^-1 * M != I");
        }
    }

    #[test]
    fn fast_matches_scalar() {
        let mut s: u32 = 0x1234_5678;
        for _ in 0..1000 {
            let m = rand_invertible(&mut s);
            let scalar = m.inverse_naive().unwrap();
            let fast = m.inverse().unwrap();
            assert!(
                approx_eq(&scalar, &fast, 1e-3),
                "fast disagrees with scalar"
            );
        }
    }

    #[test]
    fn translate_moves_point() {
        let m = Mat4x4::from_translation(Vec3::new(2.0, 3.0, 4.0));
        let p = m.transform_point(Vec3::new(1.0, 1.0, 1.0));
        assert!((p.x() - 3.0).abs() < 1e-6);
        assert!((p.y() - 4.0).abs() < 1e-6);
        assert!((p.z() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn translate_ignores_direction() {
        let m = Mat4x4::from_translation(Vec3::new(2.0, 3.0, 4.0));
        let d = m.transform_vector(Vec3::new(1.0, 0.0, 0.0));
        assert!((d.x() - 1.0).abs() < 1e-6); // unchanged: w=0 drops translation
        assert!(d.y().abs() < 1e-6);
    }

    #[test]
    fn cols_array_is_column_major() {
        // col 3 holds the translation; in a column-major [f32;16] it lands at [12..15].
        let m = Mat4x4::from_translation(Vec3::new(7.0, 8.0, 9.0));
        let a = m.to_cols_array();
        assert_eq!(a[12], 7.0);
        assert_eq!(a[13], 8.0);
        assert_eq!(a[14], 9.0);
        assert_eq!(a[15], 1.0);
    }

    #[test]
    fn look_at_basic() {
        // Camera 5 units down +Z, looking at origin, Y up.
        let v = Mat4x4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        // Origin should land 5 units down -Z (in front of an RH camera).
        let o = v.transform_point(Vec3::new(0.0, 0.0, 0.0));
        assert!(o.x().abs() < 1e-6 && o.y().abs() < 1e-6);
        assert!((o.z() - (-5.0)).abs() < 1e-6);
        // The eye itself should map to the view-space origin.
        let e = v.transform_point(Vec3::new(0.0, 0.0, 5.0));
        assert!(e.x().abs() < 1e-6 && e.y().abs() < 1e-6 && e.z().abs() < 1e-6);
    }

    #[test]
    fn perspective_reverse_z_endpoints() {
        let p = Mat4x4::perspective(90.0_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
        // Helper: project a view-space point and do the perspective divide.
        let ndc_z = |zv: f32| {
            let c = p * Vec4::new(0.0, 0.0, zv, 1.0);
            c.z() / c.w()
        };
        // Reverse-Z: near plane -> 1, far plane -> 0.
        assert!((ndc_z(-0.1) - 1.0).abs() < 1e-4, "near should map to 1");
        assert!(ndc_z(-100.0).abs() < 1e-4, "far should map to 0");
    }

    #[test]
    fn rigid_inverse_matches_general() {
        // rotation + translation, no scale -> rigid
        let m = Mat4x4::from_translation(Vec3::new(3.0, -2.0, 5.0))
            * Mat4x4::from_rotation_y(0.6)
            * Mat4x4::from_rotation_x(-0.3);

        let rigid = m.inverse_rigid();
        let general = m.inverse().unwrap();

        // cheap path agrees with the trusted general path
        assert!(approx_eq(&rigid, &general, 1e-5), "rigid != general");
        // and it actually inverts
        assert!(
            approx_eq(&(m * rigid), &Mat4x4::new(), 1e-5),
            "M * M⁻¹ != I"
        );
    }
}
