-- no-transaction
-- Keep this a single statement: SQLx batches would create an implicit transaction.
CREATE INDEX CONCURRENTLY IF NOT EXISTS crm_contacts_team_id_idx ON crm_contacts (team_id);
