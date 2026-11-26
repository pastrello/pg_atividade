SELECT 'wal'::text AS key, coalesce(stats_reset::text,'') AS reset,
       wal_bytes::text AS a, '0'::text AS b, '0'::text AS c, '0'::text AS d,
       '0'::text AS e, '0'::text AS f, '0'::text AS g
FROM pg_catalog.pg_stat_wal;
