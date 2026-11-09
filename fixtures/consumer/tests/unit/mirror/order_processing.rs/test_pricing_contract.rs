use super::{A, PricingContract};
fn verify_contract<T: PricingContract>(real: &T) {
    assert!(real.rate() > 0);
    assert_eq!(real.doubled_rate(), real.rate() * 2);
}
#[test]
fn contract_passes_for_real_a_implementor() {
    verify_contract(&A::new(11));
}
