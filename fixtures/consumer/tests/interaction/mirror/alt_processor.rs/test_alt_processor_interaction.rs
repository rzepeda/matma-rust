use super::AltProcessor;
use crate::order_processing::PricingContract;
#[test]
fn real_alt_processor_satisfies_production_contract() {
    assert_eq!(AltProcessor::new(6).doubled_rate(),12);
}
