//! Shared exact declared totals and current parliament boundary.
use super::entity_details::{
    CategoryTotal, DeclarationCategory, DeclaredGbpPeriodTotal, MemberCurrencyTotal,
    MemberDeclaration,
};
use std::collections::BTreeMap;

pub fn add_known(
    total: &mut Option<bigdecimal::BigDecimal>,
    amount: Option<&bigdecimal::BigDecimal>,
) {
    if let Some(amount) = amount {
        match total {
            Some(total) => *total += amount,
            None => *total = Some(amount.clone()),
        }
    }
}

#[derive(Default)]
pub struct PeriodAggregate {
    currencies: BTreeMap<String, MemberCurrencyTotal>,
    categories: BTreeMap<(String, DeclarationCategory, String), CategoryTotal>,
    declaration_count: usize,
    pub(crate) entry_count: usize,
    pub(crate) unknown_currency_count: usize,
}

impl PeriodAggregate {
    pub(crate) fn observe(&mut self, declaration: &MemberDeclaration) {
        self.declaration_count += 1;
        for entry in &declaration.entries {
            self.entry_count += 1;
            let Some(currency) = &entry.currency else {
                self.unknown_currency_count += 1;
                continue;
            };
            let group =
                self.currencies
                    .entry(currency.clone())
                    .or_insert_with(|| MemberCurrencyTotal {
                        currency: currency.clone(),
                        amount: None,
                        entry_count: 0,
                        unknown_amount_count: 0,
                        categories: Vec::new(),
                    });
            add_known(&mut group.amount, entry.amount.as_ref());
            group.entry_count += 1;
            group.unknown_amount_count += usize::from(entry.amount.is_none());
            let category = self
                .categories
                .entry((
                    currency.clone(),
                    declaration.category,
                    declaration.category_name.clone(),
                ))
                .or_insert_with(|| CategoryTotal {
                    category: declaration.category,
                    category_name: declaration.category_name.clone(),
                    amount: None,
                    entry_count: 0,
                    unknown_amount_count: 0,
                });
            add_known(&mut category.amount, entry.amount.as_ref());
            category.entry_count += 1;
            category.unknown_amount_count += usize::from(entry.amount.is_none());
        }
    }

    pub(crate) fn gbp_total(&self) -> DeclaredGbpPeriodTotal {
        let gbp = self.currencies.get("GBP");
        DeclaredGbpPeriodTotal {
            amount: gbp.and_then(|group| group.amount.clone()),
            declaration_count: self.declaration_count,
            incomplete: self.unknown_currency_count > 0
                || gbp.is_some_and(|group| group.unknown_amount_count > 0),
        }
    }

    pub(crate) fn finish(mut self) -> Vec<MemberCurrencyTotal> {
        for ((currency, _, _), category) in self.categories {
            if let Some(group) = self.currencies.get_mut(&currency) {
                group.categories.push(category);
            }
        }
        self.currencies.into_values().collect()
    }
}
