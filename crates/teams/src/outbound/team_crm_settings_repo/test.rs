use macro_db_migrator::MACRO_DB_MIGRATIONS;
use uuid::Uuid;

use super::*;

const TEAM1: Uuid = Uuid::from_u128(0x1111_1111_1111_1111_1111_1111_1111_1111);
const TEAM2: Uuid = Uuid::from_u128(0x2222_2222_2222_2222_2222_2222_2222_2222);

#[sqlx::test(
    migrator = "MACRO_DB_MIGRATIONS",
    fixtures(path = "../../../fixtures", scripts("teams"))
)]
async fn disabling_crm_purges_company_and_person_contacts(pool: PgPool) {
    let company = Uuid::now_v7();
    sqlx::query!(
        r#"
        INSERT INTO crm_companies (id, team_id, first_interaction, last_interaction)
        VALUES ($1, $2, now(), now())
        "#,
        company,
        TEAM1,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"
        INSERT INTO crm_contacts (team_id, company_id, email, first_interaction, last_interaction)
        VALUES ($1, $2, 'jane@acme.com', now(), now()),
               ($1, NULL, 'sam.lee@gmail.com', now(), now()),
               ($3, NULL, 'jo.park@gmail.com', now(), now())
        "#,
        TEAM1,
        company,
        TEAM2,
    )
    .execute(&pool)
    .await
    .unwrap();
    let repo = TeamCrmSettingsRepositoryImpl::new(pool.clone());

    repo.disable_crm_and_purge_data(&TEAM1).await.unwrap();

    let remaining = sqlx::query_scalar!("SELECT email FROM crm_contacts ORDER BY email")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, ["jo.park@gmail.com"]);
}
