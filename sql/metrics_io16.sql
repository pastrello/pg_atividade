SELECT concat_ws('|',backend_type,object,context) AS key, coalesce(stats_reset::text,'') AS reset,
       (coalesce(reads,0)::numeric * op_bytes)::text AS a,
       ((coalesce(writes,0)::numeric+coalesce(extends,0)::numeric)*op_bytes)::text AS b,
       coalesce(reads,0)::text AS c, (coalesce(writes,0)::numeric+coalesce(extends,0))::text AS d,
       '0'::text AS e, '0'::text AS f, '0'::text AS g
FROM pg_catalog.pg_stat_io WHERE object IN ('relation','temp relation')
ORDER BY backend_type,object,context;
