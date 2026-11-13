WITH input (parliament_member_id, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons) AS (
    VALUES
	(
		1,
		'John McDonnell',
		15,
		'Labour',
		1,
		'Poole',
		true
	),
	(
		2,
		'John Humphries',
		4,
		'Conservative',
		1,
		'Dumfries and Galloway',
		true
	)
) INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, membership_kind, membership_from, membership_status_active, membership_status_provenance, commons_imported_at) SELECT input.parliament_member_id::integer, input.name, input.party_id, input.party_name, CASE input.latest_house WHEN 1 THEN 'elected_mp' ELSE 'other_lords' END, input.latest_membership_from, CASE WHEN input.latest_house = 1 THEN input.is_current_commons END, CASE WHEN input.latest_house = 1 AND NOT input.is_current_commons THEN 'legacy_former' ELSE 'source' END, now() FROM input;
