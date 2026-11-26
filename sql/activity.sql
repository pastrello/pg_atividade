SELECT pid, COALESCE(datname::text,'') AS db,
       COALESCE(usename::text,'') AS usr,
       COALESCE(application_name,'') AS app,
       COALESCE(host(client_addr),'') AS ip,
       COALESCE(state,'') AS state,
       COALESCE(wait_event_type,'') AS wait_type,
       COALESCE(wait_event,'') AS wait_event,
       COALESCE(backend_type,'') AS backend_type,
       COALESCE(backend_start::text,'') AS started, COALESCE(query_start::text,'') AS query_started,
       CASE WHEN state = 'active' THEN EXTRACT(epoch FROM clock_timestamp()-query_start)::float8 END AS query_age,
       EXTRACT(epoch FROM clock_timestamp()-xact_start)::float8 AS xact_age,
       EXTRACT(epoch FROM clock_timestamp()-state_change)::float8 AS state_age,
       COALESCE(left(query,8192),'') AS query,
       CASE WHEN wait_event_type = 'Lock' THEN pg_blocking_pids(pid) ELSE ARRAY[]::integer[] END AS blockers
FROM pg_stat_activity
WHERE pid <> pg_backend_pid()
ORDER BY (wait_event_type = 'Lock') DESC NULLS LAST,
         (state = 'active') DESC NULLS LAST, xact_start NULLS LAST, pid
LIMIT 5001;
