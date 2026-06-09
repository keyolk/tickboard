use crate::models::{format_number, Currency};

pub fn money(value: f64, currency: Currency) -> String {
    match currency {
        Currency::Usd => format!("${}", format_number(value, 2)),
        Currency::Krw => format!("{}원", format_number(value, 0)),
    }
}
