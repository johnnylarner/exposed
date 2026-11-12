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
), identities AS (
    INSERT INTO exposed.members (parliament_member_id)
    SELECT input.parliament_member_id::integer FROM input RETURNING id, parliament_member_id
)
INSERT INTO exposed.member_profiles (member_id, chamber, name, party_id, party_name, latest_house, latest_membership_from, is_current_commons, created_at, updated_at)
SELECT identities.id, 'commons', input.name, input.party_id, input.party_name, input.latest_house::smallint, input.latest_membership_from, input.is_current_commons, now(), now()
FROM input JOIN identities ON identities.parliament_member_id = input.parliament_member_id;
