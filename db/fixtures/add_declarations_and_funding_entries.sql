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

INSERT INTO exposed.funding_entries (  
    source_declaration_id,
    funder,
    amount,
    currency,
    payment_type,
    donor_status,
    company_number
) VALUES 
	(
	    1,
	    E'McDonald\'s',
	    10,
	    'GBP',
	    'Monetary',
	    'Company',
	    '123'
	),
	(
	    2,
	    'Aaron Banks',
	    100,
	    'GBP',
	    'In Kind',
	    'Individual',
	    NULL
	);
