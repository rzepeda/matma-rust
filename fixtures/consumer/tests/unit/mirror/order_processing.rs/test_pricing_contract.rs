use super::{A, PricingContract};
use crate::alt_processor::AltProcessor;
fn verify_contract<T: PricingContract>(real: &T) {
    assert!(real.rate() > 0);
    assert_eq!(real.doubled_rate(), real.rate() * 2);
}
#[test]
fn contract_passes_for_real_a_implementor() {
    verify_contract(&A::new(11));
    verify_contract(&AltProcessor::new(7));
}
