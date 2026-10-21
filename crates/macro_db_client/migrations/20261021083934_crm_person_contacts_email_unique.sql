-- no-transaction
-- Keep this a single statement: SQLx batches would create an implicit transaction.
-- A person contact owns its address within the team. Company contacts stay
-- unique per (company_id, email).
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS crm_contacts_person_team_email_unique
    ON crm_contacts (team_id, LOWER(email)) WHERE company_id IS NULL;
