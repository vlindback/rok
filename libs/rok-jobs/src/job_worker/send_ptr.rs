// send_ptr.rs

#[repr(transparent)]
pub(super) struct SendPtr<T>(pub(super) *const T);

unsafe impl<T: Sync> Send for SendPtr<T> {}
