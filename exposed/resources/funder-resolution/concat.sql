CREATE TABLE __splink__df_concat_with_tf AS
WITH

__splink__df_concat as (
            select "key", "name", "address", "blocking_keys", "needs_resolution"
            , random() as __splink_salt
            from profiles
            )
select * from __splink__df_concat;
