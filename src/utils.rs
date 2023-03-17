use rust_decimal::Decimal;

pub fn round_to_nearest_tick(price: Decimal, tick_size: Decimal) -> Decimal {
    let mut rounded = price / tick_size;
    rounded = rounded.round();
    rounded * tick_size
}
