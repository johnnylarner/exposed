CREATE TABLE __splink__blocked_id_pairs AS

            select
            '0' as match_key,
            l."key" as join_key_l,
            r."key" as join_key_r
            from __splink__df_concat_with_tf as l
            inner join __splink__df_concat_with_tf as r
            on
            ((l.needs_resolution OR r.needs_resolution) AND (l.name = r.name))
            where l."key" < r."key"

             UNION ALL
            select
            '1' as match_key,
            l."key" as join_key_l,
            r."key" as join_key_r
            from __splink__df_concat_with_tf as l
            inner join __splink__df_concat_with_tf as r
            on
            ((l.needs_resolution OR r.needs_resolution) AND (l.address = r.address))
            where l."key" < r."key"
            AND NOT (coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.name = r.name)),false))
             UNION ALL
            select
            '2' as match_key,
            l."key" as join_key_l,
            r."key" as join_key_r
            from __splink__df_concat_with_tf as l
            inner join __splink__df_concat_with_tf as r
            on
            ((l.needs_resolution OR r.needs_resolution) AND (substr(l.name, 1, 3) = substr(r.name, 1, 3)))
            where l."key" < r."key"
            AND NOT (coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.name = r.name)),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.address = r.address)),false))
             UNION ALL
            select
            '3' as match_key,
            l."key" as join_key_l,
            r."key" as join_key_r
            from __splink__df_concat_with_tf as l
            inner join __splink__df_concat_with_tf as r
            on
            ((l.needs_resolution OR r.needs_resolution) AND (substr(l.name, 1, 1) = substr(r.name, 1, 1) AND substr(regexp_extract(l.name, '[a-z]+$'), 1, 2) = substr(regexp_extract(r.name, '[a-z]+$'), 1, 2) AND regexp_extract(l.name, '[a-z]+$') NOT IN ('limited', 'ltd', 'plc', 'llp', 'company', 'inc', 'and')))
            where l."key" < r."key"
            AND NOT (coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.name = r.name)),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.address = r.address)),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (substr(l.name, 1, 3) = substr(r.name, 1, 3))),false))
             UNION ALL
            select
            '4' as match_key,
            l."key" as join_key_l,
            r."key" as join_key_r
            from __splink__df_concat_with_tf as l
            inner join __splink__df_concat_with_tf as r
            on
            ((l.needs_resolution OR r.needs_resolution) AND list_has_any(l.blocking_keys, r.blocking_keys))
            where l."key" < r."key"
            AND NOT (coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.name = r.name)),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (l.address = r.address)),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (substr(l.name, 1, 3) = substr(r.name, 1, 3))),false) OR coalesce(((l.needs_resolution OR r.needs_resolution) AND (substr(l.name, 1, 1) = substr(r.name, 1, 1) AND substr(regexp_extract(l.name, '[a-z]+$'), 1, 2) = substr(regexp_extract(r.name, '[a-z]+$'), 1, 2) AND regexp_extract(l.name, '[a-z]+$') NOT IN ('limited', 'ltd', 'plc', 'llp', 'company', 'inc', 'and'))),false));
