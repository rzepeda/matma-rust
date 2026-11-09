use super::A;
#[test]
fn a_private_member_is_accessible_without_public_api() {
    let a = A::new(7);
    assert_eq!(a.private_rate(), 7);
}
