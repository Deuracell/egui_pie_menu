use num_traits::Float;

/// A floating-point wrapper that uses `NaN` as a sentinel for "disabled".
///
/// Instead of pairing a value with a separate `enabled: bool`, set the value
/// to `NaN` when it should have no effect. Use [`SmartFloat::new`] to create
/// an enabled value and [`SmartFloat::is_enabled`] to check it.
pub struct SmartFloat<T: Float>(T);

impl<T: Float> SmartFloat<T> {
    /// Wraps `value`. Pass `NAN` to create a disabled instance.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Returns `true` when the inner value is not `NaN`.
    pub fn is_enabled(&self) -> bool {
        self.0 == self.0
    }

    /// Returns the inner value.
    pub fn get(&self) -> T {
        self.0
    }
}
