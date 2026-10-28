INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
SELECT ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, i, 'Member ' || i,
       i, 'Party ' || i, 1, 'Place ' || i, true FROM generate_series(100, 122) i;
INSERT INTO exposed.funders (id, funder_name, funder_kind)
SELECT ('20000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 'Individual ' || i, 'Individual' FROM generate_series(100, 122) i
UNION ALL
SELECT ('30000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 'Company ' || i, 'Company' FROM generate_series(100, 122) i;
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date)
SELECT i * 100, ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 3, 'Support', now(), '2024-07-09T00:00:00Z'::timestamptz FROM generate_series(100, 122) i;
INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency)
SELECT i * 100, ('20000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 1000 - i, 'GBP' FROM generate_series(100, 122) i
UNION ALL
SELECT i * 100, ('30000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 1000 - i, 'GBP' FROM generate_series(100, 122) i;
