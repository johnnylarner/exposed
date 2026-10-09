CREATE TABLE __splink__df_predict AS
WITH

blocked_with_cols as (
    select "l"."key" AS "key_l",
"r"."key" AS "key_r",
"l"."name" AS "name_l",
"r"."name" AS "name_r",
"l"."address" AS "address_l",
"r"."address" AS "address_r",
"l"."needs_resolution" AS "needs_resolution_l",
"r"."needs_resolution" AS "needs_resolution_r",
"l"."blocking_keys" AS "blocking_keys_l",
"r"."blocking_keys" AS "blocking_keys_r", b.match_key
    from __splink__blocked_id_pairs as b
    inner join __splink__df_concat_with_tf as l
    on l."key" = b.join_key_l
    inner join __splink__df_concat_with_tf as r
    on r."key" = b.join_key_r
    ),

__splink__df_comparison_vectors as (
    select "key_l",
"key_r",
"name_l",
"name_r",
CASE WHEN name_l IS NULL OR name_r IS NULL THEN -1 WHEN name_l = name_r THEN 2 WHEN levenshtein(name_l, name_r) <= 2 THEN 1 ELSE 0 END as gamma_name,
"address_l",
"address_r",
CASE WHEN address_l IS NULL OR address_r IS NULL THEN -1 WHEN address_l = address_r THEN 1 ELSE 0 END as gamma_address,
"needs_resolution_l",
"needs_resolution_r",
"blocking_keys_l",
"blocking_keys_r",
match_key
    from blocked_with_cols
    ),

__splink__df_match_weight_parts as (
    select "key_l","key_r","name_l","name_r",gamma_name,CASE
WHEN
gamma_name = -1
THEN cast(1.0 as float8)

WHEN
gamma_name = 2
THEN cast(949.9999999999999 as float8)

WHEN
gamma_name = 1
THEN cast(4.444444444444445 as float8)

WHEN
gamma_name = 0
THEN cast(0.010101010101010102 as float8)
 END as bf_name ,"address_l","address_r",gamma_address,CASE
WHEN
gamma_address = -1
THEN cast(1.0 as float8)

WHEN
gamma_address = 1
THEN cast(90000.0 as float8)

WHEN
gamma_address = 0
THEN cast(0.1000010000100001 as float8)
 END as bf_address ,"needs_resolution_l","needs_resolution_r","blocking_keys_l","blocking_keys_r",match_key
    from __splink__df_comparison_vectors
    )

    select
    log2(least(greatest(cast(0.00010001000100010001 as float8) * bf_name * bf_address, 1e-300), 1e300)) as match_weight,
    CASE WHEN bf_name = cast('infinity' as float8) OR bf_address = cast('infinity' as float8) THEN 1.0 ELSE (least(greatest(cast(0.00010001000100010001 as float8) * bf_name * bf_address, 1e-300), 1e300))/(1+(least(greatest(cast(0.00010001000100010001 as float8) * bf_name * bf_address, 1e-300), 1e300))) END as match_probability,
    "key_l","key_r","name_l","name_r",gamma_name,bf_name,"address_l","address_r",gamma_address,bf_address,"needs_resolution_l","needs_resolution_r","blocking_keys_l","blocking_keys_r",match_key
    from __splink__df_match_weight_parts;
