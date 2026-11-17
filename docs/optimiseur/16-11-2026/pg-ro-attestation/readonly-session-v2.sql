SELECT json_build_object(
 'transaction_read_only',current_setting('transaction_read_only')='on',
 'default_transaction_read_only',current_setting('default_transaction_read_only')='on',
 'superuser',r.rolsuper,'createdb',r.rolcreatedb,'createrole',r.rolcreaterole,
 'replication',r.rolreplication,'bypassrls',r.rolbypassrls,
 'database_owner',d.datdba=r.oid,
 'user',current_user,'database',current_database(),'backend_pid',pg_backend_pid()
)
FROM pg_roles r JOIN pg_database d ON d.datname=current_database()
WHERE r.rolname=current_user;
