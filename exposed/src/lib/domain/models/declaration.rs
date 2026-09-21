//! Declarations are groups of funding entries that are submitted by MPs to Parliament.

use chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Clone)]
struct Declaration {
    category: DeclarationCategory,
    registration_date: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Clone)]
enum DeclarationCategory {}
