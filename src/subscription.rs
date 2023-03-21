use futures::{stream::BoxStream, Stream, StreamExt};

/// Wraps a Stream to add Drop/Unsubscribe handling.
pub struct Subscription<T> {
    inner: BoxStream<'static, T>,
    on_unsubscribe: Option<Box<dyn FnOnce() + Send>>,
}

impl<T> Subscription<T> {
    /// Wrap the given stream alongside an optional unsubscribe callback.
    ///
    /// The unsubscribe callback is called on Drop, or when unsubcribe is called for the first time.
    /// It can be used to clean up any state.
    pub fn wrap(
        stream: impl Stream<Item = T> + Send + 'static,
        f: Option<impl FnOnce() + Send + 'static>,
    ) -> Self {
        Self {
            inner: Box::pin(stream),
            on_unsubscribe: f.map(|f| Box::new(f) as Box<dyn FnOnce() + Send>),
        }
    }

    /// Unsubscribe from the Subscription. This is automatically called on drop.
    pub fn unsubscribe(&mut self) {
        if let Some(f) = self.on_unsubscribe.take() {
            f();
        }
    }
}

impl<T> Stream for Subscription<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

impl<T> Drop for Subscription<T> {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_is_send() {
        fn is_send<T: Send>() {}
        is_send::<Subscription<()>>();
    }
}
