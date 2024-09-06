//! [`metrics::Layer`] implementations.
//!
//! [`metrics::Layer`]: Layer

pub use metrics_util::layers::Layer;

/// No-op [`metrics::Layer`] which returns the received [`metrics::Recorder`]
/// "as is".
///
/// [`metrics::Layer`]: Layer
#[derive(Clone, Copy, Debug)]
pub struct Identity;

impl<R> Layer<R> for Identity {
    type Output = R;

    #[expect(clippy::renamed_function_params, reason = "impl related")]
    fn layer(&self, itself: R) -> R {
        itself
    }
}

/// [`metrics::Layer`] similar to [`metrics::layers::Stack`], but nests the
/// [`metrics::Layer`]s themselves instead of [`metrics::Recorder`]s.
///
/// [`metrics::Layer`]: Layer
/// [`metrics::layers::Stack`]: metrics_util::layers::Stack
#[derive(Clone, Copy, Debug)]
pub struct Stack<Head = Identity, Tail = Identity>(Head, Tail);

impl Stack {
    /// Returns a growable [`Stack`] of [`metrics::Layer`]s, being no-op by
    /// default.
    ///
    /// [`metrics::Layer`]: Layer
    #[must_use]
    pub const fn identity() -> Self {
        Self(Identity, Identity)
    }
}

impl<H, T> Stack<H, T> {
    /// Pushes the provided [`metrics::Layer`] on top of this [`Stack`],
    /// wrapping it.
    ///
    /// [`metrics::Layer`]: Layer
    #[must_use]
    pub const fn push<R, L: Layer<R>>(self, layer: L) -> Stack<L, Self> {
        Stack(layer, self)
    }
}

#[warn(clippy::missing_trait_methods)]
impl<R, H, T> Layer<R> for Stack<H, T>
where
    H: Layer<<T as Layer<R>>::Output>,
    T: Layer<R>,
{
    type Output = H::Output;

    fn layer(&self, inner: R) -> Self::Output {
        self.0.layer(self.1.layer(inner))
    }
}
