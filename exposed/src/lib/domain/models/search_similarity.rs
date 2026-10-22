//! Declarations are groups of funding entries that are submitted by MPs to Parliament.

#[derive(Clone)]
/// Trigram similarity score
pub struct SearchSimilarity(f32);

impl SearchSimilarity {
    #[must_use]
    /// Trigram similarity score value
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl From<f32> for SearchSimilarity {
    fn from(value: f32) -> Self {
        Self(value)
    }
}
