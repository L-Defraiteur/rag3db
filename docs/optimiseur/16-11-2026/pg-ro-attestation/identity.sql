SELECT json_build_object(
  'database', current_database(),
  'user', current_user,
  'session_user', session_user,
  'database_oid', d.oid::bigint,
  'user_oid', r.oid::bigint,
  'owner', o.rolname,
  'owner_oid', o.oid::bigint,
  'server_address', inet_server_addr()::text,
  'server_port', inet_server_port(),
  'server_version_num', current_setting('server_version_num')::integer,
  'postmaster_start', pg_postmaster_start_time()::text,
  'backend_pid', pg_backend_pid(),
  'application_name', current_setting('application_name')
)
FROM pg_database AS d
JOIN pg_roles AS r ON r.rolname = current_user
JOIN pg_roles AS o ON o.oid = d.datdba
WHERE d.datname = current_database();
