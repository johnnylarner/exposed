INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, membership_kind, membership_from, membership_end_date)
    VALUES
	(
		1,
		'John McDonnell',
		15,
		'Labour',
		(SELECT id FROM exposed.membership_kinds WHERE name = 'Elected MP'),
		'Poole',
		NULL
	),
	(
		2,
		'John Humphries',
		4,
		'Conservative',
		(SELECT id FROM exposed.membership_kinds WHERE name = 'Elected MP'),
		'Dumfries and Galloway',
		NULL
	);
