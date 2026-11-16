SELECT json_build_object(
  'backend_pid', current_setting('g_pg.expected_backend_pid')::integer,
  'application_name', current_setting('g_pg.expected_application_name'),
  'backend_absent', NOT EXISTS (
    SELECT 1 FROM pg_stat_activity
    WHERE pid = current_setting('g_pg.expected_backend_pid')::integer
       OR application_name = current_setting('g_pg.expected_application_name')
  ),
  'observer_backend_pid', pg_backend_pid(),
  'observer_application_name', current_setting('application_name')
);
