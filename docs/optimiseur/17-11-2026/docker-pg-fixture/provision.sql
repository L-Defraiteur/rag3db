\set ON_ERROR_STOP on
\set observer_password `cat /run/secrets/observer-password`
CREATE ROLE g_pg_observer LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS NOINHERIT PASSWORD :'observer_password';
ALTER ROLE g_pg_observer SET default_transaction_read_only = on;
REVOKE ALL ON DATABASE :"fixture_db" FROM PUBLIC;
GRANT CONNECT ON DATABASE :"fixture_db" TO g_pg_observer;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
CREATE SCHEMA fixture AUTHORIZATION g_pg_fixture_admin;
CREATE TABLE fixture.samples (id integer PRIMARY KEY, label text NOT NULL, weight integer NOT NULL);
INSERT INTO fixture.samples VALUES (1, 'red', 2), (2, 'blue', 3), (3, 'green', 5);
GRANT USAGE ON SCHEMA fixture TO g_pg_observer;
GRANT SELECT ON fixture.samples TO g_pg_observer;
