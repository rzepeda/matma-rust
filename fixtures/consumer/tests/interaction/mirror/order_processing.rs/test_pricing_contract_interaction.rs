use super::{A, PricingContract};
#[test]
fn real_implementor_exercises_default_trait_behavior() {
    let a = A::new(12);
    assert_eq!(a.doubled_rate(), 24);
}
