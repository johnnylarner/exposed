INSERT INTO exposed.declarations (  
    source_declaration_id ,
    member_id ,
    category_id ,
    category_name ,
    fetched_at ,
    registration_date 
) VALUES 
	(
	    1,
	    (SELECT id FROM members WHERE name='John McDonnell'),
	    12,
	    'Employment and earnings',
	    now(),
	    now()
	),
	(
	    2,
	    (SELECT id FROM members WHERE name='John Humphries'),
	    10,
	    'Family members employed',
	    now(),
	    now()
	);

INSERT INTO exposed.funders (funder_name, funder_kind, company_number)
VALUES
    ('McDonald''s', 'Company', '123'),
    ('Aaron Banks', 'Individual', NULL);

INSERT INTO exposed.funding_entries (
    source_declaration_id, funder_id, amount, currency, payment_type
) VALUES
    (1, (SELECT id FROM exposed.funders WHERE funder_name = 'McDonald''s'),
     10, 'GBP', 'Monetary'),
    (2, (SELECT id FROM exposed.funders WHERE funder_name = 'Aaron Banks'),
     100, 'GBP', 'In Kind');
