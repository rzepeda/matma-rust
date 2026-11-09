pub struct AltProcessor {
    rate: u32,
}
impl AltProcessor {
    pub fn new(rate: u32) -> Self { Self { rate } }
    pub(crate) fn rate_value(&self) -> u32 { self.rate }
}
