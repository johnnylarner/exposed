WITH input (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) AS (
    SELECT ('70000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, i,
       'Contract Member ' || i, 1, 'Contract Party', 1, 'Contract constituency', true
FROM generate_series(7001, 7007) i
) INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, membership_kind, membership_from, membership_status_active, membership_status_provenance, commons_imported_at) SELECT input.id::uuid, input.parliament_member_id::integer, input.name, input.party_id, input.party_name, CASE input.latest_house WHEN 1 THEN 'elected_mp' ELSE 'other_lords' END, input.latest_membership_from, CASE WHEN input.latest_house = 1 THEN input.is_current_commons END, CASE WHEN input.latest_house = 1 AND NOT input.is_current_commons THEN 'legacy_former' ELSE 'source' END, now() FROM input;
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES
(7101, '70000000-0000-0000-0000-000000007001', 1, 'Employment', '2026-01-01Z', '2024-07-08T23:59:59.999999Z'),
(7102, '70000000-0000-0000-0000-000000007001', 3, 'Support', '2026-01-01Z', '2024-07-09T00:00:00Z'),
(7103, '70000000-0000-0000-0000-000000007001', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7104, '70000000-0000-0000-0000-000000007001', 987, 'Unknown category', '2026-01-01Z', '2025-01-01Z'),
(7105, '70000000-0000-0000-0000-000000007001', 3, 'Support', '2026-01-01Z', '2025-01-01Z'),
(7106, '70000000-0000-0000-0000-000000007001', 4, 'Gifts', '2026-01-01Z', NULL),
(7107, '70000000-0000-0000-0000-000000007001', 1, 'Employment', '2026-01-01Z', '2025-01-01Z'),
(7108, '70000000-0000-0000-0000-000000007001', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7109, '70000000-0000-0000-0000-000000007001', 3, 'Support', '2026-01-01Z', '2025-01-01Z'),
(7201, '70000000-0000-0000-0000-000000007002', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7401, '70000000-0000-0000-0000-000000007004', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7501, '70000000-0000-0000-0000-000000007005', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7601, '70000000-0000-0000-0000-000000007006', 987, 'Unknown category', '2026-01-01Z', '2023-01-01Z');
INSERT INTO exposed.funding_entries (source_declaration_id, amount, currency) VALUES
(7101, 100, 'GBP'),
(7102, 200, 'GBP'),
(7103, 30.25, 'GBP'),
(7104, -5, 'GBP'),
(7105, .1, 'GBP'),
(7105, .1, 'GBP'),
(7106, 7.55, 'GBP'),
(7107, NULL, 'GBP'),
(7108, 123, NULL),
(7109, 999, 'EUR'),
(7201, 9007199254740993.123456789, 'GBP'),
(7201, 9007199254740993.123456789, 'GBP'),
(7201, -18014398509481986.246913578, 'GBP'),
(7401, 999, 'EUR'),
(7401, NULL, 'EUR'),
(7501, NULL, 'GBP');

-- Earlier and undated unknown values must not taint a complete current-term zero.
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES
(7701, '70000000-0000-0000-0000-000000007007', 4, 'Gifts', '2026-01-01Z', '2025-01-01Z'),
(7702, '70000000-0000-0000-0000-000000007007', 1, 'Employment', '2026-01-01Z', '2024-07-08Z'),
(7703, '70000000-0000-0000-0000-000000007007', 987, 'Unknown category', '2026-01-01Z', NULL);
INSERT INTO exposed.funding_entries (source_declaration_id, amount, currency) VALUES
(7701, 0, 'GBP'),
(7702, NULL, 'GBP'),
(7703, 8, NULL);
