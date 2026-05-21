// mat4x4

use crate::simd::{F32x4, shuffle_mask};
use crate::vec4::Vec4;

use std::ops::{Mul, MulAssign};

/// A column major 4x4 Matrix (F32)
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

    pub fn zero() -> Self {
        Self {
            cols: [F32x4::zero(), F32x4::zero(), F32x4::zero(), F32x4::zero()],
        }
    }

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

    pub fn t(self) {
        let col = self.cols;
        let cof_a = (col[2].z() * col[3].w()) - (col[3].z() * col[2].w());
        let cof_b = (col[2].y() * col[3].w()) - (col[3].y() * col[2].w());
        let cof_c = (col[2].y() * col[3].z()) - (col[3].y() * col[2].z());
        let cof_d = (col[2].x() * col[3].w()) - (col[3].x() * col[2].w());
        let cof_e = (col[2].x() * col[3].z()) - (col[3].x() * col[2].z());
        let cof_f = (col[2].x() * col[3].y()) - (col[3].x() * col[2].y());
        let cof_j = (col[2].x() * col[3].w()) - (col[2].w() * col[3].x());
        let cof_k = (col[2].x() * col[3].z()) - (col[2].z() * col[3].x());
        let cof_m = (col[1].z() * col[3].w()) - (col[1].w() * col[3].z());
        let cof_n = (col[1].y() * col[3].w()) - (col[1].w() * col[3].y());
        let cof_o = (col[1].y() * col[3].z()) - (col[1].z() * col[3].y());
        let cof_p = (col[1].x() * col[3].w()) - (col[1].w() * col[3].x());
        let cof_q = (col[1].x() * col[3].z()) - (col[1].z() * col[3].x());
        let cof_r = (col[1].x() * col[3].y()) - (col[1].y() * col[3].x());
        let cof_s = (col[1].z() * col[2].w()) - (col[1].w() * col[2].z());
        let cof_t = (col[1].y() * col[2].w()) - (col[1].w() * col[2].y());
        let cof_u = (col[1].y() * col[2].z()) - (col[1].z() * col[2].y());
        let cof_v = (col[1].x() * col[2].w()) - (col[1].w() * col[2].x());
        let cof_w = (col[1].x() * col[2].z()) - (col[1].z() * col[2].x());
        let cof_x = (col[1].x() * col[2].y()) - (col[1].y() * col[2].x());
    }

    pub fn inverse(self) -> Option<Self> {
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
        // This matrix is non-singular.
        // Determinant is -160 (calculated via standard tools)

        let det = (m.determinant() + 160.0).abs();
        println!("determinant is: {}", det);
        assert!(det < 1e-3);
    }

    //     fn approx_eq(a: &Mat4x4, b: &Mat4x4, eps: f32) -> bool {
    //         for j in 0..4 {
    //             let ac = a.cols[j].to_array();
    //             let bc = b.cols[j].to_array();
    //             for i in 0..4 {
    //                 if (ac[i] - bc[i]).abs() > eps {
    //                     return false;
    //                 }
    //             }
    //         }
    //         true
    //     }

    //     #[test]
    //     fn identity_inverse() {
    //         let i = Mat4x4::new();
    //         assert!(approx_eq(&i.inverse(), &i, 1e-6));
    //     }

    //     #[test]
    //     fn round_trip() {
    //         // An arbitrary non-trivial invertible matrix.
    //         let m = Mat4x4 {
    //             cols: [
    //                 F32x4::new(1.0, 2.0, 3.0, 4.0),
    //                 F32x4::new(0.0, 1.0, 4.0, 5.0),
    //                 F32x4::new(6.0, 0.0, 1.0, 7.0),
    //                 F32x4::new(8.0, 9.0, 0.0, 1.0),
    //             ],
    //         };
    //         let inv = m.inverse();
    //         let back = m * inv;
    //         assert!(approx_eq(&back, &Mat4x4::new(), 1e-4));

    //         let back2 = inv * m;
    //         assert!(approx_eq(&back2, &Mat4x4::new(), 1e-4));
    //     }

    //     #[test]
    //     fn scale_inverse() {
    //         // diag(2, 3, 5, 1) -> diag(0.5, 1/3, 0.2, 1)
    //         let m = Mat4x4 {
    //             cols: [
    //                 F32x4::new(2.0, 0.0, 0.0, 0.0),
    //                 F32x4::new(0.0, 3.0, 0.0, 0.0),
    //                 F32x4::new(0.0, 0.0, 5.0, 0.0),
    //                 F32x4::new(0.0, 0.0, 0.0, 1.0),
    //             ],
    //         };
    //         let expected = Mat4x4 {
    //             cols: [
    //                 F32x4::new(0.5, 0.0, 0.0, 0.0),
    //                 F32x4::new(0.0, 1.0 / 3.0, 0.0, 0.0),
    //                 F32x4::new(0.0, 0.0, 0.2, 0.0),
    //                 F32x4::new(0.0, 0.0, 0.0, 1.0),
    //             ],
    //         };
    //         assert!(approx_eq(&m.inverse(), &expected, 1e-6));
    //     }
}
