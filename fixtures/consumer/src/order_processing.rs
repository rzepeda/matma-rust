use std::cell::Cell;
use std::rc::Rc;

/// A contract which is exercised against real implementations.
pub trait PricingContract {
    fn rate(&self) -> u32;
    fn doubled_rate(&self) -> u32 { self.rate() * 2 }
}

/// A deliberately private production type sharing its module with B.
#[derive(Clone)]
pub struct A {
    rate: Rc<Cell<u32>>,
}
impl A {
    pub fn new(rate: u32) -> Self {
        Self { rate: Rc::new(Cell::new(rate)) }
    }
    fn private_rate(&self) -> u32 { self.rate.get() }
    pub fn create_b(&self) -> B<A> { B::new(self.clone()) }
    pub fn checkout(&self, quantity: u32) -> u32 {
        self.create_b().calculate(quantity)
    }
}
impl PricingContract for A {
    fn rate(&self) -> u32 { self.private_rate() }
}

/// B retains a live reference to state behind the contract.
pub struct B<S: PricingContract> {
    owner: S,
}
impl<S: PricingContract> B<S> {
    pub fn new(owner: S) -> Self { Self { owner } }
    pub fn calculate(&self, quantity: u32) -> u32 {
        quantity * self.owner.rate()
    }
    fn internal_rate(&self) -> u32 { self.owner.rate() }
}
