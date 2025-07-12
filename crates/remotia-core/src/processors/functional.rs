use std::marker::PhantomData;

use async_trait::async_trait;

use crate::{pipeline::component::Component, traits::FrameProcessor};

pub struct Function<F> {
    function: fn(F) -> Option<F>,
}

impl<F> Function<F> {
    pub fn new(function: fn(F) -> Option<F>) -> Self {
        Self { function }
    }
}

#[async_trait]
impl<F: Send> FrameProcessor<F> for Function<F> {
    async fn process(&mut self, frame_data: F) -> Option<F> {
        (self.function)(frame_data)
    }
}

pub struct Closure<FD, FN>
where
    FN: Fn(FD) -> Option<FD>,
{
    function: FN,
    data_type: PhantomData<FD>,
}

impl<FD, FN> Closure<FD, FN>
where
    FN: Fn(FD) -> Option<FD>,
{
    pub fn new(function: FN) -> Self {
        Self {
            function,
            data_type: PhantomData,
        }
    }
}

#[async_trait]
impl<FD, FN> FrameProcessor<FD> for Closure<FD, FN>
where
    FD: Send,
    FN: Fn(FD) -> Option<FD> + Send,
{
    async fn process(&mut self, frame_data: FD) -> Option<FD> {
        (self.function)(frame_data)
    }
}

pub trait ClosureAppends<FN> {
    fn closure(self, closure: FN) -> Self;
}

impl<FD, FN> ClosureAppends<FN> for Component<FD>
where
    FD: Default + Send + 'static,
    FN: Fn(FD) -> Option<FD> + Send + 'static,
{
    fn closure(self, closure: FN) -> Self {
        self.append(Closure::new(closure))
    }
}
