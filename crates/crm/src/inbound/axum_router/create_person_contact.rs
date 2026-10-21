use axum::{Json, extract::State};
use entity_access::{
    domain::{models::MemberTeamRole, ports::EntityAccessService},
    inbound::axum_extractors::MacroUserTeamExtractorV2,
};
use macro_authorization::MacroAuthorizationService;
use model_error_response::ErrorResponse;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::domain::{auth::CrmTeamReceipt, model::CrmError, service::CrmService};

use super::{CrmRouterState, list_company_contacts::CrmContactResponse};

/// Request body for `POST /crm/contacts`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCrmPersonContactRequest {
    /// Display name for the contact.
    pub name: String,
    /// The contact's email address, e.g. "jane.doe@gmail.com". Personal
    /// provider domains are allowed; a domain the team tracks as a company
    /// is not (409): add the contact under that company instead.
    pub email: String,
}

/// Manually create a person contact: a contact with no company, for a
/// client on a personal address. Any team member may add one. Email to or
/// from the address attaches to the contact, including on generic provider
/// domains. The team must not already have a contact with the email or
/// track its domain as a company (409), and the team's CRM killswitch must
/// be on (403).
#[utoipa::path(
    post,
    path = "/crm/contacts",
    operation_id = "create_crm_person_contact",
    request_body = CreateCrmPersonContactRequest,
    responses(
        (status = 200, body = CrmContactResponse),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 500, body = ErrorResponse),
    ),
)]
#[tracing::instrument(skip_all, err)]
pub async fn handler<
    C: CrmService,
    St,
    Eas: EntityAccessService,
    Auth: MacroAuthorizationService,
>(
    access: MacroUserTeamExtractorV2<MemberTeamRole, Eas, Auth>,
    State(state): State<CrmRouterState<C, St, Eas, Auth>>,
    Json(req): Json<CreateCrmPersonContactRequest>,
) -> Result<Json<CrmContactResponse>, CrmError> {
    let receipt = CrmTeamReceipt::from_team_receipt(access.entity_access_receipt)?;
    let contact = state
        .service
        .create_person_contact(&receipt, &req.name, &req.email)
        .await?;
    Ok(Json(contact.into()))
}
