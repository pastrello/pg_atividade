SELECT datid::text AS key, coalesce(stats_reset::text,'') AS reset,
       xact_commit::text AS a, xact_rollback::text AS b,
       (blks_read::numeric * current_setting('block_size')::numeric)::text AS c,
       blks_hit::text AS d, blks_read::text AS e, temp_bytes::text AS f,
       deadlocks::text AS g
FROM pg_catalog.pg_stat_database WHERE datid <> 0 ORDER BY datid;
