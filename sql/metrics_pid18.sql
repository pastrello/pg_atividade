WITH targets AS MATERIALIZED (
    SELECT pid, backend_start FROM pg_catalog.pg_stat_activity
    WHERE backend_type='client backend' AND state='active' AND backend_start IS NOT NULL
      AND pid<>pg_backend_pid()
    ORDER BY query_start NULLS LAST,pid LIMIT 128
)
SELECT t.pid::text||'|'||t.backend_start::text AS key,
       coalesce(max(i.stats_reset)::text,'') AS reset,
       sum(coalesce(i.read_bytes,0))::text AS a,
       sum(coalesce(i.write_bytes,0)+coalesce(i.extend_bytes,0))::text AS b,
       '0'::text AS c, '0'::text AS d, '0'::text AS e, '0'::text AS f, '0'::text AS g
FROM targets t CROSS JOIN LATERAL pg_catalog.pg_stat_get_backend_io(t.pid) i
WHERE i.object IN ('relation','temp relation')
GROUP BY t.pid,t.backend_start;
