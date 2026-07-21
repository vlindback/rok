// quaternion.rs
//
// rok-math library
//

//! Quaternion: unit quaternions representing 3D rotations.
//!
//! Layout: (x, y, z, w) - vector part in (x, y, z), scalar part in w.
//!   Matches Vec4 component order; identity is (0, 0, 0, 1).
//!
//! Convention: active, right-handed rotations (consistent with rok-math's
//!   right-handed view space). A unit quaternion q rotates a vector v by
//!       v' = q * v * q⁻¹.
//!
//! Composition: `a * b` means "apply b first, then a" - identical to
//!   Mat4x4 column-vector composition (M = A*B applies B then A). So
//!       (a * b).rotate_vec3(v) == a.rotate_vec3(b.rotate_vec3(v)).
//!   Uses the Hamilton product (NOT JPL).
//!
//! Euler: from_euler uses YXZ order (yaw about Y, then pitch about X,
//!   then roll about Z) - see from_euler for the exact composition.
//!
//! Drift: composing many rotations accumulates float error; renormalize
//!   periodically via normalize / normalized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

use crate::{mat4x4::Mat4x4, vec3::Vec3};
use std::ops::Mul;

impl Quat {
    /// The identity rotation (no rotation).
    pub const fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Construct directly from components. Does NOT normalize - callers needing
    /// a unit quaternion should build via from_axis_angle / from_euler, or call
    /// .normalized().
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Construct a rotation from Euler angles using INTRINSIC YXZ order:
    /// yaw about world Y, THEN pitch about the (yawed) local X, THEN roll about
    /// the (yawed+pitched) local Z. This is the conventional FPS-camera order -
    /// yaw keeps the horizon level, pitch is relative to current heading.
    ///
    /// Why this product order: per the file header, `a * b` applies b first and
    /// maps to matrix product A*B. Intrinsic Y->X->Z equals the matrix product
    /// M_Y * M_X * M_Z, so the quaternion is qY * qX * qZ in that same order.
    /// (Equivalently, intrinsic YXZ == extrinsic ZXY.)
    ///
    /// All angles in radians. NOTE: Euler angles suffer gimbal lock at pitch =
    /// +-90 deg; this is a property of the representation, not a bug. Store and
    /// interpolate as quaternions (see slerp) to avoid it.
    pub fn from_euler(yaw: f32, pitch: f32, roll: f32) -> Self {
        let qy = Quat::from_axis_angle(Vec3::unit_y(), yaw);
        let qx = Quat::from_axis_angle(Vec3::unit_x(), pitch);
        let qz = Quat::from_axis_angle(Vec3::unit_z(), roll);
        qy * qx * qz
    }

    pub fn from_array(a: &[f32; 4]) -> Self {
        Self {
            x: a[0],
            y: a[1],
            z: a[2],
            w: a[3],
        }
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
    pub fn w(&self) -> f32 {
        self.w
    }

    /// 4D dot product (treats both as Vec4s). Used by length and, later, slerp.
    #[inline]
    pub fn dot(self, other: Quat) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    #[inline]
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a unit quaternion. Falls back to identity on (near-)zero length.
    pub fn normalized(self) -> Self {
        let len_sq = self.length_squared();
        let eps_tolerance = 1e-8;
        if len_sq > eps_tolerance {
            let inv_len = 1.0 / len_sq.sqrt();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
                w: self.w * inv_len,
            }
        } else {
            Self::identity()
        }
    }

    /// Normalizes in place to a unit quaternion (identity on near-zero length).
    pub fn normalize(&mut self) {
        *self = self.normalized();
    }

    /// The conjugate q* = (-x, -y, -z, w): negate the vector part, keep scalar.
    /// For a UNIT quaternion this equals the inverse - see `inverse`.
    #[inline]
    pub fn conjugate(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: self.w,
        }
    }

    /// The multiplicative inverse q⁻¹ = q* / |q|².
    ///
    /// Derivation: a quaternion times its conjugate is real and equals the
    /// squared norm,
    ///     q q* = w² + x² + y² + z² = |q|²,
    /// so q (q*/|q|²) = 1, giving q⁻¹ = q*/|q|². For a unit quaternion |q|² = 1,
    /// hence q⁻¹ = q* exactly - which is why rotate_vec3 will use the cheaper
    /// conjugate rather than a full inverse.
    pub fn inverse(self) -> Self {
        let len_sq = self.length_squared();
        let eps_tolerance = 1e-8;
        if len_sq > eps_tolerance {
            let inv = 1.0 / len_sq;
            Self {
                x: -self.x * inv,
                y: -self.y * inv,
                z: -self.z * inv,
                w: self.w * inv,
            }
        } else {
            Self::identity()
        }
    }

    /// Construct a unit quaternion representing a rotation of `angle` radians
    /// about `axis` (right-handed). The axis is normalized internally.
    ///
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let n = axis.normalized();
        // Vec3::normalized returns the zero vector for a zero-length axis, which
        // would yield a non-unit quat (0,0,0,cos(θ/2)). A zero axis isn't a valid
        // rotation, so fall back to identity (consistent with normalized/inverse).
        // The == is exact here: normalized() writes literal 0.0 in its fallback.
        if n == Vec3::zero() {
            return Self::identity();
        }
        let half = angle * 0.5;
        let (s, c) = half.sin_cos();
        Self {
            x: n.x() * s,
            y: n.y() * s,
            z: n.z() * s,
            w: c,
        }
    }

    /// Rotate a vector by this quaternion (active, right-handed): v' = q v q⁻¹.
    ///
    /// Assumes a UNIT quaternion (uses the conjugate, not a full inverse).
    ///
    #[inline]
    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let u = Vec3::new(self.x, self.y, self.z);
        let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }

    /// Spherical linear interpolation: constant-angular-velocity interpolation
    /// along the shortest arc between two unit rotations. The reason to store
    /// orientation as quaternions - smooth, gimbal-free blending for animation
    /// and camera smoothing.
    ///
    /// Derivation: unit quats lie on the unit 3-sphere; the shortest path is the
    /// great-circle arc, and slerp is its constant-speed parameterization:
    ///   slerp = sin((1-t)O)/sin(O) * q0 + sin(tO)/sin(O) * q1,  cos(O) = q0.q1.
    /// The sin ratios advance the angle from q0 linearly as t*O -> constant speed.
    ///
    /// Guards:
    ///  - double cover: q and -q are the same rotation, so if the dot is negative
    ///    we negate one endpoint, else the arc takes the long way (>180 deg).
    ///  - near-parallel: as O->0, sin(O)->0 and the ratios blow up; fall back to
    ///    nlerp, which is numerically stable and visually identical at small O.
    ///
    /// Assumes both inputs are unit quaternions. t is not clamped: t in [0,1]
    /// interpolates; outside that it extrapolates along the same arc.
    pub fn slerp(self, other: Quat, t: f32) -> Quat {
        let mut other = other;
        let mut d = self.dot(other);

        // Double-cover / shortest-path fix.
        if d < 0.0 {
            other = Quat {
                x: -other.x,
                y: -other.y,
                z: -other.z,
                w: -other.w,
            };
            d = -d;
        }

        const DOT_THRESHOLD: f32 = 0.9995;
        if d > DOT_THRESHOLD {
            // Nearly parallel: nlerp (lerp + renormalize) avoids the sin(O)->0 blowup.
            let r = Quat {
                x: self.x + (other.x - self.x) * t,
                y: self.y + (other.y - self.y) * t,
                z: self.z + (other.z - self.z) * t,
                w: self.w + (other.w - self.w) * t,
            };
            return r.normalized();
        }

        let omega = d.acos(); // angle between the two quaternions
        let sin_omega = omega.sin();
        let a = ((1.0 - t) * omega).sin() / sin_omega;
        let b = (t * omega).sin() / sin_omega;

        Quat {
            x: a * self.x + b * other.x,
            y: a * self.y + b * other.y,
            z: a * self.z + b * other.z,
            w: a * self.w + b * other.w,
        }
    }

    pub fn to_mat4x4(&self) -> Mat4x4 {
        let (x, y, z, w) = (self.x, self.y, self.z, self.w);

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

        Mat4x4::from_cols_array(
            1.0 - (yy + zz),
            xy + wz,
            xz - wy,
            0.0,
            xy - wz,
            1.0 - (xx + zz),
            yz + wx,
            0.0,
            xz + wy,
            yz - wx,
            1.0 - (xx + yy),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }
}

// Operations

impl Mul for Quat {
    type Output = Quat;

    #[inline]
    fn mul(self, rhs: Quat) -> Quat {
        let (a, b) = (self, rhs);
        // Hamilton product (composition of rotations).
        Quat {
            w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
            x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            y: a.w * b.y + a.y * b.w + a.z * b.x - a.x * b.z,
            z: a.w * b.z + a.z * b.w + a.x * b.y - a.y * b.x,
        }
    }
}

impl Default for Quat {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl From<[f32; 4]> for Quat {
    #[inline]
    fn from(comps: [f32; 4]) -> Self {
        Self {
            x: comps[0],
            y: comps[1],
            z: comps[2],
            w: comps[3],
        }
    }
}

// Tests:

#[cfg(test)]
mod tests {
    use crate::mat4x4::Mat4x4;

    use super::*;

    const EPS: f32 = 1e-6;
    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn identity_is_unit() {
        assert!(approx(Quat::identity().length(), 1.0));
    }

    #[test]
    fn conjugate_is_an_involution() {
        let q = Quat::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(q.conjugate().conjugate(), q);
    }

    #[test]
    fn unit_inverse_equals_conjugate() {
        let q = Quat::new(0.1, 0.2, 0.3, 0.4).normalized();
        let (inv, con) = (q.inverse(), q.conjugate());
        assert!(
            approx(inv.x, con.x)
                && approx(inv.y, con.y)
                && approx(inv.z, con.z)
                && approx(inv.w, con.w)
        );
    }

    #[test]
    fn normalized_has_unit_length() {
        let q = Quat::new(1.0, 2.0, 3.0, 4.0).normalized();
        assert!(approx(q.length(), 1.0));
    }

    #[test]
    fn axis_angle_is_unit() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 1.234);
        assert!(approx(q.length(), 1.0));
    }

    #[test]
    fn axis_angle_zero_angle_is_identity() {
        let q = Quat::from_axis_angle(Vec3::unit_y(), 0.0);
        assert_eq!(q, Quat::identity());
    }

    #[test]
    fn axis_angle_zero_axis_is_identity() {
        let q = Quat::from_axis_angle(Vec3::zero(), 1.0);
        assert_eq!(q, Quat::identity());
    }

    #[test]
    fn axis_angle_90deg_z() {
        // θ = π/2 → half = π/4 → sin = cos = √2/2; axis Z.
        let q = Quat::from_axis_angle(Vec3::unit_z(), std::f32::consts::FRAC_PI_2);
        let r = std::f32::consts::FRAC_1_SQRT_2;
        assert!(approx(q.x, 0.0) && approx(q.y, 0.0) && approx(q.z, r) && approx(q.w, r));
    }

    #[test]
    fn axis_angle_180deg_x() {
        // θ = π → half = π/2 → sin = 1, cos = 0; axis X. q = (1,0,0,0).
        let q = Quat::from_axis_angle(Vec3::unit_x(), std::f32::consts::PI);
        assert!(approx(q.x, 1.0) && approx(q.y, 0.0) && approx(q.z, 0.0) && approx(q.w, 0.0));
    }

    #[test]
    fn mul_identity_is_neutral() {
        let q = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7);
        let i = Quat::identity();
        let qi = q * i;
        let iq = i * q;
        assert!(approx(qi.x, q.x) && approx(qi.y, q.y) && approx(qi.z, q.z) && approx(qi.w, q.w));
        assert!(approx(iq.x, q.x) && approx(iq.y, q.y) && approx(iq.z, q.z) && approx(iq.w, q.w));
    }

    #[test]
    fn mul_by_inverse_is_identity() {
        let q = Quat::from_axis_angle(Vec3::new(0.3, -1.0, 0.5), 1.1);
        let r = q * q.inverse();
        let i = Quat::identity();
        assert!(approx(r.x, i.x) && approx(r.y, i.y) && approx(r.z, i.z) && approx(r.w, i.w));
    }

    #[test]
    fn mul_is_not_commutative() {
        // Two rotations about different axes shouldn't commute. Guards against a
        // sign slip in the cross-product term that would symmetrize the product.
        let a = Quat::from_axis_angle(Vec3::unit_x(), 0.9);
        let b = Quat::from_axis_angle(Vec3::unit_y(), 0.6);
        let ab = a * b;
        let ba = b * a;
        assert!(
            !(approx(ab.x, ba.x) && approx(ab.y, ba.y) && approx(ab.z, ba.z) && approx(ab.w, ba.w))
        );
    }

    #[test]
    fn mul_preserves_unit_length() {
        // Product of two unit quats is unit (up to float error) composition
        // shouldn't introduce gross drift in a single step.
        let a = Quat::from_axis_angle(Vec3::unit_z(), 1.3);
        let b = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 0.4);
        assert!(approx((a * b).length(), 1.0));
    }

    /// Independent Rodrigues oracle for the OFF-AXIS case: rotation of v about a
    /// unit axis k by angle, derived straight from the vector triple-product
    /// geometry. No quaternion algebra anywhere in here.
    fn rodrigues(axis: Vec3, angle: f32, v: Vec3) -> Vec3 {
        let k = axis.normalized();
        let (s, c) = angle.sin_cos();
        v * c + k.cross(v) * s + k * (k.dot(v) * (1.0 - c))
    }

    fn vapprox(a: Vec3, b: Vec3) -> bool {
        (a.x() - b.x()).abs() < 1e-5 && (a.y() - b.y()).abs() < 1e-5 && (a.z() - b.z()).abs() < 1e-5
    }

    #[test]
    fn rotate_known_value_90z() {
        // 90° about +Z sends +X to +Y (right-handed).
        let q = Quat::from_axis_angle(Vec3::unit_z(), std::f32::consts::FRAC_PI_2);
        assert!(vapprox(q.rotate_vec3(Vec3::unit_x()), Vec3::unit_y()));
    }

    #[test]
    fn rotate_identity_is_noop() {
        let v = Vec3::new(1.0, -2.0, 3.0);
        assert!(vapprox(Quat::identity().rotate_vec3(v), v));
    }

    #[test]
    fn rotate_matches_matrix_axis_aligned() {
        // Against from_rotation_{x,y,z}. Each v carries a component ALONG the axis
        // so the (n̂·v)n̂ parallel term is non-zero and actually exercised.
        let v = Vec3::new(1.0, 1.0, 1.0);
        let cases = [
            (Vec3::unit_x(), Mat4x4::from_rotation_x(0.73), 0.73),
            (Vec3::unit_y(), Mat4x4::from_rotation_y(1.21), 1.21),
            (Vec3::unit_z(), Mat4x4::from_rotation_z(2.40), 2.40),
        ];
        for (axis, m, angle) in cases {
            let q = Quat::from_axis_angle(axis, angle);
            // NOTE: assumes Mat4x4::transform_vector(Vec3) -> Vec3 (w=0, ignores
            // translation). Tell me if it's named differently and I'll adjust.
            assert!(vapprox(q.rotate_vec3(v), m.transform_vector(v)));
        }
    }

    #[test]
    fn rotate_matches_rodrigues_arbitrary_axis() {
        // The off-axis case the single-axis matrices can't reach.
        let axis = Vec3::new(1.0, -2.0, 0.5);
        let v = Vec3::new(0.3, 0.9, -1.4);
        for &angle in &[0.2_f32, 1.0, 2.5, -1.1] {
            let q = Quat::from_axis_angle(axis, angle);
            assert!(vapprox(q.rotate_vec3(v), rodrigues(axis, angle, v)));
        }
    }

    #[test]
    fn rotate_composition_matches_convention() {
        // End-to-end check of the header convention: (a*b) applies b THEN a.
        let a = Quat::from_axis_angle(Vec3::unit_y(), 0.6);
        let b = Quat::from_axis_angle(Vec3::unit_x(), 1.1);
        let v = Vec3::new(1.0, 2.0, 3.0);
        let composed = (a * b).rotate_vec3(v);
        let sequential = a.rotate_vec3(b.rotate_vec3(v));
        assert!(vapprox(composed, sequential));
    }

    #[test]
    fn double_cover_rotation_is_identical() {
        let q1 = Quat::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 1.0);
        let q2 = Quat::new(-q1.x(), -q1.y(), -q1.z(), -q1.w()); // Negated q

        let v = Vec3::new(0.5, -1.2, 3.3);
        assert!(vapprox(q1.rotate_vec3(v), q2.rotate_vec3(v)));
    }

    #[test]
    fn euler_matches_sequential_matrix_rotation() {
        // q = qY*qX*qZ means q.rotate(v) == M_Y(M_X(M_Z(v))). Validates against the
        // independent hand-written single-axis matrices, not against itself.
        let (yaw, pitch, roll) = (0.6_f32, -0.4, 1.1);
        let q = Quat::from_euler(yaw, pitch, roll);
        let v = Vec3::new(1.0, 2.0, 3.0);

        let my = Mat4x4::from_rotation_y(yaw);
        let mx = Mat4x4::from_rotation_x(pitch);
        let mz = Mat4x4::from_rotation_z(roll);
        let expected = my.transform_vector(mx.transform_vector(mz.transform_vector(v)));

        assert!(vapprox(q.rotate_vec3(v), expected));
    }

    #[test]
    fn euler_zero_is_identity() {
        assert!(approx(Quat::from_euler(0.0, 0.0, 0.0).length(), 1.0));
        let q = Quat::from_euler(0.0, 0.0, 0.0);
        assert!(approx(q.x, 0.0) && approx(q.y, 0.0) && approx(q.z, 0.0) && approx(q.w, 1.0));
    }

    #[test]
    fn euler_pure_yaw_is_y_axis_rotation() {
        // Only yaw set -> must equal a plain Y-axis quat.
        let q = Quat::from_euler(0.7, 0.0, 0.0);
        let expected = Quat::from_axis_angle(Vec3::unit_y(), 0.7);
        assert!(
            approx(q.x, expected.x)
                && approx(q.y, expected.y)
                && approx(q.z, expected.z)
                && approx(q.w, expected.w)
        );
    }

    #[test]
    fn slerp_endpoints() {
        let q0 = Quat::from_axis_angle(Vec3::unit_x(), 0.3);
        let q1 = Quat::from_axis_angle(Vec3::new(1.0, 1.0, 0.0), 1.2);
        let s0 = q0.slerp(q1, 0.0);
        let s1 = q0.slerp(q1, 1.0);
        assert!(
            approx(s0.x, q0.x) && approx(s0.y, q0.y) && approx(s0.z, q0.z) && approx(s0.w, q0.w)
        );
        assert!(
            approx(s1.x, q1.x) && approx(s1.y, q1.y) && approx(s1.z, q1.z) && approx(s1.w, q1.w)
        );
    }

    #[test]
    fn slerp_stays_unit() {
        let q0 = Quat::from_axis_angle(Vec3::unit_y(), 0.2);
        let q1 = Quat::from_axis_angle(Vec3::unit_z(), 2.0);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!(approx(q0.slerp(q1, t).length(), 1.0));
        }
    }

    #[test]
    fn slerp_constant_velocity() {
        // identity -> 90deg about Z: slerp(t) must equal rotation by t*90deg.
        // This is the defining property - uniform angular steps, not just endpoints.
        let q0 = Quat::identity();
        let q1 = Quat::from_axis_angle(Vec3::unit_z(), std::f32::consts::FRAC_PI_2);
        for &t in &[0.25_f32, 0.5, 0.75] {
            let got = q0.slerp(q1, t);
            let expected = Quat::from_axis_angle(Vec3::unit_z(), t * std::f32::consts::FRAC_PI_2);
            assert!(
                approx(got.x, expected.x)
                    && approx(got.y, expected.y)
                    && approx(got.z, expected.z)
                    && approx(got.w, expected.w)
            );
        }
    }

    #[test]
    fn slerp_takes_shortest_path() {
        // q1 and -q1 are the same rotation; slerp to either must give the same
        // result (the double-cover fix picks the short arc both times).
        let q0 = Quat::from_axis_angle(Vec3::unit_x(), 0.4);
        let q1 = Quat::from_axis_angle(Vec3::unit_x(), 0.9);
        let neg = Quat {
            x: -q1.x,
            y: -q1.y,
            z: -q1.z,
            w: -q1.w,
        };
        let a = q0.slerp(q1, 0.5);
        let b = q0.slerp(neg, 0.5);
        assert!(approx(a.x, b.x) && approx(a.y, b.y) && approx(a.z, b.z) && approx(a.w, b.w));
    }
}
