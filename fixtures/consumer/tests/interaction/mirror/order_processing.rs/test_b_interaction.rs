use super::{A, B};
#[test]
fn real_b_interacts_with_real_a() {
    let a = A::new(5);
    let b = B::new(a);
    assert_eq!(b.calculate(2), 10);
}
