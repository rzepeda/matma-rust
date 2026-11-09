use super::A;
#[test]
fn real_a_creates_real_b_using_the_production_path() {
    let a = A::new(6);
    assert_eq!(a.create_b().calculate(3), 18);
    assert_eq!(a.checkout(4), 24);
}
