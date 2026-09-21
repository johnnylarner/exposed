//! Declarations are groups of funding entries that are submitted by MPs to Parliament.

use rust_decimal::Decimal;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SearchSimilarity(i128);

impl SearchSimilarity {
    pub fn value(&self) -> i128 {
        self.0
    }
}

impl From<f64> for SearchSimilarity {
    fn from(value: f64) -> Self {
        Self(Decimal::from_f64_retain(value).map_or(-1, |v| v.as_i128()))
    }
}

#[cfg(test)]
mod f64_conversion {
    use crate::domain::models::search_similarity::SearchSimilarity;

    #[test]
    fn behaves_as_expected() {
        let small = SearchSimilarity::from(1_f64);
        let big = SearchSimilarity::from(2_f64);

        assert!(big.value() > small.value());
    }
}
