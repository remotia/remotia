use std::{collections::HashMap, time::Duration};

use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use crate::{pool::BuffersPool, BufferAllocator};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum BufferType {
    Test,
}

#[derive(Default)]
struct TestFrameData {
    buffers: HashMap<BufferType, BytesMut>,
}

impl PullableFrameProperties<BufferType, BytesMut> for TestFrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        self.buffers.insert(key, value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        self.buffers.remove(key)
    }
}

#[tokio::test]
async fn test_allocation() {
    let mut allocator = BufferAllocator::new(BufferType::Test, 1024);
    let mut dto = TestFrameData::default();
    dto = allocator.process(dto).await.unwrap();
    assert!(dto.pull(&BufferType::Test).is_some());
}

/// A hard borrower on an exhausted pool must *park* and be woken by the
/// redeeming send, not spin on `try_recv` + `yield_now`: the borrow only
/// completes after the redeem, and promptly.
#[tokio::test]
async fn borrower_parks_on_an_empty_pool_and_wakes_on_redeem() {
    let pool = BuffersPool::new(BufferType::Test, 1, 64).await;
    let mut borrower = pool.borrower();
    let mut redeemer = pool.redeemer();

    // Drain the pool's single slot into a frame that stays held here.
    let held = borrower.process(TestFrameData::default()).await.unwrap();

    let borrowing = tokio::spawn(async move { borrower.process(TestFrameData::default()).await });

    // Long enough for the borrower to reach the empty pool and park there.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        !borrowing.is_finished(),
        "the borrower took a buffer from an empty pool"
    );

    // The redeeming send is what wakes the parked borrower.
    redeemer.process(held).await.unwrap();

    let mut dto = tokio::time::timeout(Duration::from_secs(5), borrowing)
        .await
        .expect("the borrower never woke up after the buffer was redeemed")
        .unwrap()
        .expect("the borrower passes the frame through");
    assert!(dto.pull(&BufferType::Test).is_some());
}

/// When every redeemer of a pool is gone, a hard borrower must fail loudly
/// instead of waiting (or spinning) forever for a buffer that can never
/// arrive.
#[tokio::test]
async fn borrower_panics_once_every_redeemer_is_gone() {
    let pool = BuffersPool::new(BufferType::Test, 1, 64).await;
    let mut borrower = pool.borrower();

    // Take the pool's only buffer, then drop the pool (its sender). No
    // redeemer was ever created, so nothing can refill the channel.
    let held = borrower.process(TestFrameData::default()).await.unwrap();
    drop(pool);
    drop(held);

    let borrowing = tokio::spawn(async move { borrower.process(TestFrameData::default()).await });

    let joined = tokio::time::timeout(Duration::from_secs(5), borrowing)
        .await
        .expect("the borrower waited on a pool whose redeemers are all gone");
    match joined {
        Ok(_) => panic!("the borrower must panic on a disconnected pool"),
        Err(err) => assert!(err.is_panic(), "the borrower must panic, not hang"),
    }
}
