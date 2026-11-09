use super::{B, PricingContract};
use std::cell::Cell;
use std::rc::Rc;

/// A centralized mock; it can create a *real* B and vary its state after creation.
#[derive(Clone)]
pub struct MockA {
    rate: Rc<Cell<u32>>,
}
impl MockA {
    pub fn new(rate: u32) -> Self { Self { rate: Rc::new(Cell::new(rate)) } }
    pub fn set_rate(&self, rate: u32) { self.rate.set(rate); }
    pub fn create_b(&self) -> B<Self> { B::new(self.clone()) }
}
impl PricingContract for MockA {
    fn rate(&self) -> u32 { self.rate.get() }
}
