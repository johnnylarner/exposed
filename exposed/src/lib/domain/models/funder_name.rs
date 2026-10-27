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

/// A recognized leading honorific, retained in source order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonTitle {
    /// The `mr` honorific.
    Mr,
    /// The `mrs` honorific.
    Mrs,
    /// The `ms` honorific.
    Ms,
    /// The `miss` honorific.
    Miss,
    /// The `mx` honorific.
    Mx,
    /// The `dr` honorific.
    Dr,
    /// The `prof` honorific.
    Prof,
    /// The `professor` honorific.
    Professor,
    /// The `sir` honorific.
    Sir,
    /// The `dame` honorific.
    Dame,
    /// The `lord` honorific.
    Lord,
    /// The `baron` honorific.
    Baron,
    /// The `lady` honorific.
    Lady,
    /// The `rev` honorific.
    Rev,
    /// The `reverend` honorific.
    Reverend,
    /// The `rt` honorific.
    Rt,
    /// The `hon` honorific.
    Hon,
}

impl PersonTitle {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "mr" => Some(Self::Mr),
            "mrs" => Some(Self::Mrs),
            "ms" => Some(Self::Ms),
            "miss" => Some(Self::Miss),
            "mx" => Some(Self::Mx),
            "dr" => Some(Self::Dr),
            "prof" => Some(Self::Prof),
            "professor" => Some(Self::Professor),
            "sir" => Some(Self::Sir),
            "dame" => Some(Self::Dame),
            "lord" => Some(Self::Lord),
            "baron" => Some(Self::Baron),
            "lady" => Some(Self::Lady),
            "rev" => Some(Self::Rev),
            "reverend" => Some(Self::Reverend),
            "rt" => Some(Self::Rt),
            "hon" => Some(Self::Hon),
            _ => None,
        }
    }

    const fn is_peerage(self) -> bool {
        match self {
            Self::Lord | Self::Baron => true,
            Self::Mr
            | Self::Mrs
            | Self::Ms
            | Self::Miss
            | Self::Mx
            | Self::Dr
            | Self::Prof
            | Self::Professor
            | Self::Sir
            | Self::Dame
            | Self::Lady
            | Self::Rev
            | Self::Reverend
            | Self::Rt
            | Self::Hon => false,
        }
    }
}

pub(crate) struct PeerageName {
    spelling: String,
    bridge: Option<PeerageBridge>,
}

pub(crate) struct PeerageBridge {
    style: String,
    full_style: String,
    personal_name: String,
}

impl PeerageName {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let normalized = normalize(raw);
        let person = ParsedPersonName::parse(&normalized);
        let spelling = person.normalized?;
        if spelling.contains(" of ") && (person.titles.len() != 1 || !person.titles[0].is_peerage())
        {
            return None;
        }
        if name_like(&spelling) {
            return Some(Self {
                spelling,
                bridge: None,
            });
        }
        if person.titles.len() != 1 || !person.titles[0].is_peerage() {
            return None;
        }
        let (style, personal_name) = spelling.strip_suffix(')')?.split_once(" (")?;
        let personal_name = personal_name.trim();
        let (stem, territory) = style.split_once(" of ")?;
        if !name_like(stem)
            || !name_like(territory)
            || !name_like(personal_name)
            || personal_name.split_whitespace().count() < 2
            || personal_name
                .split_whitespace()
                .any(|word| word.chars().count() < 2)
            || !personal_name
                .strip_suffix(stem)
                .is_some_and(|given| given.ends_with(' '))
        {
            return None;
        }
        Some(Self {
            spelling: format!("{style} ({personal_name})"),
            bridge: Some(PeerageBridge {
                style: style.to_owned(),
                full_style: format!("{personal_name} of {territory}"),
                personal_name: personal_name.to_owned(),
            }),
        })
    }

    pub(crate) fn spelling(&self) -> &str {
        &self.spelling
    }
    pub(crate) const fn bridge(&self) -> Option<&PeerageBridge> {
        self.bridge.as_ref()
    }
}

impl PeerageBridge {
    pub(crate) fn style(&self) -> &str {
        &self.style
    }
    pub(crate) fn personal_name(&self) -> &str {
        &self.personal_name
    }
    pub(crate) fn styles(&self) -> [&str; 2] {
        [&self.style, &self.full_style]
    }
}

struct ParsedPersonName {
    titles: Vec<PersonTitle>,
    normalized: Option<String>,
    core: String,
}

impl ParsedPersonName {
    fn parse(name: &str) -> Self {
        let mut titles = Vec::new();
        let mut remainder = name;
        while let Some(word) = remainder.split_whitespace().next() {
            let Some(title) = PersonTitle::from_token(word.strip_suffix('.').unwrap_or(word))
            else {
                break;
            };
            titles.push(title);
            remainder = remainder[word.len()..].trim_start();
        }
        let normalized = if titles.is_empty() {
            remainder.to_owned()
        } else {
            unwrap_given_name(remainder).unwrap_or_else(|| remainder.to_owned())
        };
        let tokenized = tokens(name);
        let core = tokenized
            .split_whitespace()
            .skip_while(|word| PersonTitle::from_token(word).is_some())
            .collect::<Vec<_>>()
            .join(" ");
        Self {
            titles,
            normalized: (!normalized.is_empty()).then_some(normalized),
            core,
        }
    }
}

fn unwrap_given_name(name: &str) -> Option<String> {
    let (given, surname) = name.strip_prefix('(')?.split_once(')')?;
    if !surname.starts_with(char::is_whitespace) {
        return None;
    }
    let given = given.trim();
    let surname = surname.trim();
    (name_like(given) && name_like(surname)).then(|| format!("{given} {surname}"))
}

fn name_like(value: &str) -> bool {
    !value.is_empty()
        && value.split_whitespace().all(|word| {
            !matches!(
                word,
                "and"
                    | "or"
                    | "trading"
                    | "as"
                    | "trustee"
                    | "ceo"
                    | "director"
                    | "representative"
                    | "representing"
                    | "behalf"
            ) && word.chars().next().is_some_and(char::is_alphabetic)
                && word.chars().last().is_some_and(char::is_alphabetic)
                && word.chars().all(|c| {
                    c.is_alphabetic()
                        || is_combining_mark(c)
                        || matches!(c, '-' | '\'' | '’' | '‘' | 'ʼ')
                })
        })
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
    /// Recognized leading titles, including titles without a remaining name.
    pub person_titles: Vec<PersonTitle>,
    /// Title-free normalized candidate, retaining name punctuation.
    pub person_name_normalized: Option<String>,
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
        let person = usable.as_deref().map(ParsedPersonName::parse);
        let (person_titles, person_name_normalized, person_core) = person.map_or_else(
            || (Vec::new(), None, None),
            |person| (person.titles, person.normalized, Some(person.core)),
        );
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
            person_titles,
            person_name_normalized,
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
