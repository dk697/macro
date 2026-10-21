SET LOCAL lock_timeout = '5s';

-- Person contacts: a CRM contact may exist without a company (a client on
-- gmail.com, a sole trader). The contact now carries its own team so team
-- scoping no longer has to go through crm_companies.
ALTER TABLE crm_contacts
    ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES team (id) ON DELETE CASCADE;

UPDATE crm_contacts ct
SET team_id = co.team_id
FROM crm_companies co
WHERE co.id = ct.company_id
  AND ct.team_id IS NULL;

-- Deployed writers insert company contacts without team_id; fill it from
-- the company so the column can be NOT NULL through the rolling deploy.
CREATE OR REPLACE FUNCTION crm_contacts_fill_team_id() RETURNS trigger AS $$
BEGIN
    IF NEW.team_id IS NULL AND NEW.company_id IS NOT NULL THEN
        SELECT team_id INTO NEW.team_id FROM crm_companies WHERE id = NEW.company_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS crm_contacts_fill_team_id ON crm_contacts;
CREATE TRIGGER crm_contacts_fill_team_id
    BEFORE INSERT ON crm_contacts
    FOR EACH ROW EXECUTE FUNCTION crm_contacts_fill_team_id();

ALTER TABLE crm_contacts ALTER COLUMN team_id SET NOT NULL;

ALTER TABLE crm_contacts ALTER COLUMN company_id DROP NOT NULL;
