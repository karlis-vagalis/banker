use banker::models::{
    Lookback,
    transaction_query::{BookingDateRange, FieldSelection},
};

#[test]
fn inputs_carry_their_invariants() {
    assert!("0d".parse::<Lookback>().is_err());
    assert!("-1d".parse::<Lookback>().is_err());
    assert!("1month".parse::<Lookback>().is_ok());
    let date = "2026-01-01".parse().unwrap();
    assert!(BookingDateRange::new(Some(date), Some(date)).is_err());
    assert!("".parse::<FieldSelection>().is_err());
    assert!("id,id".parse::<FieldSelection>().is_err());
    assert!("metadata..category".parse::<FieldSelection>().is_err());
    assert!("metadata.category,amount".parse::<FieldSelection>().is_ok());
    assert!("content.anything".parse::<FieldSelection>().is_err());
}
