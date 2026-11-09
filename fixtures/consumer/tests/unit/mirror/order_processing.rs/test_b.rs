use super::matma_mock_a::MockA;
#[test]
fn real_b_uses_stateful_mock_a_after_creation() {
    let mock = MockA::new(4);
    let b = mock.create_b(); // B is real; MockA supplies the creator role.
    assert_eq!(b.calculate(3), 12);
    mock.set_rate(9); // B observes changes in MockA's state.
    assert_eq!(b.calculate(3), 27);
    assert_eq!(b.internal_rate(), 9);
}
