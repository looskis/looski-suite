-- SPDX-License-Identifier: AGPL-3.0-only
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
\getenv runtime_password LOOSKI_RUNTIME_PASSWORD
CREATE ROLE looski_runtime LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD :'runtime_password';
ALTER ROLE looski_runtime SET statement_timeout = '5s';
ALTER ROLE looski_runtime SET search_path = '';
GRANT CONNECT ON DATABASE postgres TO looski_runtime;
