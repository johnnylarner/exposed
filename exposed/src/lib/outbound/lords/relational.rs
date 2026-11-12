use crate::domain::models::{
    lords::{
        LordProfile, LordsCategory, LordsCategoryId, LordsError, RegisteredInterest,
        RegisteredInterestId, SourceTimestamp,
    },
    parliament_member::MemberId,
};

pub(super) struct ProfileRow {
    pub member_id: uuid::Uuid,
    pub parliament_member_id: i32,
    pub name: String,
    pub party_name: Option<String>,
    pub membership_from: Option<String>,
    pub membership_status: Option<String>,
    pub membership_end_date: Option<String>,
    pub captured_at: time::OffsetDateTime,
    pub similarity: Option<f32>,
}

impl ProfileRow {
    pub fn profile(self) -> Result<LordProfile, LordsError> {
        Ok(LordProfile {
            id: MemberId::new(u32::try_from(self.parliament_member_id).map_err(stored)?)
                .map_err(stored)?,
            name: self.name,
            party_name: self.party_name,
            membership_from: self.membership_from,
            membership_status: self.membership_status,
            membership_end_date: timestamp(self.membership_end_date)?,
        })
    }
}

pub(super) struct CategoryRow {
    pub category_position: i64,
    pub source_id: i64,
    pub name: Option<String>,
    pub sort_order: i32,
}

pub(super) struct InterestRow {
    pub category_position: i64,
    pub node_position: i64,
    pub parent_position: Option<i64>,
    pub sibling_position: i64,
    pub source_id: i64,
    pub interest_text: Option<String>,
    pub created_when: Option<String>,
    pub last_amended_when: Option<String>,
    pub deleted_when: Option<String>,
    pub is_correction: bool,
}

pub(super) fn stored(error: impl std::fmt::Display) -> LordsError {
    LordsError::Database(error.to_string())
}

fn timestamp(value: Option<String>) -> Result<Option<SourceTimestamp>, LordsError> {
    value
        .map(SourceTimestamp::try_from)
        .transpose()
        .map_err(stored)
}

pub(super) fn position(value: usize) -> Result<i64, LordsError> {
    i64::try_from(value).map_err(stored)
}

pub(super) fn reconstruct(
    categories: Vec<CategoryRow>,
    interests: Vec<InterestRow>,
) -> Result<Vec<LordsCategory>, LordsError> {
    let mut nodes = interests.into_iter().peekable();
    let mut result = Vec::with_capacity(categories.len());
    for (index, category) in categories.into_iter().enumerate() {
        let category_position = position(index)?;
        if category.category_position != category_position {
            return Err(stored("non-contiguous Lords category positions"));
        }
        let source_id =
            LordsCategoryId::try_from(u32::try_from(category.source_id).map_err(stored)?)
                .map_err(stored)?;
        let mut next_node = 0;
        let interests = children(&mut nodes, category_position, None, &mut next_node)?;
        if nodes
            .peek()
            .is_some_and(|node| node.category_position <= category_position)
        {
            return Err(stored("orphan or misordered Lords interest"));
        }
        result.push(LordsCategory {
            source_id,
            name: category.name,
            sort_order: category.sort_order,
            placement: source_id.placement(),
            interests,
        });
    }
    if nodes.next().is_some() {
        return Err(stored("Lords interest without category"));
    }
    Ok(result)
}

fn children(
    nodes: &mut std::iter::Peekable<std::vec::IntoIter<InterestRow>>,
    category: i64,
    parent: Option<i64>,
    next_node: &mut i64,
) -> Result<Vec<RegisteredInterest>, LordsError> {
    let mut result = Vec::new();
    while nodes
        .peek()
        .is_some_and(|node| node.category_position == category && node.parent_position == parent)
    {
        let row = nodes.next().expect("peeked node");
        if row.node_position != *next_node || row.sibling_position != position(result.len())? {
            return Err(stored("non-contiguous Lords interest positions"));
        }
        *next_node += 1;
        result.push(RegisteredInterest {
            source_id: RegisteredInterestId::try_from(
                u32::try_from(row.source_id).map_err(stored)?,
            )
            .map_err(stored)?,
            text: row.interest_text,
            created_when: timestamp(row.created_when)?,
            last_amended_when: timestamp(row.last_amended_when)?,
            deleted_when: timestamp(row.deleted_when)?,
            is_correction: row.is_correction,
            children: children(nodes, category, Some(row.node_position), next_node)?,
        });
    }
    Ok(result)
}
