//! Binding helpers for buffer pools.
//!
//! [`BufferPoolBinding`] extends any [`ProcessorChain`](remotia_core::traits::ProcessorChain)
//! so a pool can be bound with `.borrow(&pool)` / `.redeem(&pool)` instead of
//! `.append(pool.borrower())` / `.append(pool.redeemer())`:
//!
//! ```ignore
//! Component::new()
//!     .borrow(&raw_pool)
//!     .append(decoder)
//!     .redeem(&raw_pool)
//! ```
//!
//! The trait is blanket-implemented over `ProcessorChain`, so `Component`,
//! `Sequential` and `ProfiledSequential` (which implements `ProcessorChain`
//! in `remotia-profilation-utils`) all gain the helpers without this crate
//! depending on the profilation crate.

use std::fmt::Debug;

use remotia_core::traits::{ProcessorChain, PullableFrameProperties};

use crate::{pool::BuffersPool, BytesMut};

/// Binds [`BuffersPool`]s to a processor chain.
///
/// Each method appends the corresponding pool processor:
/// - [`borrow`](Self::borrow): hard [`BufferBorrower`](crate::pool::BufferBorrower),
/// - [`borrow_soft`](Self::borrow_soft): soft borrower (lets frames through
///   without a slot when the pool is empty),
/// - [`redeem`](Self::redeem): hard [`BufferRedeemer`](crate::pool::BufferRedeemer),
/// - [`redeem_soft`](Self::redeem_soft): soft redeemer (tolerates a missing slot).
pub trait BufferPoolBinding<F, K: Copy>: ProcessorChain<F> + Sized {
    fn borrow(self, pool: &BuffersPool<K>) -> Self;
    fn borrow_soft(self, pool: &BuffersPool<K>) -> Self;
    fn redeem(self, pool: &BuffersPool<K>) -> Self;
    fn redeem_soft(self, pool: &BuffersPool<K>) -> Self;
}

impl<T, F, K> BufferPoolBinding<F, K> for T
where
    T: ProcessorChain<F>,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
    K: Copy + Debug + Send + 'static,
{
    fn borrow(self, pool: &BuffersPool<K>) -> Self {
        self.chain(pool.borrower())
    }

    fn borrow_soft(self, pool: &BuffersPool<K>) -> Self {
        self.chain(pool.borrower().soft())
    }

    fn redeem(self, pool: &BuffersPool<K>) -> Self {
        self.chain(pool.redeemer())
    }

    fn redeem_soft(self, pool: &BuffersPool<K>) -> Self {
        self.chain(pool.redeemer().soft())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use remotia_core::{
        pipeline::component::Component, processors::containers::sequential::Sequential,
        traits::FrameProcessor,
    };

    use super::*;

    #[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
    enum TestKey {
        Slot,
    }

    #[derive(Default)]
    struct TestFrame {
        buffers: HashMap<TestKey, BytesMut>,
    }

    impl PullableFrameProperties<TestKey, BytesMut> for TestFrame {
        fn push(&mut self, key: TestKey, value: BytesMut) {
            self.buffers.insert(key, value);
        }

        fn pull(&mut self, key: &TestKey) -> Option<BytesMut> {
            self.buffers.remove(key)
        }
    }

    #[tokio::test]
    async fn sequential_borrow_then_redeem_roundtrips_a_buffer() {
        let pool = BuffersPool::new(TestKey::Slot, 1, 16).await;
        let mut chain = Sequential::<TestFrame>::new().borrow(&pool).redeem(&pool);

        let frame = chain.process(TestFrame::default()).await.unwrap();
        assert!(
            !frame.buffers.contains_key(&TestKey::Slot),
            "the redeemer must pull the borrowed slot"
        );

        // No leak: the pool still lends afterwards.
        let mut borrow_only = Sequential::<TestFrame>::new().borrow(&pool);
        let frame = borrow_only.process(TestFrame::default()).await.unwrap();
        assert!(frame.buffers.contains_key(&TestKey::Slot));
    }

    #[tokio::test]
    async fn soft_borrow_lets_frames_through_on_an_empty_pool() {
        let pool = BuffersPool::new(TestKey::Slot, 1, 16).await;
        // Drain the only buffer with a hard borrow that is never redeemed.
        let mut drain = Sequential::<TestFrame>::new().borrow(&pool);
        let _held = drain.process(TestFrame::default()).await.unwrap();

        let mut soft = Sequential::<TestFrame>::new().borrow_soft(&pool);
        let frame = soft.process(TestFrame::default()).await.unwrap();
        assert!(
            !frame.buffers.contains_key(&TestKey::Slot),
            "a soft borrower on an empty pool must pass the frame without a slot"
        );
    }

    #[tokio::test]
    async fn soft_redeem_tolerates_a_missing_slot() {
        let pool = BuffersPool::new(TestKey::Slot, 1, 16).await;
        let mut chain = Sequential::<TestFrame>::new().redeem_soft(&pool);
        let frame = chain.process(TestFrame::default()).await.unwrap();
        assert!(!frame.buffers.contains_key(&TestKey::Slot));
    }

    #[tokio::test]
    async fn component_and_sequential_both_implement_the_binding() {
        fn assert_binding<C: BufferPoolBinding<TestFrame, TestKey>>() {}
        assert_binding::<Component<TestFrame>>();
        assert_binding::<Sequential<TestFrame>>();

        // Runtime behaviour is exercised through `Sequential` (same blanket
        // impl, same borrower type); `Component` forwards `chain` to the same
        // `append`, so a compile-time proof plus the roundtrip above covers it.
        let pool = BuffersPool::new(TestKey::Slot, 1, 16).await;
        let _component = Component::<TestFrame>::new()
            .borrow(&pool)
            .borrow_soft(&pool)
            .redeem(&pool)
            .redeem_soft(&pool);
    }
}
