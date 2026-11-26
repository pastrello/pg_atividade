SELECT current_setting('server_version') AS version,
       current_setting('server_version_num')::int AS version_num,
       current_user::text AS usr,
       (SELECT rolsuper FROM pg_roles WHERE rolname = current_user) AS superuser,
       pg_has_role(current_user, 'pg_read_all_stats', 'USAGE') AS read_all_stats,
       pg_has_role(current_user, 'pg_signal_backend', 'USAGE') AS can_signal,
       pg_is_in_recovery() AS replica,
       concat_ws('|',pg_postmaster_start_time()::text,inet_server_addr()::text,inet_server_port()::text) AS instance;
