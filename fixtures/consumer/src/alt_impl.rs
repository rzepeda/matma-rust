use crate::alt_processor::AltProcessor;
impl crate::order_processing::PricingContract for AltProcessor {
    fn rate(&self) -> u32 { self.rate_value() }
}
