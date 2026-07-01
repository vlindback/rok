// simd.rs

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::vec4::Vec4;

// _MM_SHUFFLE replacement
#[inline(always)]
pub const fn shuffle_mask(z: u32, y: u32, x: u32, w: u32) -> i32 {
    ((z << 6) | (y << 4) | (x << 2) | w) as i32
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct F32x4(__m128);

impl F32x4 {
    /// Broadcast a single scalar into all 4 lanes.
    #[inline(always)]
    pub fn splat(v: f32) -> Self {
        Self(unsafe { _mm_set1_ps(v) })
    }

    /// All lanes zero.
    #[inline(always)]
    pub fn zero() -> Self {
        Self(unsafe { _mm_setzero_ps() })
    }

    /// Set lanes individually. Memory order: `x` -> lane 0, `w` -> lane 3.
    #[inline(always)]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self(unsafe { _mm_setr_ps(x, y, z, w) })
    }

    /// Wrap a raw `__m128` register.
    #[inline(always)]
    pub fn from_raw(v: __m128) -> Self {
        Self(v)
    }

    /// Unwrap to the raw `__m128` register.
    #[inline(always)]
    pub fn raw(self) -> __m128 {
        self.0
    }

    // Load & Store:

    /// Unaligned load (default, safe, slightly slower on ancient hardware).
    #[inline(always)]
    pub fn load(ptr: &[f32; 4]) -> Self {
        Self(unsafe { _mm_loadu_ps(ptr.as_ptr()) })
    }

    /// Aligned load.  The pointer **must** be 16-byte aligned.
    ///
    /// In debug builds this is asserted; in release it's UB if violated)
    #[inline(always)]
    pub fn load_aligned(ptr: &[f32; 4]) -> Self {
        debug_assert!(
            ptr.as_ptr() as usize % 16 == 0,
            "F32x4::load_aligned: pointer is not 16-byte aligned"
        );
        Self(unsafe { _mm_load_ps(ptr.as_ptr()) })
    }

    /// Unaligned store.
    #[inline(always)]
    pub fn store(self, dst: &mut [f32; 4]) {
        unsafe { _mm_storeu_ps(dst.as_mut_ptr(), self.0) }
    }

    /// Aligned store. The pointer **must** be 16-byte aligned.
    ///
    /// In debug builds this is asserted; in release it's UB if violated)
    #[inline(always)]
    pub fn store_aligned(self, dst: &mut [f32; 4]) {
        debug_assert!(
            dst.as_ptr() as usize % 16 == 0,
            "F32x4::store_aligned: pointer is not 16-byte aligned"
        );
        unsafe { _mm_store_ps(dst.as_mut_ptr(), self.0) }
    }

    /// Insert a scalar via `insertps`. `IMM` is the `_mm_insert_ps` encoding:
    /// src-lane [7:6], dst-lane [5:4], zmask [3:0]. Scalar is placed in lane 0
    /// (src `00`), so the dst nibble selects the target:
    ///   0x00 -> lane 0, 0x10 -> lane 1, 0x20 -> lane 2, 0x30 -> lane 3.
    #[inline(always)]
    pub fn insert<const IMM: i32>(self, scalar: f32) -> Self {
        Self(unsafe { _mm_insert_ps::<IMM>(self.0, _mm_set_ss(scalar)) })
    }

    // Ahritmetic

    #[inline(always)]
    pub fn add(self, rhs: Self) -> Self {
        Self(unsafe { _mm_add_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn sub(self, rhs: Self) -> Self {
        Self(unsafe { _mm_sub_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn mul(self, rhs: Self) -> Self {
        Self(unsafe { _mm_mul_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn div(self, rhs: Self) -> Self {
        Self(unsafe { _mm_div_ps(self.0, rhs.0) })
    }

    /// Fused multiply-add: `(self * a) + b`  (FMA3, single rounding).
    pub fn mul_add(self, a: Self, b: Self) -> Self {
        Self(unsafe { _mm_fmadd_ps(self.0, a.0, b.0) })
    }

    /// Fused multiply-sub: `(self * a) - b`.
    pub fn mul_sub(self, a: Self, b: Self) -> Self {
        Self(unsafe { _mm_fmsub_ps(self.0, a.0, b.0) })
    }

    // Approximate reciprocal (12-bit precision, ~5× faster than `div`).
    #[inline(always)]
    pub fn rcp(self) -> Self {
        Self(unsafe { _mm_rcp_ps(self.0) })
    }

    /// Approximate reciprocal square root.
    #[inline(always)]
    pub fn rsqrt(self) -> Self {
        Self(unsafe { _mm_rsqrt_ps(self.0) })
    }

    /// Per-lane square root.
    #[inline(always)]
    pub fn sqrt(self) -> Self {
        Self(unsafe { _mm_sqrt_ps(self.0) })
    }

    // Min, Max, Abs, Clamp

    #[inline(always)]
    pub fn min(self, rhs: Self) -> Self {
        Self(unsafe { _mm_min_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn max(self, rhs: Self) -> Self {
        Self(unsafe { _mm_max_ps(self.0, rhs.0) })
    }

    /// Absolute value via clearing the sign bit.
    #[inline(always)]
    pub fn abs(self) -> Self {
        let mask = unsafe { _mm_castsi128_ps(_mm_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
        Self(unsafe { _mm_and_ps(self.0, mask) })
    }

    /// Clamp each lane to `[lo, hi]`.
    #[inline(always)]
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        self.max(lo).min(hi)
    }

    // Bitwise

    #[inline(always)]
    pub fn bit_and(self, rhs: Self) -> Self {
        Self(unsafe { _mm_and_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn bit_or(self, rhs: Self) -> Self {
        Self(unsafe { _mm_or_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn bit_xor(self, rhs: Self) -> Self {
        Self(unsafe { _mm_xor_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn bit_andnot(self, rhs: Self) -> Self {
        Self(unsafe { _mm_andnot_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn bit_not(self) -> Self {
        let all_ones = unsafe { _mm_castsi128_ps(_mm_set1_epi32(-1)) };
        Self(unsafe { _mm_xor_ps(self.0, all_ones) })
    }

    // Comparisons

    #[inline(always)]
    pub fn cmpeq(self, rhs: Self) -> Self {
        Self(unsafe { _mm_cmpeq_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn cmplt(self, rhs: Self) -> Self {
        Self(unsafe { _mm_cmplt_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn cmple(self, rhs: Self) -> Self {
        Self(unsafe { _mm_cmple_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn cmpgt(self, rhs: Self) -> Self {
        Self(unsafe { _mm_cmpgt_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn cmpge(self, rhs: Self) -> Self {
        Self(unsafe { _mm_cmpge_ps(self.0, rhs.0) })
    }

    #[inline(always)]
    pub fn movemask(self) -> u32 {
        unsafe { _mm_movemask_ps(self.0) as u32 }
    }

    // Blend, Shuffle & Swizzle

    /// Compile-time blend: for each bit `i` in `IMM`, pick lane `i` from `b`
    /// if the bit is 1, else from `a`.
    #[inline(always)]
    pub fn blend<const IMM: i32>(a: Self, b: Self) -> Self {
        Self(unsafe { _mm_blend_ps(a.0, b.0, IMM) })
    }

    /// Runtime blend: picks `b` where `mask` sign bit is set, else `a`.
    #[inline(always)]
    pub fn blendv(a: Self, b: Self, mask: Self) -> Self {
        Self(unsafe { _mm_blendv_ps(a.0, b.0, mask.0) })
    }

    /// General 2-source shuffle.  `IMM` is a `shuffle_mask(z, y, x, w)`.
    /// Result lanes: `[a[w], a[x], b[y], b[z]]`.
    #[inline(always)]
    pub fn shuffle<const IMM: i32>(a: Self, b: Self) -> Self {
        Self(unsafe { _mm_shuffle_ps(a.0, b.0, IMM) })
    }

    #[inline(always)]
    pub fn unpack_lo(a: Self, b: Self) -> Self {
        Self(unsafe { _mm_unpacklo_ps(a.0, b.0) })
    }

    #[inline(always)]
    pub fn unpack_hi(a: Self, b: Self) -> Self {
        Self(unsafe { _mm_unpackhi_ps(a.0, b.0) })
    }

    // Swizzle convenience

    #[inline(always)]
    pub fn splat_x(self) -> Self {
        Self::shuffle::<{ shuffle_mask(0, 0, 0, 0) }>(self, self)
    }

    #[inline(always)]
    pub fn splat_y(self) -> Self {
        Self::shuffle::<{ shuffle_mask(1, 1, 1, 1) }>(self, self)
    }

    #[inline(always)]
    pub fn splat_z(self) -> Self {
        Self::shuffle::<{ shuffle_mask(2, 2, 2, 2) }>(self, self)
    }

    #[inline(always)]
    pub fn splat_w(self) -> Self {
        Self::shuffle::<{ shuffle_mask(3, 3, 3, 3) }>(self, self)
    }

    /// `[x, y, z, w]` → `[y, x, z, w]`
    #[inline(always)]
    pub fn swap_xy(self) -> Self {
        Self::shuffle::<{ shuffle_mask(3, 2, 0, 1) }>(self, self)
    }

    /// `[x, y, z, w]` → `[y, x, w, z]` (swap each adjacent pair).
    #[inline(always)]
    pub fn swap_pairs(self) -> Self {
        Self::shuffle::<{ shuffle_mask(2, 3, 0, 1) }>(self, self)
    }

    /// `[x, y, z, w]` → `[z, w, x, y]` (swap the two halves).
    #[inline(always)]
    pub fn swap_halves(self) -> Self {
        Self::shuffle::<{ shuffle_mask(1, 0, 3, 2) }>(self, self)
    }

    /// `[x, y, z, w]` → `[w, z, y, x]`
    #[inline(always)]
    pub fn reverse(self) -> Self {
        Self::shuffle::<{ shuffle_mask(0, 1, 2, 3) }>(self, self)
    }

    // Horizontal operations

    /// SSE4.1 dot product. `IMM` controls which lanes participate and where
    /// the result is placed. For a full 4-lane dot written to all lanes:
    /// `a.dot::<0xFF>(b)`.
    ///
    /// Common masks:
    ///   `0xFF`:  dot of all 4 lanes, broadcast to all 4 lanes
    ///   `0x7F`: dot of lanes 0-2 (vec3), broadcast to all 4 lanes
    ///   `0x71`: dot of lanes 0-2, result only in lane 0
    #[inline(always)]
    pub fn dot<const IMM: i32>(self, rhs: Self) -> Self {
        Self(unsafe { _mm_dp_ps(self.0, rhs.0, IMM) })
    }

    /// Sum all 4 lanes into a scalar.
    /// Uses two `hadd` instructions (SSE3), fine for non-hot-loop usage.
    /// In a hot loop you'd likely use shuffles instead.
    #[inline(always)]
    pub fn hsum(self) -> f32 {
        unsafe {
            let shuf = _mm_movehdup_ps(self.0); // [1,1,3,3]
            let sums = _mm_add_ps(self.0, shuf); // [0+1, _, 2+3, _]
            let shuf = _mm_movehl_ps(sums, sums); // [2+3, _, _, _]
            let sums = _mm_add_ss(sums, shuf);
            _mm_cvtss_f32(sums)
        }
    }

    // Lane access (scalar extract, insert)

    /// Extract lane 0.
    #[inline(always)]
    pub fn x(self) -> f32 {
        unsafe { _mm_cvtss_f32(self.0) }
    }

    /// Extract lane 1.
    #[inline(always)]
    pub fn y(self) -> f32 {
        unsafe {
            let shuffled = _mm_shuffle_ps(self.0, self.0, shuffle_mask(1, 1, 1, 1));
            _mm_cvtss_f32(shuffled)
        }
    }

    /// Extract lane 2.
    #[inline(always)]
    pub fn z(self) -> f32 {
        unsafe {
            let shuffled = _mm_shuffle_ps(self.0, self.0, shuffle_mask(2, 2, 2, 2));
            _mm_cvtss_f32(shuffled)
        }
    }

    /// Extract lane 3.
    #[inline(always)]
    pub fn w(self) -> f32 {
        unsafe {
            let shuffled = _mm_shuffle_ps(self.0, self.0, shuffle_mask(3, 3, 3, 3));
            _mm_cvtss_f32(shuffled)
        }
    }

    /// Store all 4 lanes to an array (unaligned).
    #[inline(always)]
    pub fn to_array(self) -> [f32; 4] {
        // We could use MaybeUninit here but the gains seems neglible compared to one more unsafe.
        let mut out = [0.0f32; 4];
        self.store(&mut out);
        out
    }
}

// std::ops trait impls

impl Add for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        self.add(rhs)
    }
}

impl Sub for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        self.sub(rhs)
    }
}

impl Mul for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        self.mul(rhs)
    }
}

impl Div for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        self.div(rhs)
    }
}

impl Neg for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self {
        Self::zero().sub(self)
    }
}

impl AddAssign for F32x4 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs);
    }
}

impl SubAssign for F32x4 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.sub(rhs);
    }
}

impl MulAssign for F32x4 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}

impl DivAssign for F32x4 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(rhs);
    }
}

// Scalar broadcast ops

impl Mul<f32> for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self {
        self.mul(Self::splat(rhs))
    }
}

impl Div<f32> for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self {
        self.div(Self::splat(rhs))
    }
}

impl Add<f32> for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: f32) -> Self {
        self.add(Self::splat(rhs))
    }
}

impl Sub<f32> for F32x4 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: f32) -> Self {
        self.sub(Self::splat(rhs))
    }
}

// Debug

impl std::fmt::Debug for F32x4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.to_array();
        write!(f, "F32x4({}, {}, {}, {})", v[0], v[1], v[2], v[3])
    }
}

impl std::fmt::Display for F32x4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.to_array();
        write!(f, "[{}, {}, {}, {}]", v[0], v[1], v[2], v[3])
    }
}
