//! CreateContact tool: manually add a contact to a CRM company, or a person
//! contact with no company.

use ai_toolset::{AsyncTool, RequestContext, ServiceContext, ToolResult};
use ai_toolset::{ToolAnnotated, ToolAnnotations};
use async_trait::async_trait;
use entity_access::domain::ports::EntityAccessService;
use properties::PropertiesService;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::service::CrmService;

use super::{CrmToolContext, caller_team_receipt, company_view_receipt, crm_error};

/// The created contact.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactResponse {
    /// The new CRM contact id.
    pub contact_id: Uuid,
    /// The company the contact belongs to; absent for a person contact.
    pub company_id: Option<Uuid>,
    /// The contact's email.
    pub email: String,
    /// The contact's display name.
    pub name: Option<String>,
}

/// Manually add a contact to a CRM company, or a person contact with no
/// company.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "camelCase")]
#[schemars(
    title = "CreateContact",
    description = "Add a contact person to the user's team CRM, as New contact does. With a companyId, the contact goes under that company and the email's domain must be one of the company's domains; find the company with ListCompanies, or add it first with CreateCompany. Without one, the contact is a person contact with no company, for a client on a personal address such as gmail.com; their email then links to them automatically."
)]
pub struct CreateContact {
    /// Company to add the contact to; omit for a person contact.
    #[schemars(
        description = "The CRM company id to add the contact to. Omit it for a person contact with no company."
    )]
    pub company_id: Option<Uuid>,
    /// Display name.
    #[schemars(description = "The contact's name.")]
    pub name: String,
    /// Email address.
    #[schemars(description = "The contact's email address, e.g. \"jane@acme.com\".")]
    pub email: String,
}

impl ToolAnnotated for CreateContact {
    const ANNOTATIONS: ToolAnnotations = ToolAnnotations::additive("Create CRM contact");
}

#[async_trait]
impl<CSvc, ESvc, PSvc> AsyncTool<CrmToolContext<CSvc, ESvc, PSvc>> for CreateContact
where
    CSvc: CrmService,
    ESvc: EntityAccessService,
    PSvc: PropertiesService,
{
    type Output = CreateContactResponse;

    #[tracing::instrument(skip_all, fields(user_id=?request_context.user_id, company_id=?self.company_id), err)]
    async fn call(
        &self,
        service_context: ServiceContext<CrmToolContext<CSvc, ESvc, PSvc>>,
        request_context: RequestContext,
    ) -> ToolResult<Self::Output> {
        let contact = match self.company_id {
            Some(company_id) => {
                let (company, _) = company_view_receipt(
                    &*service_context.entity_access_service,
                    &request_context,
                    company_id,
                )
                .await?;
                service_context
                    .service
                    .create_contact(&company, self.name.trim(), self.email.trim())
                    .await
            }
            None => {
                let (team, _) =
                    caller_team_receipt(&*service_context.entity_access_service, &request_context)
                        .await?;
                service_context
                    .service
                    .create_person_contact(&team, self.name.trim(), self.email.trim())
                    .await
            }
        }
        .map_err(crm_error)?;

        Ok(CreateContactResponse {
            contact_id: contact.id,
            company_id: contact.company_id,
            email: contact.email,
            name: contact.name,
        })
    }
}
