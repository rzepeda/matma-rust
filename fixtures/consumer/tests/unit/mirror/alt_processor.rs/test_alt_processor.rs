use super::AltProcessor;
#[test]
fn private_state_can_be_tested_inside_another_source_module() {
    assert_eq!(AltProcessor::new(14).rate,14);
}
