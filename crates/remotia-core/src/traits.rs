use std::borrow::Borrow;

use async_trait::async_trait;

#[async_trait]
pub trait FrameProcessor<F> {
    async fn process(&mut self, frame_data: F) -> Option<F>;
}

pub trait FrameProperties<K, V> {
    fn set(&mut self, key: K, value: V);
    fn get(&self, key: &K) -> Option<V>;
}

pub trait PullableFrameProperties<K, V> {
    fn push(&mut self, key: K, value: V);
    fn pull(&mut self, key: &K) -> Option<V>;
}

pub trait OptionalFrameData<D> {
    fn find_mut(&mut self) -> &mut Option<D>;
    fn find(&self) -> &Option<D>;
}

pub trait BorrowFrameProperties<K, V> {
    fn get_ref(&self, key: &K) -> Option<&V>;
}

pub trait BorrowMutFrameProperties<K, V> {
    fn get_mut_ref(&mut self, key: &K) -> Option<&mut V>;
}

pub trait FrameError<E> {
    fn report_error(&mut self, error: E);
    fn get_error(&self) -> Option<E>;
}

/// Ordered chain of [`FrameProcessor`]s (`Component`, `Sequential`,
/// `ProfiledSequential`, ...).
///
/// This abstracts over the concrete `append` builders so helpers like the
/// buffer-pool binding (`remotia-buffer-utils`) can be defined once over this
/// trait instead of once per chain type. Each chain keeps its inherent
/// `append`; this trait's `chain` forwards to it.
pub trait ProcessorChain<F>: Sized {
    fn chain<T: FrameProcessor<F> + Send + 'static>(self, processor: T) -> Self;
}