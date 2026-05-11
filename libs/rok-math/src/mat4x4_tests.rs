// mat4x4 tests

use crate::mat4x4::Mat4x4;
use crate::vec4::Vec4;

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- helpers ----------

    fn approx_eq(a: &Mat4x4, b: &Mat4x4, eps: f32) -> bool {
        for j in 0..4 {
            for i in 0..4 {
                let av = match i {
                    0 => a.cols[j].x(),
                    1 => a.cols[j].y(),
                    2 => a.cols[j].z(),
                    _ => a.cols[j].w(),
                };
                let bv = match i {
                    0 => b.cols[j].x(),
                    1 => b.cols[j].y(),
                    2 => b.cols[j].z(),
                    _ => b.cols[j].w(),
                };
                if (av - bv).abs() > eps {
                    return false;
                }
            }
        }
        true
    }

    /// Pretty-print a Mat4x4 row-by-row when an assertion fails.
    fn dump(label: &str, m: &Mat4x4) {
        println!("{}:", label);
        for i in 0..4 {
            let row = (0..4)
                .map(|j| match i {
                    0 => m.cols[j].x(),
                    1 => m.cols[j].y(),
                    2 => m.cols[j].z(),
                    _ => m.cols[j].w(),
                })
                .collect::<Vec<_>>();
            println!(
                "  [{:>10.4} {:>10.4} {:>10.4} {:>10.4}]",
                row[0], row[1], row[2], row[3]
            );
        }
    }

    fn assert_mat_eq(a: &Mat4x4, b: &Mat4x4, eps: f32) {
        if !approx_eq(a, b, eps) {
            dump("got", a);
            dump("expected", b);
            panic!("matrices differ by more than {}", eps);
        }
    }

    // ---------- determinant ----------

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
        // diag(2,2,2,1) → det = 8.
        let m = Mat4x4::from_scale(2.0, 2.0, 2.0);
        assert!((m.determinant() - 8.0).abs() < 1e-5);
    }

    #[test]
    fn det_non_uniform_scale() {
        // diag(2,3,5,1) → det = 30.
        let m = Mat4x4::from_scale(2.0, 3.0, 5.0);
        assert!((m.determinant() - 30.0).abs() < 1e-5);
    }

    #[test]
    fn det_rotation_is_one() {
        // Rotation preserves volume.
        for angle in [0.1f32, 0.7, 1.3, 2.5] {
            assert!((Mat4x4::from_rotation_x(angle).determinant() - 1.0).abs() < 1e-5);
            assert!((Mat4x4::from_rotation_y(angle).determinant() - 1.0).abs() < 1e-5);
            assert!((Mat4x4::from_rotation_z(angle).determinant() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn det_singular_duplicate_col() {
        let mut m = Mat4x4::new();
        m.cols[1] = m.cols[0];
        assert!(m.determinant().abs() < 1e-5);
    }

    #[test]
    fn det_known_value() {
        // Columns in column-major order. Verified against numpy: det = -3042.
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 1.0, 6.0, 7.0),
            Vec4::new(8.0, 9.0, 1.0, 10.0),
            Vec4::new(11.0, 12.0, 13.0, 1.0),
        );
        let d = m.determinant();
        assert!((d - (-3042.0)).abs() < 1e-2, "got det = {}", d);
    }

    #[test]
    fn det_transpose_equals_original() {
        // det(M) == det(M^T) for any square M.
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 1.0, 6.0, 7.0),
            Vec4::new(8.0, 9.0, 1.0, 10.0),
            Vec4::new(11.0, 12.0, 13.0, 1.0),
        );
        let d1 = m.determinant();
        let d2 = m.transpose().determinant();
        assert!((d1 - d2).abs() < 1e-2, "{} vs {}", d1, d2);
    }

    // ---------- multiplication ----------

    #[test]
    fn mul_identity_left() {
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let r = Mat4x4::new() * m;
        assert_mat_eq(&r, &m, 1e-5);
    }

    #[test]
    fn mul_identity_right() {
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let r = m * Mat4x4::new();
        assert_mat_eq(&r, &m, 1e-5);
    }

    #[test]
    fn mul_zero() {
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let r = m * Mat4x4::zero();
        assert_mat_eq(&r, &Mat4x4::zero(), 1e-5);
        let r = Mat4x4::zero() * m;
        assert_mat_eq(&r, &Mat4x4::zero(), 1e-5);
    }

    #[test]
    fn mul_scale_composition() {
        // diag(2,3,5,1) * diag(7,11,13,1) = diag(14,33,65,1)
        let a = Mat4x4::from_scale(2.0, 3.0, 5.0);
        let b = Mat4x4::from_scale(7.0, 11.0, 13.0);
        let expected = Mat4x4::from_scale(14.0, 33.0, 65.0);
        assert_mat_eq(&(a * b), &expected, 1e-5);
    }

    #[test]
    fn mul_rotation_inverse_is_identity() {
        // Rotating by θ then by −θ around the same axis = identity.
        let a = Mat4x4::from_rotation_y(0.9);
        let b = Mat4x4::from_rotation_y(-0.9);
        assert_mat_eq(&(a * b), &Mat4x4::new(), 1e-5);
        assert_mat_eq(&(b * a), &Mat4x4::new(), 1e-5);
    }

    #[test]
    fn mul_non_commutative() {
        // Specific case where A*B != B*A, to catch ordering bugs.
        let a = Mat4x4::from_rotation_x(0.7);
        let b = Mat4x4::from_rotation_y(0.5);
        assert!(
            !approx_eq(&(a * b), &(b * a), 1e-4),
            "expected rotations on different axes to not commute"
        );
    }

    #[test]
    fn mul_known_value() {
        // Verified against numpy. Columns are listed column-major.
        let a = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let b = Mat4x4::from_cols(
            Vec4::new(17.0, 18.0, 19.0, 20.0),
            Vec4::new(21.0, 22.0, 23.0, 24.0),
            Vec4::new(25.0, 26.0, 27.0, 28.0),
            Vec4::new(29.0, 30.0, 31.0, 32.0),
        );
        let expected = Mat4x4::from_cols(
            Vec4::new(538.0, 612.0, 686.0, 760.0),
            Vec4::new(650.0, 740.0, 830.0, 920.0),
            Vec4::new(762.0, 868.0, 974.0, 1080.0),
            Vec4::new(874.0, 996.0, 1118.0, 1240.0),
        );
        assert_mat_eq(&(a * b), &expected, 1e-3);
    }

    // ---------- inverse ----------

    #[test]
    fn inv_identity() {
        let i = Mat4x4::new();
        let inv = i.inverse().expect("identity should be invertible");
        assert_mat_eq(&inv, &i, 1e-5);
    }

    #[test]
    fn inv_scale() {
        // diag(2, 4, 8, 1) -> diag(0.5, 0.25, 0.125, 1)
        let m = Mat4x4::from_scale(2.0, 4.0, 8.0);
        let expected = Mat4x4::from_scale(0.5, 0.25, 0.125);
        let inv = m.inverse().expect("non-singular");
        assert_mat_eq(&inv, &expected, 1e-5);
    }

    #[test]
    fn inv_rotation_is_transpose() {
        // For any pure rotation R, R^-1 == R^T.
        let r = Mat4x4::from_rotation_y(0.7);
        let inv = r.inverse().expect("rotation is invertible");
        assert_mat_eq(&inv, &r.transpose(), 1e-5);
    }

    #[test]
    fn inv_round_trip_identity_left() {
        // M * M^-1 == I
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(0.0, 1.0, 4.0, 5.0),
            Vec4::new(6.0, 0.0, 1.0, 7.0),
            Vec4::new(8.0, 9.0, 0.0, 1.0),
        );
        let inv = m.inverse().expect("non-singular");
        assert_mat_eq(&(m * inv), &Mat4x4::new(), 1e-4);
    }

    #[test]
    fn inv_round_trip_identity_right() {
        // M^-1 * M == I (catches transpose-bug failure mode).
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(0.0, 1.0, 4.0, 5.0),
            Vec4::new(6.0, 0.0, 1.0, 7.0),
            Vec4::new(8.0, 9.0, 0.0, 1.0),
        );
        let inv = m.inverse().expect("non-singular");
        assert_mat_eq(&(inv * m), &Mat4x4::new(), 1e-4);
    }

    #[test]
    fn inv_known_value() {
        // M and its inverse verified against numpy. det(M) = -48.
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(0.0, 1.0, 4.0, 5.0),
            Vec4::new(6.0, 0.0, 1.0, 7.0),
            Vec4::new(8.0, 9.0, 0.0, 1.0),
        );
        let expected = Mat4x4::from_cols(
            Vec4::new(-4.333_333_3, 3.229_166_7, 0.083_333_3, 0.604_166_7),
            Vec4::new(3.333_333_3, -2.479_166_7, -0.083_333_3, -0.354_166_7),
            Vec4::new(-6.666_666_7, 5.270_833_3, -0.083_333_3, 0.895_833_3),
            Vec4::new(4.666_666_7, -3.520_833_3, 0.083_333_3, -0.645_833_3),
        );
        let inv = m.inverse().expect("non-singular");
        assert_mat_eq(&inv, &expected, 1e-4);
    }

    #[test]
    fn inv_affine_composition() {
        // T * R * S round-trips through inverse.
        let t = Mat4x4::from_cols(
            Vec4::new(1.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(3.0, -2.0, 5.0, 1.0),
        );
        let r = Mat4x4::from_rotation_y(0.7);
        let s = Mat4x4::from_scale(2.0, 3.0, 0.5);
        let m = t * r * s;
        let inv = m.inverse().expect("non-singular");
        assert_mat_eq(&(m * inv), &Mat4x4::new(), 1e-4);
        assert_mat_eq(&(inv * m), &Mat4x4::new(), 1e-4);
    }

    #[test]
    fn inv_singular_returns_none() {
        // Two identical columns -> singular.
        let mut m = Mat4x4::new();
        m.cols[1] = m.cols[0];
        assert!(m.inverse().is_none());

        // Zero matrix -> singular.
        assert!(Mat4x4::zero().inverse().is_none());

        // Zero row (third row all zero) -> singular.
        let m = Mat4x4::from_cols(
            Vec4::new(1.0, 2.0, 0.0, 4.0),
            Vec4::new(5.0, 6.0, 0.0, 8.0),
            Vec4::new(9.0, 10.0, 0.0, 12.0),
            Vec4::new(13.0, 14.0, 0.0, 16.0),
        );
        assert!(m.inverse().is_none());
    }
}
