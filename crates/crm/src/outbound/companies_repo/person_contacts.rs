//! Person contacts: CRM contacts with no company, owned by their team.

use super::CompaniesRepositoryImpl;
use crate::domain::model::{CrmContact, CrmError};
use chrono::{DateTime, Utc};

impl CompaniesRepositoryImpl {
    /// When the team has CRM enabled and a person contact for
    /// `normalized_email`, merges the interaction range into the contact and
    /// upserts the link's source row, in one statement. Returns whether a
    /// person contact took the address.
    ///
    /// Needs no advisory lock: it never creates a contact, so a concurrent
    /// create or delete either happens before it (and is seen) or after it.
    pub(super) async fn attach_person_contact<'e>(
        executor: impl sqlx::PgExecutor<'e>,
        team_id: &uuid::Uuid,
        link_id: &uuid::Uuid,
        normalized_email: &str,
        first_at: DateTime<Utc>,
        last_at: DateTime<Utc>,
        is_sent: bool,
    ) -> Result<bool, CrmError> {
        sqlx::query_scalar!(
            r#"
            WITH contact AS (
                UPDATE crm_contacts
                SET updated_at = now(),
                    first_interaction = CASE
                        WHEN $5 THEN LEAST(first_interaction, $3)
                        ELSE first_interaction
                    END,
                    last_interaction = GREATEST(last_interaction, $4)
                WHERE team_id = $1
                  AND company_id IS NULL
                  AND LOWER(email) = $2
                  AND EXISTS (
                      SELECT 1 FROM team_crm_settings
                      WHERE team_id = $1 AND crm_enabled
                  )
                RETURNING id
            ),
            source AS (
                INSERT INTO crm_contact_sources (contact_id, link_id)
                SELECT id, $6 FROM contact
                ON CONFLICT (contact_id, link_id) DO NOTHING
            )
            SELECT EXISTS (SELECT 1 FROM contact) AS "attached!"
            "#,
            team_id,
            normalized_email,
            first_at,
            last_at,
            is_sent,
            link_id,
        )
        .fetch_one(executor)
        .await
        .map_err(|e| CrmError::StorageLayerError(e.into()))
    }

    pub(super) async fn insert_person_contact(
        &self,
        team_id: &uuid::Uuid,
        email: &str,
        name: &str,
        now: DateTime<Utc>,
    ) -> Result<CrmContact, CrmError> {
        let normalized_email = email.to_ascii_lowercase();
        let Some((_, email_domain)) = normalized_email.split_once('@') else {
            return Err(CrmError::InvalidRequest("email must contain an '@'".into()));
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| CrmError::StorageLayerError(e.into()))?;

        Self::lock_team_domain(&mut tx, team_id, email_domain).await?;

        if !Self::team_crm_enabled(&mut tx, team_id).await? {
            return Err(CrmError::CrmDisabledForTeam);
        }

        if Self::find_company_by_domain(&mut tx, team_id, email_domain)
            .await?
            .is_some()
        {
            return Err(CrmError::ContactDomainTrackedByCompany);
        }

        // Company contacts count too: an address belongs to one contact.
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM crm_contacts
                WHERE team_id = $1 AND LOWER(email) = $2
            ) AS "exists!"
            "#,
            team_id,
            normalized_email,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CrmError::StorageLayerError(e.into()))?;

        if exists {
            return Err(CrmError::ContactAlreadyExistsForTeam);
        }

        let row = sqlx::query!(
            r#"
            INSERT INTO crm_contacts
                (team_id, company_id, email, name, first_interaction, last_interaction, manually_created)
            VALUES ($1, NULL, $2, $3, $4, $4, TRUE)
            RETURNING id, company_id, email, name, hidden,
                      first_interaction, last_interaction, created_at, updated_at
            "#,
            team_id,
            normalized_email,
            name,
            now,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| CrmError::StorageLayerError(e.into()))?;

        tx.commit()
            .await
            .map_err(|e| CrmError::StorageLayerError(e.into()))?;

        Ok(CrmContact {
            id: row.id,
            company_id: row.company_id,
            email: row.email,
            name: row.name,
            hidden: row.hidden,
            first_interaction: row.first_interaction,
            last_interaction: row.last_interaction,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
