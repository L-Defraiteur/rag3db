SELECT json_build_object(
 'database',current_database(),'database_oid',d.oid::bigint,
 'owner',o.rolname,'owner_oid',o.oid::bigint,
 'observer','g_pg_observer','observer_oid',r.oid::bigint,
 'superuser',r.rolsuper,'createdb',r.rolcreatedb,'createrole',r.rolcreaterole,
 'replication',r.rolreplication,'bypassrls',r.rolbypassrls,'inherit',r.rolinherit,
 'owner_is_observer',d.datdba=r.oid,
 'memberships',(SELECT coalesce(json_agg(roleid::bigint),'[]'::json) FROM pg_auth_members WHERE member=r.oid),
 'role_settings',r.rolconfig,
 'can_connect',has_database_privilege(r.oid,d.oid,'CONNECT'),
 'can_create',has_database_privilege(r.oid,d.oid,'CREATE'),
 'can_temp',has_database_privilege(r.oid,d.oid,'TEMP'),
 'schema_create',has_schema_privilege(r.oid,'fixture','CREATE'),
 'public_create',has_schema_privilege(r.oid,'public','CREATE'),
 'table_select',has_table_privilege(r.oid,'fixture.samples','SELECT'),
 'table_insert',has_table_privilege(r.oid,'fixture.samples','INSERT'),
 'table_update',has_table_privilege(r.oid,'fixture.samples','UPDATE'),
 'table_delete',has_table_privilege(r.oid,'fixture.samples','DELETE'),
 'server_version_num',current_setting('server_version_num')::integer,
 'postmaster_start',pg_postmaster_start_time()::text,
 'count',(SELECT count(*) FROM fixture.samples),
 'weight_sum',(SELECT sum(weight) FROM fixture.samples),
 'rows',(SELECT json_agg(s ORDER BY id) FROM fixture.samples s)
)
FROM pg_database d JOIN pg_roles o ON o.oid=d.datdba
JOIN pg_roles r ON r.rolname='g_pg_observer'
WHERE d.datname=current_database();
