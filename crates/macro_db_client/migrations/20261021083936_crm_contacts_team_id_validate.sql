SET LOCAL lock_timeout = '5s';

-- Takes SHARE UPDATE EXCLUSIVE: reads and writes continue during the scan.
ALTER TABLE crm_contacts VALIDATE CONSTRAINT crm_contacts_team_id_not_null;
