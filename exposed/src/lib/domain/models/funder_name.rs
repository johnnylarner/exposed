//! Parallel name features for later entity resolution, without identity decisions.

use serde::Serialize;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Whether the source supplies a usable name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NameStatus {
    /// A nonempty name, retained without guessing its entity type.
    Present,
    /// No name or only whitespace.
    Missing,
    /// An explicit confidentiality or anonymity placeholder.
    Withheld,
}

/// Independent candidate representations, never keys for merging observations.
#[derive(Debug, Serialize)]
pub struct FunderNameFeatures {
    /// Original source spelling, including placeholders and whitespace.
    pub name_raw: Option<String>,
    /// Explicit absence or withholding classification.
    pub name_status: NameStatus,
    /// NFKC, lowercase, and collapsed whitespace.
    pub name_normalized: Option<String>,
    /// Normalized name with decomposed combining marks removed.
    pub name_accent_folded: Option<String>,
    /// Punctuation-normalized tokens in source order.
    pub name_tokens: Option<String>,
    /// Sorted tokens, retaining repeated words.
    pub name_token_key: Option<String>,
    /// Normalized primary name without explicit trading-as text.
    pub primary_name_normalized: Option<String>,
    /// Primary-name tokens without an explicit UK legal suffix.
    pub organisation_core: Option<String>,
    /// Token sequence without common leading honorifics.
    pub person_core: Option<String>,
    /// Initial letters of the person-core tokens.
    pub person_initials: Option<String>,
    /// Organisation-core initials, excluding listed connective words.
    pub organisation_initials: Option<String>,
    /// A source acronym whose letters agree with an explicit parenthetical expansion.
    pub explicit_acronym: Option<String>,
    /// Normalized expansion validated against the explicit acronym.
    pub acronym_expansion_normalized: Option<String>,
    /// Parenthetical contents in their original spelling and order.
    pub parenthetical_text: Vec<String>,
    /// Names following an explicit trading-as marker, without inferred aliases.
    pub explicit_aliases: Vec<String>,
    /// Normalized forms of those explicit aliases, in the same order.
    pub alias_normalized: Vec<String>,
    /// Literal `and` or ampersand evidence, without claiming multiple entities.
    pub has_conjunction: bool,
    /// At least one unmatched opening or closing parenthesis.
    pub has_unbalanced_parentheses: bool,
}

impl FunderNameFeatures {
    /// Derives parallel features while retaining the original evidence.
    #[must_use]
    pub fn from_name(raw: Option<&str>) -> Self {
        let normalized = raw.map(normalize);
        let name_status = match normalized.as_deref() {
            None | Some("") => NameStatus::Missing,
            Some("confidential" | "withheld" | "name withheld" | "anonymous" | "not disclosed") => {
                NameStatus::Withheld
            }
            Some(_) => NameStatus::Present,
        };
        let (parenthetical_text, has_unbalanced_parentheses) = parentheses(raw.unwrap_or(""));
        let usable = normalized.filter(|_| name_status == NameStatus::Present);
        let name_accent_folded = usable.as_ref().map(|name| {
            name.nfd()
                .filter(|c| !is_combining_mark(*c))
                .collect::<String>()
        });
        let name_tokens = usable.as_deref().map(tokens);
        let has_conjunction = name_tokens
            .as_deref()
            .is_some_and(|name| name.split_whitespace().any(|word| word == "and"));
        let name_token_key = name_tokens.as_deref().map(|name| {
            let mut words = name.split_whitespace().collect::<Vec<_>>();
            words.sort_unstable();
            words.join(" ")
        });
        let primary_name_normalized = raw
            .filter(|_| name_status == NameStatus::Present)
            .map(|raw| normalize(&primary_name(raw)));
        let organisation_core = primary_name_normalized
            .as_deref()
            .map(|name| organisation_core(&tokens(name)));
        let person_core = name_tokens.as_deref().map(person_core);
        let person_initials = person_core.as_deref().map(|name| initials(name, false));
        let organisation_initials = organisation_core
            .as_deref()
            .map(|name| initials(name, true));
        let (explicit_acronym, acronym_expansion_normalized) = raw
            .and_then(|name| explicit_acronym(name, &parenthetical_text))
            .unzip();
        let explicit_aliases = raw.map_or_else(Vec::new, |name| aliases(name, &parenthetical_text));
        let alias_normalized = explicit_aliases
            .iter()
            .map(|name| normalize(name))
            .collect();
        Self {
            name_raw: raw.map(str::to_owned),
            name_status,
            name_normalized: usable,
            name_accent_folded,
            name_tokens,
            name_token_key,
            primary_name_normalized,
            organisation_core,
            person_core,
            person_initials,
            organisation_initials,
            explicit_acronym,
            acronym_expansion_normalized,
            parenthetical_text,
            explicit_aliases,
            alias_normalized,
            has_conjunction,
            has_unbalanced_parentheses,
        }
    }
}

fn normalize(raw: &str) -> String {
    raw.nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn tokens(name: &str) -> String {
    let mut result = String::new();
    for word in name.split_whitespace() {
        let acronym = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
        let dotted_acronym = acronym.contains('.')
            && acronym.trim_end_matches('.').split('.').all(|part| {
                let mut letters = part.chars();
                letters.next().is_some_and(char::is_alphabetic) && letters.next().is_none()
            });
        for c in word.chars() {
            match c {
                '&' => result.push_str(" and "),
                '\'' | '\u{2019}' | '\u{2018}' | '\u{02bc}' => {}
                '.' if dotted_acronym => {}
                c if c.is_alphanumeric() || is_combining_mark(c) => result.push(c),
                _ => result.push(' '),
            }
        }
        result.push(' ');
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn organisation_core(name: &str) -> String {
    const SUFFIXES: &[&[&str]] = &[
        &["public", "limited", "company"],
        &["community", "interest", "company"],
        &["limited", "liability", "partnership"],
        &["limited", "partnership"],
        &["limited"],
        &["ltd"],
        &["plc"],
        &["llp"],
        &["lp"],
        &["cic"],
    ];
    let mut words = name.split_whitespace().collect::<Vec<_>>();
    if let Some(suffix) = SUFFIXES.iter().find(|suffix| words.ends_with(suffix)) {
        words.truncate(words.len() - suffix.len());
    }
    words.join(" ")
}

fn person_core(name: &str) -> String {
    const TITLES: &[&str] = &[
        "mr",
        "mrs",
        "ms",
        "miss",
        "mx",
        "dr",
        "prof",
        "professor",
        "sir",
        "dame",
        "lord",
        "lady",
        "rev",
        "reverend",
        "rt",
        "hon",
    ];
    name.split_whitespace()
        .skip_while(|word| TITLES.contains(word))
        .collect::<Vec<_>>()
        .join(" ")
}

fn initials(name: &str, omit_connectives: bool) -> String {
    const CONNECTIVES: &[&str] = &["a", "an", "and", "for", "in", "of", "on", "the", "to"];
    name.split_whitespace()
        .filter(|word| !omit_connectives || !CONNECTIVES.contains(word))
        .filter_map(|word| word.chars().next())
        .collect()
}

fn parentheses(raw: &str) -> (Vec<String>, bool) {
    let mut result = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    let mut unbalanced = false;
    for (index, c) in raw.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = index + 1;
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    result.push(raw[start..index].to_owned());
                }
            }
            ')' => unbalanced = true,
            _ => {}
        }
    }
    if depth > 0 {
        result.push(raw[start..].to_owned());
    }
    (result, unbalanced || depth > 0)
}

fn explicit_acronym(raw: &str, parenthetical: &[String]) -> Option<(String, String)> {
    let before = raw.split('(').next()?.trim();
    for expansion in parenthetical {
        for (candidate, full) in [(before, expansion.as_str()), (expansion.as_str(), before)] {
            let letters = candidate
                .chars()
                .filter(|c| *c != '.' && !c.is_whitespace())
                .collect::<String>();
            if (2..=10).contains(&letters.chars().count())
                && letters.chars().all(|c| c.is_ascii_uppercase())
                && letters.to_lowercase() == initials(&tokens(&normalize(full)), true)
            {
                return Some((letters.to_lowercase(), normalize(full)));
            }
        }
    }
    None
}

fn alias_marker(candidate: &str, top_level_only: bool) -> Option<(usize, usize)> {
    let lower = candidate.to_ascii_lowercase();
    let mut found = None;
    for marker in ["trading as", "t/a"] {
        for (index, _) in lower.match_indices(marker) {
            let boundary = (index == 0 || lower[..index].ends_with([' ', '(', ',']))
                && lower[index + marker.len()..].starts_with(char::is_whitespace);
            let depth = candidate[..index].chars().fold(0_i32, |depth, c| match c {
                '(' => depth + 1,
                ')' => depth - 1,
                _ => depth,
            });
            if boundary
                && (!top_level_only || depth == 0)
                && found.is_none_or(|(previous, _)| index < previous)
            {
                found = Some((index, marker.len()));
            }
        }
    }
    found
}

fn aliases(raw: &str, parenthetical: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for (candidate, top_level_only) in parenthetical
        .iter()
        .map(|name| (name.as_str(), false))
        .chain(std::iter::once((raw, true)))
    {
        if let Some((index, length)) = alias_marker(candidate, top_level_only) {
            let alias = candidate[index + length..].trim().to_owned();
            if !alias.is_empty() && !result.contains(&alias) {
                result.push(alias);
            }
        }
    }
    result
}

fn primary_name(raw: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    let mut start = 0;
    let mut keep_from = 0;
    for (index, c) in raw.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    start = index;
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 && alias_marker(&raw[start + 1..index], false).is_some() {
                    result.push_str(&raw[keep_from..start]);
                    result.push(' ');
                    keep_from = index + 1;
                }
            }
            _ => {}
        }
    }
    result.push_str(&raw[keep_from..]);
    if let Some((index, _)) = alias_marker(&result, true) {
        result.truncate(index);
    }
    result.trim().trim_end_matches(',').trim().to_owned()
}

#[cfg(test)]
mod tests;
