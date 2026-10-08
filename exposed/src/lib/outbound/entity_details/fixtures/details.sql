INSERT INTO exposed.members (id, parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
SELECT ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, i,
       CASE i WHEN 1 THEN 'Ava Example' WHEN 2 THEN 'Bea Example' WHEN 3 THEN 'Empty Member' ELSE 'Recipient ' || i END,
       CASE WHEN i = 2 THEN 2 ELSE 1 END, CASE WHEN i = 2 THEN 'Party B' ELSE 'Party A' END,
       1, 'Example constituency', true FROM generate_series(1, 15) i;
INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons)
VALUES (16, 'Former MP Example', 1, 'Party A', 2, 'Life peer', false);
INSERT INTO exposed.funders (id, funder_name, funder_kind, company_number) VALUES
('10000000-0000-0000-0000-000000000001', 'Exact Funder', 'Company', '00123456'),
('10000000-0000-0000-0000-000000000002', 'Empty Funder', NULL, NULL);
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date)
SELECT i, '00000000-0000-0000-0000-000000000001', 1, 'Support', now(),
       CASE WHEN i > 120 THEN NULL ELSE '2026-01-01'::timestamptz + (i - 100) * interval '1 day' END
FROM generate_series(100, 122) i;
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date) VALUES
(200, '00000000-0000-0000-0000-000000000002', 2, 'Visits', now(), NULL),
(201, '00000000-0000-0000-0000-000000000002', 1, 'Support', now(), '2026-02-01'),
(202, '00000000-0000-0000-0000-000000000002', 1, 'Support', now(), '2026-02-01');
INSERT INTO exposed.declarations (source_declaration_id, member_id, category_id, category_name, fetched_at, registration_date)
SELECT 300 + i, ('00000000-0000-0000-0000-' || lpad(i::text, 12, '0'))::uuid, 1, 'Support', now(), NULL FROM generate_series(4, 15) i;
INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency, payment_type) VALUES
(120, '10000000-0000-0000-0000-000000000001', 9007199254740993.123456789, 'GBP', 'Cash'),
(120, '10000000-0000-0000-0000-000000000001', 9007199254740993.123456789, 'GBP', 'Cash'),
(120, '10000000-0000-0000-0000-000000000001', NULL, 'GBP', NULL),
(120, '10000000-0000-0000-0000-000000000001', 999, NULL, NULL),
(120, '10000000-0000-0000-0000-000000000001', NULL, NULL, NULL),
(120, '10000000-0000-0000-0000-000000000001', 888, '  ', NULL),
(120, '10000000-0000-0000-0000-000000000001', 777, E'\t\n', NULL),
(120, '10000000-0000-0000-0000-000000000001', 666, U&'\00A0\FEFF', NULL),
(120, '10000000-0000-0000-0000-000000000001', 100, E'\tUSD\n', NULL),
(120, NULL, NULL, NULL, NULL),
(200, '10000000-0000-0000-0000-000000000001', -5, 'GBP', NULL),
(200, '10000000-0000-0000-0000-000000000001', 0, 'GBP', NULL),
(200, '10000000-0000-0000-0000-000000000001', 500, 'USD', NULL),
(200, '10000000-0000-0000-0000-000000000001', NULL, 'EUR', NULL);
INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount, currency)
SELECT 300 + i, '10000000-0000-0000-0000-000000000001', 1, 'GBP' FROM generate_series(4, 15) i;
