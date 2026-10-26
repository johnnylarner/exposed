INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
SELECT ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, i, 'Member ' || i,
       CASE WHEN i = 2 THEN 2 ELSE 1 END, CASE WHEN i = 2 THEN 'Latest Party' ELSE 'Party A' END,
       1, 'Constituency ' || i, i <> 6 FROM generate_series(1, 6) i;
INSERT INTO exposed.funders (id, funder_name, funder_kind) VALUES
('10000000-0000-0000-0000-000000000001', 'Company root', 'Company'),
('10000000-0000-0000-0000-000000000002', 'Company alias with individual kind', 'Individual'),
('10000000-0000-0000-0000-000000000003', 'Individual', 'Individual'),
('10000000-0000-0000-0000-000000000004', 'Trust root', 'Trust'),
('10000000-0000-0000-0000-000000000005', 'Trust alias with company kind', 'Company'),
('10000000-0000-0000-0000-000000000006', 'Second individual', 'Individual'),
('10000000-0000-0000-0000-000000000007', 'Unclassified funder', NULL);
INSERT INTO exposed.funder_alias_groups (funder_id, group_id) VALUES
('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000001'),
('10000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001'),
('10000000-0000-0000-0000-000000000004', '10000000-0000-0000-0000-000000000004'),
('10000000-0000-0000-0000-000000000005', '10000000-0000-0000-0000-000000000004');
INSERT INTO exposed.funder_group_names (group_id, preferred_name, family_key) VALUES ('10000000-0000-0000-0000-000000000001', 'Preferred company', 'company');
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES
(100, '00000000-0000-0000-0000-000000000001', 3, 'Support including loans', now(), '2026-10-01T00:00:00Z'),
(101, '00000000-0000-0000-0000-000000000001', 987, 'Unknown category', now(), '2026-10-31T23:59:59.999999Z'),
(102, '00000000-0000-0000-0000-000000000001', 3, 'Previous month', now(), '2026-09-30T23:59:59.999999Z'),
(103, '00000000-0000-0000-0000-000000000001', 3, 'Undated', now(), NULL),
(200, '00000000-0000-0000-0000-000000000002', 3, 'Support including loans', now(), '2026-10-15T12:00:00Z'),
(201, '00000000-0000-0000-0000-000000000002', 3, 'Empty declaration', now(), '2026-10-15T12:00:00Z'),
(300, '00000000-0000-0000-0000-000000000003', 3, 'Company declaration', now(), '2026-10-15T12:00:00Z'),
(400, '00000000-0000-0000-0000-000000000004', 3, 'Empty declaration', now(), '2026-10-15T12:00:00Z'),
(500, '00000000-0000-0000-0000-000000000005', 3, 'Individual declaration', now(), '2026-10-15T12:00:00Z'),
(600, '00000000-0000-0000-0000-000000000006', 3, 'Former MP declaration', now(), '2026-10-15T12:00:00Z');
INSERT INTO exposed.funding_entries (source_declaration_id, funder_id) VALUES
(100, '10000000-0000-0000-0000-000000000001'),
(100, '10000000-0000-0000-0000-000000000002'),
(100, '10000000-0000-0000-0000-000000000002'),
(100, '10000000-0000-0000-0000-000000000002'),
(101, '10000000-0000-0000-0000-000000000001'),
(101, NULL),
(102, '10000000-0000-0000-0000-000000000001'),
(103, '10000000-0000-0000-0000-000000000001'),
(200, '10000000-0000-0000-0000-000000000003'),
(200, '10000000-0000-0000-0000-000000000006'),
(200, '10000000-0000-0000-0000-000000000005'),
(200, '10000000-0000-0000-0000-000000000007'),
(300, '10000000-0000-0000-0000-000000000001'),
(500, '10000000-0000-0000-0000-000000000003'),
(600, NULL);
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date)
VALUES (700, '00000000-0000-0000-0000-000000000001', 987, 'Next month', now(), '2026-11-01T00:00:00Z');
