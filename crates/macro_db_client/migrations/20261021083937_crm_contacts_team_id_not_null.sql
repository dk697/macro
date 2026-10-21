SET LOCAL lock_timeout = '5s';

-- The validated CHECK proves no NULLs, so Postgres skips the scan.
ALTER TABLE crm_contacts ALTER COLUMN team_id SET NOT NULL;

ALTER TABLE crm_contacts DROP CONSTRAINT crm_contacts_team_id_not_null;
