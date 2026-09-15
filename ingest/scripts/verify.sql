\set ON_ERROR_STOP on

-- Cohort totals. Counts are observations, not hard-coded seat-count assertions.
SELECT t.term_start, t.term_end,
       count(DISTINCT m.id) AS members_who_served,
       count(DISTINCT m.id) FILTER (WHERE m.is_current_commons) AS current_commons,
       count(DISTINCT m.id) FILTER (WHERE NOT m.is_current_commons) AS former_commons
FROM exposed.parliament_terms t
JOIN exposed.member_terms mt ON mt.term_id = t.id
JOIN exposed.members m ON m.id = mt.member_id
GROUP BY t.term_start, t.term_end;

-- Both should be zero.
SELECT count(*) AS invalid_service_dates
FROM exposed.member_terms mt
JOIN exposed.parliament_terms t ON t.id = mt.term_id
WHERE mt.served_from <> greatest(mt.source_start_date, t.term_start)
   OR mt.served_until < mt.served_from;

SELECT count(*) AS duplicate_member_ids
FROM (
    SELECT parliament_member_id FROM exposed.members
    GROUP BY parliament_member_id HAVING count(*) > 1
) duplicates;

-- Former MPs remain linked, with their actual service end dates.
SELECT m.parliament_member_id, m.name, mt.served_from, mt.served_until
FROM exposed.members m
JOIN exposed.member_terms mt ON mt.member_id = m.id
WHERE NOT m.is_current_commons
ORDER BY m.name, mt.served_from;

