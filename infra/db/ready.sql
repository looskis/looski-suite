-- The upstream entrypoint skips initialization after a partial init failure.
-- Write the sentinel only after all upstream bootstrap migrations succeed.
\! touch /var/lib/postgresql/data/.looski-bootstrap-complete
