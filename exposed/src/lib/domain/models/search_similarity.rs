//! Declarations are groups of funding entries that are submitted by MPs to Parliament.

#[allow(dead_code)]
#[derive(Clone)]
/// Trigram similarity score
pub struct SearchSimilarity(f32);

impl SearchSimilarity {
    /// Trigram similarity score value
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl From<f32> for SearchSimilarity {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod f64_conversion {
    use crate::domain::models::search_similarity::SearchSimilarity;

    #[test]
    fn behaves_as_expected() {
        let small = SearchSimilarity::from(1_f32);
        let big = SearchSimilarity::from(2_f32);

        assert!(big.value() > small.value());
    }

    #[test]
    fn does_not_round() {
        let small = SearchSimilarity::from(0.1_f32);
        let big = SearchSimilarity::from(0.9_f32);

        assert!(big.value() > small.value());
    }
}
