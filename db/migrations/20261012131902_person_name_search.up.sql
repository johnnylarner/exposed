CREATE EXTENSION fuzzystrmatch WITH SCHEMA exposed;

CREATE FUNCTION exposed.person_name_words(value text) RETURNS text[]
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT COALESCE(array_agg(word), ARRAY[]::text[])
    FROM regexp_split_to_table(lower(value), '[^[:alnum:]]+') AS word
    WHERE word <> ''
$$;

CREATE FUNCTION exposed.person_name_codes(value text) RETURNS text[]
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT COALESCE(array_agg(exposed.soundex(word)), ARRAY[]::text[])
    FROM unnest(exposed.person_name_words(value)) AS word
$$;

CREATE FUNCTION exposed.person_name_distance(query text, candidate text) RETURNS integer
LANGUAGE plpgsql IMMUTABLE STRICT PARALLEL SAFE AS $$
DECLARE
    query_words text[] := exposed.person_name_words(query);
    candidate_words text[] := exposed.person_name_words(candidate);
    query_count integer := cardinality(query_words);
    candidate_count integer := cardinality(candidate_words);
    invalid_cost integer := query_count + 1;
    costs integer[][];
    query_potential integer[];
    candidate_potential integer[];
    assigned_query integer[];
    predecessor integer[];
    minimum_cost integer[];
    visited boolean[];
    query_position integer;
    candidate_position integer;
    current_candidate integer;
    next_candidate integer;
    current_query integer;
    reduced_cost integer;
    potential_change integer;
    distance integer;
    total_distance integer := 0;
BEGIN
    IF query_count = 0 OR query_count > candidate_count
       OR EXISTS (SELECT FROM unnest(query_words) word WHERE char_length(word) > 255) THEN
        RETURN NULL;
    END IF;

    -- Any invalid edge costs more than an entire valid assignment.
    costs := array_fill(invalid_cost, ARRAY[query_count, candidate_count]);
    FOR query_position IN 1..query_count LOOP
        FOR candidate_position IN 1..candidate_count LOOP
            IF char_length(candidate_words[candidate_position]) <= 255
               AND exposed.soundex(query_words[query_position]) = exposed.soundex(candidate_words[candidate_position]) THEN
                distance := exposed.levenshtein_less_equal(query_words[query_position], candidate_words[candidate_position], 1);
                IF distance <= 1 THEN
                    costs[query_position][candidate_position] := distance;
                END IF;
            END IF;
        END LOOP;
    END LOOP;

    query_potential := array_fill(0, ARRAY[query_count + 1], ARRAY[0]);
    candidate_potential := array_fill(0, ARRAY[candidate_count + 1], ARRAY[0]);
    assigned_query := array_fill(0, ARRAY[candidate_count + 1], ARRAY[0]);
    predecessor := array_fill(0, ARRAY[candidate_count + 1], ARRAY[0]);

    -- Hungarian assignment uses position zero as the augmenting path's root.
    FOR query_position IN 1..query_count LOOP
        assigned_query[0] := query_position;
        current_candidate := 0;
        minimum_cost := array_fill(NULL::integer, ARRAY[candidate_count + 1], ARRAY[0]);
        visited := array_fill(false, ARRAY[candidate_count + 1], ARRAY[0]);
        LOOP
            visited[current_candidate] := true;
            current_query := assigned_query[current_candidate];
            potential_change := NULL;
            FOR candidate_position IN 1..candidate_count LOOP
                IF NOT visited[candidate_position] THEN
                    reduced_cost := costs[current_query][candidate_position]
                        - query_potential[current_query] - candidate_potential[candidate_position];
                    IF minimum_cost[candidate_position] IS NULL OR reduced_cost < minimum_cost[candidate_position] THEN
                        minimum_cost[candidate_position] := reduced_cost;
                        predecessor[candidate_position] := current_candidate;
                    END IF;
                    IF potential_change IS NULL OR minimum_cost[candidate_position] < potential_change THEN
                        potential_change := minimum_cost[candidate_position];
                        next_candidate := candidate_position;
                    END IF;
                END IF;
            END LOOP;
            FOR candidate_position IN 0..candidate_count LOOP
                IF visited[candidate_position] THEN
                    query_potential[assigned_query[candidate_position]] :=
                        query_potential[assigned_query[candidate_position]] + potential_change;
                    candidate_potential[candidate_position] := candidate_potential[candidate_position] - potential_change;
                ELSE
                    minimum_cost[candidate_position] := minimum_cost[candidate_position] - potential_change;
                END IF;
            END LOOP;
            current_candidate := next_candidate;
            EXIT WHEN assigned_query[current_candidate] = 0;
        END LOOP;
        LOOP
            next_candidate := predecessor[current_candidate];
            assigned_query[current_candidate] := assigned_query[next_candidate];
            current_candidate := next_candidate;
            EXIT WHEN current_candidate = 0;
        END LOOP;
    END LOOP;

    FOR candidate_position IN 1..candidate_count LOOP
        IF assigned_query[candidate_position] <> 0 THEN
            distance := costs[assigned_query[candidate_position]][candidate_position];
            IF distance = invalid_cost THEN
                RETURN NULL;
            END IF;
            total_distance := total_distance + distance;
        END IF;
    END LOOP;
    RETURN total_distance;
END
$$;

CREATE INDEX members_person_name_codes ON exposed.members USING gin (exposed.person_name_codes(name));
CREATE INDEX funders_person_name_codes ON exposed.funders USING gin (exposed.person_name_codes(funder_name));
CREATE INDEX funder_aliases_person_name_codes ON exposed.funder_aliases USING gin (exposed.person_name_codes(funder_alias));
