/** CRM contact values, independent of the transport schema. */
export type CrmContact = {
  id: string;
  /** Absent for a person contact, which has no company. */
  companyId?: string | null;
  name?: string | null;
  email: string;
  hidden: boolean;
  firstInteraction: string;
  lastInteraction: string;
  createdAt: string;
  updatedAt: string;
};
