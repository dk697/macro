import { mkdirSync, writeFileSync } from 'node:fs';
import { chromium } from '@playwright/test';
import { bootstrapResponses } from '../native/fixtures/bootstrap';
import { USER_ID } from '../native/fixtures/mail';

// Full production components and routing, with API transport intercepted.
// Screenshots the person contacts flows against synthetic CRM data.
const origin = process.env.PERSON_CONTACTS_URL ?? 'http://127.0.0.1:3038';
const out = process.env.PERSON_CONTACTS_OUT ?? '/tmp/person-contacts-shots';
mkdirSync(out, { recursive: true });

const TEAM = '00000000-0000-0000-0000-000000001111';
const T0 = '2026-09-01T00:00:00Z';
const T1 = '2026-10-20T00:00:00Z';
const ACME = '00000000-0000-0000-0000-00000000c001';
const SAM = '00000000-0000-0000-0000-00000000a001';
const PRIYA = '00000000-0000-0000-0000-00000000a002';
const JANE = '00000000-0000-0000-0000-00000000a003';
const PIPELINE = '00000000-0000-0000-0000-00000000b001';
const DB = '00000000-0000-0000-0000-00000000b002';
const TABLE = '00000000-0000-0000-0000-00000000b003';
const VIEW = '00000000-0000-0000-0000-00000000b004';
const COL_CONTACT = '00000000-0000-0000-0000-00000000c101';
const COL_STAGE = '00000000-0000-0000-0000-00000000c102';
const DEF_CONTACT = '00000000-0000-0000-0000-00000000d101';
const DEF_STAGE = '00000000-0000-0000-0000-00000000d102';
const OPT_LEAD = '00000000-0000-0000-0000-00000000f101';
const OPT_ONBOARDING = '00000000-0000-0000-0000-00000000f102';
const OPT_SEARCHING = '00000000-0000-0000-0000-00000000f103';
const OPT_CONTRACT = '00000000-0000-0000-0000-00000000f104';
const OPT_SETTLED = '00000000-0000-0000-0000-00000000f105';

type Contact = {
  id: string;
  companyId: string | null;
  email: string;
  name: string;
  stage: string;
};
const contacts: Contact[] = [
  { id: SAM, companyId: null, email: 'sam.lee@gmail.com', name: 'Sam & Alex Lee', stage: OPT_SEARCHING },
  { id: PRIYA, companyId: null, email: 'priya.nair@outlook.com', name: 'Priya Nair', stage: OPT_ONBOARDING },
  { id: JANE, companyId: ACME, email: 'jane@acmeconvey.com.au', name: 'Jane Doe', stage: OPT_LEAD },
];
const company = {
  id: ACME,
  teamId: TEAM,
  name: 'Acme Conveyancing',
  description: null,
  domains: [{ id: 'd1', companyId: ACME, domain: 'acmeconvey.com.au', createdAt: T0 }],
  emailSync: true,
  hidden: false,
  createdAt: T0,
  updatedAt: T1,
  viewedAt: null,
  properties: [],
};
const contactResponse = (c: Contact) => ({
  id: c.id,
  companyId: c.companyId,
  email: c.email,
  name: c.name,
  hidden: false,
  firstInteraction: T0,
  lastInteraction: T1,
  createdAt: T0,
  updatedAt: T1,
});
const contactSoupItem = (c: Contact) => ({
  tag: 'crmContact',
  frecency_score: 0,
  is_favorited: false,
  data: {
    ...contactResponse(c),
    teamId: TEAM,
    companyName: c.companyId ? company.name : null,
    viewedAt: null,
    properties: [],
  },
});
const companySoupItem = { tag: 'crmCompany', frecency_score: 0, is_favorited: false, data: company };
const contactGraphql = (c: Contact) => ({
  __typename: 'GraphqlSoupCrmContact',
  id: c.id,
  contactTeamId: TEAM,
  companyId: c.companyId ?? '',
  companyName: c.companyId ? company.name : '',
  parentCompanyId: c.companyId,
  parentCompanyName: c.companyId ? company.name : null,
  email: c.email,
  crmContactName: c.name,
  hidden: false,
  firstInteraction: T0,
  lastInteraction: T1,
  createdAt: T0,
  updatedAt: T1,
  viewedAt: null,
});
const option = (id: string, label: string, order: number) => ({
  id,
  property_definition_id: DEF_STAGE,
  display_order: order,
  color: null,
  value: { type: 'string', value: label },
  created_at: T0,
  updated_at: T0,
});
const definition = (id: string, name: string, dataType: string, entity: string | null) => ({
  id,
  display_name: name,
  data_type: dataType,
  specific_entity_type: entity,
  is_multi_select: false,
  is_system: false,
  is_metadata: false,
  owner: { scope: 'database', database_id: DB },
  created_at: T0,
  updated_at: T0,
});
const table = {
  sql_name: 'Buyers',
  table: { id: TABLE, database_id: DB, name: 'Buyers', position: 'a0', version: 1 },
  columns: [
    {
      sql_name: 'Contact',
      writable: true,
      shared_outside_database: false,
      column: { id: COL_CONTACT, table_id: TABLE, property_definition_id: DEF_CONTACT, position: 'a0', config: null, display_name: null, infer_type: false, nullable: false, protections: ['delete', 'change_type'] },
      definition: { definition: definition(DEF_CONTACT, 'Contact', 'ENTITY', 'CONTACT'), property_options: [] },
    },
    {
      sql_name: 'Stage',
      writable: true,
      shared_outside_database: false,
      column: { id: COL_STAGE, table_id: TABLE, property_definition_id: DEF_STAGE, position: 'a1', config: null, display_name: null, infer_type: false },
      definition: {
        definition: definition(DEF_STAGE, 'Stage', 'SELECT_STRING', null),
        property_options: [
          option(OPT_LEAD, 'Lead', 0),
          option(OPT_ONBOARDING, 'Onboarding', 1),
          option(OPT_SEARCHING, 'Searching', 2),
          option(OPT_CONTRACT, 'Under contract', 3),
          option(OPT_SETTLED, 'Settled', 4),
        ],
      },
    },
  ],
  views: [
    {
      id: VIEW,
      tableId: TABLE,
      databaseId: DB,
      name: 'Board',
      position: 'a0',
      createdAt: T0,
      updatedAt: T0,
      query: { filter: null, sort: [] },
      layout: { kind: 'board', groupBy: COL_STAGE, title: COL_CONTACT, cardFields: [], lanes: [], hideEmptyLanes: false },
    },
  ],
};
const pipeline = {
  id: PIPELINE,
  teamId: TEAM,
  name: 'Buyers',
  userId: USER_ID,
  recordType: 'contact',
  databaseId: DB,
  tableId: TABLE,
  primaryColumnId: COL_CONTACT,
  sharing: 'team',
  grant: 'owner',
  createdAt: T0,
  trashedAt: null,
};
const rows = () => ({
  rows: contacts.map((c) => ({
    rowId: c.id.replace('0000a', '0000e'),
    cells: {
      [COL_CONTACT]: { type: 'EntityReference', value: [{ entity_id: c.id, entity_type: 'CONTACT', specific_message_id: null }] },
      [COL_STAGE]: { type: 'SelectOption', value: [c.stage] },
    },
  })),
  next: null,
  version: 1,
});
const emptyPage = { items: [], next_cursor: null };

const browser = await chromium.launch({
  executablePath: process.env.CHROMIUM_PATH,
  headless: true,
  args: ['--no-sandbox'],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  timezoneId: 'UTC',
  serviceWorkers: 'block',
});
const page = await context.newPage();
page.setDefaultTimeout(20000);
const errors: string[] = [];
const unknown: string[] = [];
const steps: Record<string, string> = {};
const created: unknown[] = [];
page.on('pageerror', (error) => errors.push(error.message));
page.on('console', (message) => {
  if (message.type() === 'error' || message.type() === 'warning')
    errors.push(`${message.type()}: ${message.text().slice(0, 400)}`);
});
await context.addCookies([{ name: 'login', value: 'true', url: origin }]);
await page.addInitScript(
  ([key, view]) => localStorage.setItem(key, JSON.stringify(view)),
  [`macro:pref:crm:pipeline-view:${PIPELINE}`, VIEW]
);
await page.routeWebSocket('**/*', (socket) => socket.close());

await page.route('**/*', async (route) => {
  const request = route.request();
  const url = new URL(request.url());
  const path = url.pathname;
  const method = request.method();
  const respond = (body: unknown, status = 200) =>
    route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) });
  if (url.origin !== origin) return route.abort();
  if (path.startsWith('/app') || path === '/') return route.continue();
  if (path === '/auth/jwt/refresh') return respond({});
  if (path === '/auth/team')
    return respond({
      team: { id: TEAM, name: 'Compass Buyers Agency', owner_id: USER_ID, slug: 'compass', enterprise: false, crm_enabled: true, allow_non_admin_invites: true },
      members: [{ user_id: USER_ID, team_id: TEAM, role: 'owner', plan: 'free' }],
    });
  if (method === 'GET' && path === '/dss/crm/settings')
    return respond({ edit_stages_role: 'admin', move_closed_deals_role: 'admin', delete_records_role: 'admin', legacy_stage_ids: {}, team_views: [], closed_stage_ids: null, default_team_view_id: null });
  if (method === 'GET' && path === '/dss/saved_views') return respond({ views: [], excludedDefaultViews: [] });
  if (method === 'GET' && path === '/dss/crm/pipelines') return respond([pipeline]);
  if (method === 'GET' && path === `/dss/crm/pipelines/${PIPELINE}`) return respond(pipeline);
  if (method === 'GET' && path === `/dss/crm/pipelines/${PIPELINE}/table`) return respond(table);
  if (method === 'POST' && path === `/dss/crm/pipelines/${PIPELINE}/rows`) return respond(rows());
  if (method === 'POST' && path === `/dss/crm/pipelines/${PIPELINE}/ops`) return respond({ results: [], changes: [] });
  const contactMatch = path.match(/^\/dss\/crm\/contacts\/([^/]+)$/);
  if (method === 'GET' && contactMatch) {
    const c = contacts.find((x) => x.id === contactMatch[1]);
    return c ? respond(contactResponse(c)) : respond({ message: 'not found' }, 404);
  }
  if (method === 'GET' && path === `/dss/crm/companies/${ACME}`)
    return respond({ ...company, contacts: contacts.filter((c) => c.companyId === ACME).map(contactResponse) });
  if (method === 'POST' && path === '/dss/crm/contacts') {
    const body = request.postDataJSON() as { name: string; email: string };
    created.push(body);
    if (contacts.some((c) => c.email === body.email.toLowerCase()))
      return respond({ message: 'a crm contact with this email already exists for the team' }, 409);
    const c: Contact = { id: '00000000-0000-0000-0000-00000000a0ff', companyId: null, email: body.email.toLowerCase(), name: body.name, stage: OPT_LEAD };
    contacts.push(c);
    return respond(contactResponse(c));
  }
  if (method === 'POST' && path === '/dss/items/soup/ast') {
    const body = (request.postDataJSON() ?? {}) as Record<string, unknown>;
    if (body.crmf && !body.crma) return respond({ items: contacts.map(contactSoupItem), next_cursor: null });
    if (body.crma || body.eca || body.ecd || body.ef) return respond(emptyPage);
    if (JSON.stringify(body.ccf ?? '').includes('"hidden":false')) return respond({ items: [companySoupItem], next_cursor: null });
    return respond(emptyPage);
  }
  if (method === 'POST' && path === '/dss/items/soup') {
    const body = (request.postDataJSON() ?? {}) as Record<string, unknown>;
    if (body.sort_method === 'viewed_updated' && !('crm_company_filters' in body))
      return respond({ items: [companySoupItem], next_cursor: null });
    return respond(emptyPage);
  }
  if (method === 'POST' && path === '/dss/items/soup/graphql') {
    const body = request.postDataJSON() as { operationName?: string };
    const items = body.operationName === 'CrmContacts' ? contacts.map(contactGraphql) : [];
    return respond({ data: { user: { __typename: 'GraphqlUser', id: USER_ID, soup: { __typename: 'SoupPage', items, nextCursor: null } } } });
  }
  if (method === 'GET' && /^\/dss\/messages\/crm_(contact|company)\/[^/]+\/timeline$/.test(path))
    return respond({ entries: [], next_cursor: null, previous_cursor: null });
  if (method === 'GET' && /^\/dss\/entities\/[^/]+\/[^/]+\/access-requests$/.test(path))
    return respond({ canReview: false, pendingCount: 0, requests: [], supportedLevels: [] });
  const fixture = bootstrapResponses.get(`${method} ${path}`);
  if (fixture) return respond(fixture.body, fixture.status ?? 200);
  unknown.push(`${method} ${path}`);
  return respond({}, 404);
});

async function step(name: string, run: () => Promise<void>) {
  try {
    await run();
    steps[name] = 'ok';
  } catch (error) {
    steps[name] = `failed: ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`;
    await page.screenshot({ path: `${out}/${name}-failed.png`, fullPage: true }).catch(() => {});
  }
}
const shot = (name: string) => page.screenshot({ path: `${out}/${name}.png`, fullPage: true });
const settle = () => page.waitForTimeout(1500);

await step('01-people', async () => {
  await page.goto(`${origin}/app/companies?s0.crm.tab=people`, { waitUntil: 'domcontentloaded' });
  await page.getByText('sam.lee@gmail.com').first().waitFor({ timeout: 60000 });
  await settle();
  await shot('01-people');
});

await step('02-new-contact-no-company', async () => {
  await page.getByRole('button', { name: 'New contact' }).first().click();
  const dialog = page.getByRole('form', { name: 'New contact' });
  await dialog.getByLabel('Contact name').fill('Jordan & Kim Walsh');
  await dialog.getByLabel('Email').fill('jordan.walsh@gmail.com');
  await settle();
  await shot('02-new-contact-no-company');
});

await step('03-created-person-contact', async () => {
  await page
    .getByRole('form', { name: 'New contact' })
    .getByRole('button', { name: /Create Contact/i })
    .click();
  await page.getByText('jordan.walsh@gmail.com').first().waitFor();
  await settle();
  await shot('03-created-person-contact');
});

await step('04-duplicate-refused', async () => {
  await page.goto(`${origin}/app/companies?s0.crm.tab=people`, { waitUntil: 'domcontentloaded' });
  await page.getByText('sam.lee@gmail.com').first().waitFor();
  await page.getByRole('button', { name: 'New contact' }).first().click();
  const dialog = page.getByRole('form', { name: 'New contact' });
  await dialog.getByLabel('Contact name').fill('Sam Lee');
  await dialog.getByLabel('Email').fill('sam.lee@gmail.com');
  await dialog.getByRole('button', { name: /Create Contact/i }).click();
  await dialog.getByRole('alert').waitFor();
  await settle();
  await shot('04-duplicate-refused');
  await page.keyboard.press('Escape');
});

await step('05-person-contact-page', async () => {
  await page.goto(`${origin}/app/companies/contact/${SAM}?s0.crm.tab=people`, { waitUntil: 'domcontentloaded' });
  await page.getByText('sam.lee@gmail.com').first().waitFor();
  await settle();
  await shot('05-person-contact-page');
});

await step('06-company-contact-page', async () => {
  await page.goto(`${origin}/app/companies/contact/${JANE}?s0.crm.tab=people`, { waitUntil: 'domcontentloaded' });
  await page.getByText('jane@acmeconvey.com.au').first().waitFor();
  await settle();
  await shot('06-company-contact-page');
});

await step('07-buyers-pipeline', async () => {
  await page.goto(`${origin}/app/companies/pipeline/${PIPELINE}`, { waitUntil: 'domcontentloaded' });
  await page.getByText('Sam & Alex Lee').first().waitFor();
  await page.waitForTimeout(3000);
  await shot('07-buyers-pipeline');
});

writeFileSync(
  `${out}/report.json`,
  JSON.stringify({ steps, errors, unknown: [...new Set(unknown)], created }, null, 2)
);
await browser.close();
console.log(JSON.stringify(steps, null, 2));
