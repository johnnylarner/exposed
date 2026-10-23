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
('10000000-0000-0000-0000-000000000006', 'Family individual', 'Individual');
INSERT INTO exposed.funder_alias_groups (funder_id, group_id) VALUES
('10000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000001'),
('10000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001'),
('10000000-0000-0000-0000-000000000004', '10000000-0000-0000-0000-000000000004'),
('10000000-0000-0000-0000-000000000005', '10000000-0000-0000-0000-000000000004');
INSERT INTO exposed.funder_group_names (group_id, preferred_name, family_key) VALUES ('10000000-0000-0000-0000-000000000001', 'Preferred company', 'company');
INSERT INTO exposed.funder_family_links (left_funder_id, right_funder_id, family_key, left_name, right_name)
VALUES ('10000000-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000006', 'family', 'Individual', 'Family individual');
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES
(100, '00000000-0000-0000-0000-000000000001', 3, 'Support including loans', now(), '2024-07-09T00:00:00Z'),
(101, '00000000-0000-0000-0000-000000000001', 987, 'Unknown category', now(), '2025-01-01'),
(102, '00000000-0000-0000-0000-000000000001', 3, 'Before cutoff', now(), '2024-07-08T23:59:59.999999Z'),
(103, '00000000-0000-0000-0000-000000000001', 3, 'Undated', now(), NULL),
(200, '00000000-0000-0000-0000-000000000002', 3, 'Support including loans', now(), '2025-01-01'),
(201, '00000000-0000-0000-0000-000000000002', 3, 'Empty declaration', now(), '2025-01-01'),
(300, '00000000-0000-0000-0000-000000000003', 3, 'Missing amount', now(), '2025-01-01'),
(400, '00000000-0000-0000-0000-000000000004', 3, 'Empty declaration', now(), '2025-01-01'),
(500, '00000000-0000-0000-0000-000000000005', 3, 'Foreign declaration', now(), '2025-01-01'),
(600, '00000000-0000-0000-0000-000000000006', 3, 'Former MP declaration', now(), '2025-01-01');
INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency, payment_type) VALUES
(100, '10000000-0000-0000-0000-000000000001', 9007199254740993.123456789, 'GBP', 'Loan'),
(100, '10000000-0000-0000-0000-000000000002', -9007199254740993.023456789, E'\tGBP\n', 'Loan'),
(100, '10000000-0000-0000-0000-000000000002', 1.23456789123456789, 'GBP', 'Support'),
(100, '10000000-0000-0000-0000-000000000002', 1.23456789123456789, 'GBP', 'Support'),
(101, '10000000-0000-0000-0000-000000000001', 50, 'GBP', NULL),
(101, NULL, NULL, 'GBP', NULL),
(101, NULL, 25, U&'\00a0\feff', NULL),
(102, '10000000-0000-0000-0000-000000000001', 1000000, 'GBP', NULL),
(103, '10000000-0000-0000-0000-000000000001', 1000000, 'GBP', NULL),
(200, '10000000-0000-0000-0000-000000000003', 0, 'GBP', NULL),
(200, '10000000-0000-0000-0000-000000000006', -2, 'GBP', NULL),
(200, '10000000-0000-0000-0000-000000000005', 8, 'GBP', NULL),
(200, NULL, 4, 'GBP', NULL),
(300, '10000000-0000-0000-0000-000000000001', NULL, 'GBP', NULL),
(500, '10000000-0000-0000-0000-000000000003', NULL, 'EUR', NULL),
(600, NULL, -1, 'GBP', NULL);
