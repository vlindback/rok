// lerp.rs
//
// rok-math library
//

pub trait Lerp<F> {
    fn lerp(self, other: Self, t: F) -> Self;
}
