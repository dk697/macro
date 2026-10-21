use super::helpers::*;
use crate::domain::companies_repo::*;
use crate::domain::contact_listing::{CrmContactListQuery, CrmContactListScope};
use crate::domain::model::CrmError;
use crate::domain::service::{CrmService, CrmServiceImpl};
use crate::outbound::companies_repo::*;
use crate::outbound::no_op_resolver::NoOpCompanyMetadataResolver;
use chrono::{TimeZone, Utc};
use filter_ast::Expr;
use item_filters::ast::crm_contact::CrmContactLiteral;
use macro_db_migrator::MACRO_DB_MIGRATIONS;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn create_person_contact_stores_a_contact_without_a_company(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;

    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "Sam.Lee@Gmail.com", "Sam Lee", Utc::now())
        .await?;

    assert_eq!(contact.company_id, None);
    assert_eq!(contact.email, "sam.lee@gmail.com");
    assert_eq!(contact.name.as_deref(), Some("Sam Lee"));
    let (stored_team, manually_created): (Uuid, bool) =
        sqlx::query_as("SELECT team_id, manually_created FROM crm_contacts WHERE id = $1")
            .bind(contact.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored_team, team_id);
    assert!(manually_created);
    assert_eq!(
        count_companies_for_domain(&pool, team_id, "gmail.com").await?,
        0
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn create_person_contact_refuses_an_address_the_team_already_has(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;

    let repo = CompaniesRepositoryImpl::new(pool.clone());
    repo.create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;
    let err = repo
        .create_person_contact(&team_id, "SAM.LEE@gmail.com", "Sam", Utc::now())
        .await
        .unwrap_err();

    assert!(
        matches!(err, CrmError::ContactAlreadyExistsForTeam),
        "{err:?}"
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn create_person_contact_refuses_a_domain_tracked_as_a_company(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;
    insert_company(&pool, team_id, true, &["acme.com"]).await?;

    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let err = repo
        .create_person_contact(&team_id, "jane@acme.com", "Jane", Utc::now())
        .await
        .unwrap_err();

    assert!(
        matches!(err, CrmError::ContactDomainTrackedByCompany),
        "{err:?}"
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn create_person_contact_refuses_when_crm_is_disabled(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;

    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let err = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await
        .unwrap_err();

    assert!(matches!(err, CrmError::CrmDisabledForTeam), "{err:?}");
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn company_contact_refuses_an_address_a_person_contact_owns(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    repo.create_person_contact(&team_id, "jo@parkfamily.net", "Jo Park", Utc::now())
        .await?;
    let company_id = insert_company(&pool, team_id, true, &["parkfamily.net"]).await?;

    let err = repo
        .create_contact_for_company(
            &team_id,
            &company_id,
            "jo@parkfamily.net",
            "Jo",
            Utc::now(),
            false,
        )
        .await
        .unwrap_err();

    assert!(
        matches!(err, CrmError::ContactAlreadyExistsForTeam),
        "{err:?}"
    );
    assert_eq!(count_contacts(&pool, company_id).await?, 0);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn populate_on_a_generic_domain_without_a_person_contact_writes_nothing(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@test.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let link_id = insert_email_link(&pool, owner_id, "owner@compass.test").await?;
    let service = CrmServiceImpl::new(
        CompaniesRepositoryImpl::new(pool.clone()),
        NoOpCompanyMetadataResolver,
    );

    service
        .populate_contact(
            &team_id,
            &link_id,
            "owner@compass.test",
            "stranger@gmail.com",
            Some("Stranger"),
            Utc::now(),
            Utc::now(),
            true,
        )
        .await?;

    let (contacts,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM crm_contacts WHERE team_id = $1")
            .bind(team_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(contacts, 0);
    assert_eq!(
        count_companies_for_domain(&pool, team_id, "gmail.com").await?,
        0
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn populate_on_a_generic_domain_attaches_email_to_the_person_contact(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@test.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let link_id = insert_email_link(&pool, owner_id, "owner@compass.test").await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let created_at = Utc.with_ymd_and_hms(2026, 10, 1, 9, 0, 0).unwrap();
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", created_at)
        .await?;
    let service = CrmServiceImpl::new(repo, NoOpCompanyMetadataResolver);

    let earlier = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
    let later = Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap();
    service
        .populate_contact(
            &team_id,
            &link_id,
            "owner@compass.test",
            "Sam.Lee@gmail.com",
            Some("Samuel"),
            earlier,
            later,
            true,
        )
        .await?;

    let (sources,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM crm_contact_sources WHERE contact_id = $1 AND link_id = $2",
    )
    .bind(contact.id)
    .bind(link_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(sources, 1);
    assert_eq!(
        fetch_contact_interactions(&pool, contact.id).await?,
        Some((earlier, later))
    );
    assert_eq!(
        fetch_contact_name(&pool, contact.id).await?,
        Some(Some("Sam Lee".to_owned())),
        "a person contact keeps the name it was given"
    );
    assert_eq!(
        count_companies_for_domain(&pool, team_id, "gmail.com").await?,
        0
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn populate_attaches_to_a_person_contact_when_the_user_shares_its_domain(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@gmail.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let link_id = insert_email_link(&pool, owner_id, "owner@gmail.com").await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;
    let service = CrmServiceImpl::new(repo, NoOpCompanyMetadataResolver);

    service
        .populate_contact(
            &team_id,
            &link_id,
            "owner@gmail.com",
            "sam.lee@gmail.com",
            None,
            Utc::now(),
            Utc::now(),
            false,
        )
        .await?;

    let (sources,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM crm_contact_sources WHERE contact_id = $1")
            .bind(contact.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(sources, 1);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn populate_on_an_untracked_domain_attaches_to_the_person_contact(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@test.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let link_id = insert_email_link(&pool, owner_id, "owner@compass.test").await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "jo@parkfamily.net", "Jo Park", Utc::now())
        .await?;
    let service = CrmServiceImpl::new(repo, NoOpCompanyMetadataResolver);

    service
        .populate_contact(
            &team_id,
            &link_id,
            "owner@compass.test",
            "jo@parkfamily.net",
            None,
            Utc::now(),
            Utc::now(),
            true,
        )
        .await?;

    assert_eq!(
        count_companies_for_domain(&pool, team_id, "parkfamily.net").await?,
        0,
        "the person contact owns the address, so no company is created"
    );
    let (sources,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM crm_contact_sources WHERE contact_id = $1")
            .bind(contact.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(sources, 1);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn populate_on_a_generic_domain_noops_when_crm_is_disabled(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@test.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let link_id = insert_email_link(&pool, owner_id, "owner@compass.test").await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;
    sqlx::query("UPDATE team_crm_settings SET crm_enabled = FALSE WHERE team_id = $1")
        .bind(team_id)
        .execute(&pool)
        .await?;

    repo.populate_person_contact(
        &team_id,
        &link_id,
        "sam.lee@gmail.com",
        Utc::now(),
        Utc::now(),
        true,
    )
    .await?;

    let (sources,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM crm_contact_sources WHERE contact_id = $1")
            .bind(contact.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(sources, 0);
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn depopulate_drops_the_source_and_keeps_the_person_contact(
    pool: PgPool,
) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let owner_id = "macro|owner@test.com";
    seed_team(&pool, team_id, owner_id).await?;
    enable_crm_for_team(&pool, team_id).await?;
    let first_link = insert_email_link(&pool, owner_id, "owner@compass.test").await?;
    let second_link = insert_email_link(&pool, owner_id, "owner@compass.example").await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;
    for link_id in [first_link, second_link] {
        repo.populate_person_contact(
            &team_id,
            &link_id,
            "sam.lee@gmail.com",
            Utc::now(),
            Utc::now(),
            true,
        )
        .await?;
    }

    let outcome = repo
        .depopulate_contact(&team_id, &first_link, "gmail.com", "sam.lee@gmail.com")
        .await?;
    assert!(outcome.source_deleted);
    assert!(!outcome.contact_deleted);
    repo.depopulate_link_in_team(&team_id, &second_link).await?;

    let (sources,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM crm_contact_sources WHERE contact_id = $1")
            .bind(contact.id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(sources, 0);
    assert!(
        repo.get_contact_for_team(&team_id, &contact.id, false)
            .await?
            .is_some()
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn person_contacts_are_readable_within_their_team_only(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    let other_team = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    seed_team(&pool, other_team, "macro|other@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;
    enable_crm_for_team(&pool, other_team).await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;

    let by_id = repo
        .get_contact_for_team(&team_id, &contact.id, false)
        .await?
        .unwrap();
    assert_eq!(by_id.company_id, None);
    let by_email = repo
        .get_contact_by_email_for_team(&team_id, "SAM.LEE@gmail.com", false)
        .await?
        .unwrap();
    assert_eq!(by_email.id, contact.id);
    let searched = repo
        .search_contacts_for_team(&team_id, "sam", 10, false)
        .await?;
    assert_eq!(
        searched.iter().map(|c| c.id).collect::<Vec<_>>(),
        [contact.id]
    );

    assert!(
        repo.get_contact_for_team(&other_team, &contact.id, true)
            .await?
            .is_none()
    );
    assert!(
        repo.get_contact_by_email_for_team(&other_team, "sam.lee@gmail.com", true)
            .await?
            .is_none()
    );
    assert!(
        repo.search_contacts_for_team(&other_team, "sam", 10, true)
            .await?
            .is_empty()
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn person_contacts_list_without_a_company_name(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    let contact = repo
        .create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;

    let rows = repo
        .list_contacts_for_soup(
            CrmContactListScope::Team {
                team_id,
                include_hidden: false,
            },
            "macro|owner@test.com",
            &CrmContactListQuery {
                filter: Expr::Literal(CrmContactLiteral::Include),
                sort: CrmCompanyListSort::UpdatedAt,
                cursor: None,
                ascending: false,
                limit: 10,
            },
        )
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].contact.id, contact.id);
    assert_eq!(rows[0].contact.company_id, None);
    assert_eq!(rows[0].company_name, None);
    assert_eq!(rows[0].team_id, team_id);

    let association = repo
        .resolve_association(
            CrmContactListScope::Team {
                team_id,
                include_hidden: false,
            },
            crate::domain::association::CrmAssociation::Contact(contact.id),
        )
        .await?;
    assert_eq!(
        association,
        Some(crate::domain::association::CrmRecordAssociation::contact(
            contact.id
        ))
    );
    Ok(())
}

#[sqlx::test(migrator = "MACRO_DB_MIGRATIONS")]
async fn precheck_reports_a_person_contact_as_synced(pool: PgPool) -> anyhow::Result<()> {
    let team_id = Uuid::now_v7();
    seed_team(&pool, team_id, "macro|owner@test.com").await?;
    enable_crm_for_team(&pool, team_id).await?;
    let repo = CompaniesRepositoryImpl::new(pool.clone());
    repo.create_person_contact(&team_id, "sam.lee@gmail.com", "Sam Lee", Utc::now())
        .await?;

    let result = repo
        .crm_scope_precheck(&team_id, &[], &["sam.lee@gmail.com".to_string()])
        .await?;

    let sam = &result.addresses[0];
    assert!(sam.exists);
    assert!(sam.email_sync);
    assert!(!sam.company_hidden);
    assert!(!sam.contact_hidden);
    Ok(())
}
