# Menu Smoke Checklist

Run after `docker compose up --build -d postgres api` and `docker compose run --rm test cargo run --bin seed-demo -- --reset`.

Demo login: `demo / admin / ChangeMe123!`.

v1.2 status: the active menu follows the design document's 99-leaf completion gate. Fine-grained v1.1 duplicates are folded into their parent functional screens, and `tests/menu_full_tree.rs` asserts exact 99-key coverage.

## v1.2 Full Tree Checks

| Area | Leaf count | Smoke check |
|---|---:|---|
| Dashboard | 5 | Each `#/dashboard/*` URL opens the dashboard delegate and shows the leaf watermark. |
| Workspace | 43 | Each `#/workspace/ws/*/*` URL reloads, keeps stepper/LawBanner layout, and shows the one-click work-context guide when no year is selected. |
| Post filing | 6 | History leaves render without context; amendment/correction leaves show the work-context guide when no year is selected. |
| Reports | 6 | Each `#/report/*` URL uses the plain layout and delegates to the current report surfaces. |
| Admin | 39 | Each `#/admin/*/*` URL uses the admin layout and has one menu-function permission mapping. |

## Leaf Key Checklist

| Group | Keys |
|---|---|
| dashboard | `dashboard:overview`, `dashboard:duesoon`, `dashboard:inbox`, `dashboard:recent`, `dashboard:kpi-tax` |
| ws/start | `ws/start:customer-pick`, `ws/start:by-pick`, `ws/start:snapshot` |
| ws/info | `ws/info:fs`, `ws/info:mapping`, `ws/info:assets`, `ws/info:transactions`, `ws/info:vehicle`, `ws/info:consistency` |
| ws/adj | `ws/adj:B1`, `ws/adj:B2`, `ws/adj:B3`, `ws/adj:B4`, `ws/adj:B5`, `ws/adj:B6`, `ws/adj:B7`, `ws/adj:B8`, `ws/adj:B9`, `ws/adj:B10`, `ws/adj:B11`, `ws/adj:B12`, `ws/adj:B13`, `ws/adj:B14`, `ws/adj:B15`, `ws/adj:B16`, `ws/adj:B17` |
| ws/form | `ws/form:form3`, `ws/form:attachments`, `ws/form:preview`, `ws/form:linkage` |
| ws/val | `ws/val:run`, `ws/val:issues`, `ws/val:rules` |
| ws/appr | `ws/appr:request`, `ws/appr:inbox`, `ws/appr:rejected` |
| ws/print | `ws/print:preview`, `ws/print:bulk`, `ws/print:history` |
| ws/file | `ws/file:precheck`, `ws/file:generate`, `ws/file:submit`, `ws/file:done` |
| post | `post/hist:list`, `post/amend:unlock`, `post/amend:version`, `post/amend:diff`, `post/amend:resubmit`, `post/correction` |
| report | `report:year-compare`, `report:tax-burden`, `report:reserve-trend`, `report:loss-expiry`, `report:industry-stats`, `report:custom` |
| admin/cust | `admin/cust:list`, `admin/cust:by-master`, `admin/cust:agent` |
| admin/sec | `admin/sec:users`, `admin/sec:roles`, `admin/sec:matrix`, `admin/sec:menus`, `admin/sec:functions`, `admin/sec:mask`, `admin/sec:scope` |
| admin/cacc | `admin/cacc:assign`, `admin/cacc:groups`, `admin/cacc:rules`, `admin/cacc:delegate`, `admin/cacc:override` |
| admin/law | `admin/law:master`, `admin/law:rates`, `admin/law:limits`, `admin/law:credits`, `admin/law:depr-lives`, `admin/law:sme`, `admin/law:loss-rule`, `admin/law:snapshots`, `admin/law:impact`, `admin/law:history` |
| admin/form | `admin/form:master`, `admin/form:versions`, `admin/form:fields`, `admin/form:validations`, `admin/form:linkage-rule`, `admin/form:migration`, `admin/form:efile-map`, `admin/form:by-set`, `admin/form:impact` |
| admin/code | `admin/code:manage` |
| admin/audit | `admin/audit:events`, `admin/audit:login`, `admin/audit:perm`, `admin/audit:settings` |

## Data Smoke Checks

| Legacy delegate | Data check | Interaction check |
|---|---|---|
| dashboard | Customer, business-year, filed, review, due-soon, notification, and audit KPIs are non-zero. | Dashboard leaf links route through `#/dashboard/*`. |
| ws-start | At least 3 customers and 5 business years are listed. | Selecting a business year keeps customer/year context. |
| ws-info | Financial statements >=30, assets >=8, transactions >=10, vehicle logs >=3, import batches >=3. | Tax-data validation returns balanced data. |
| ws-adj | B1-B17 module items, reserves, B1/B4/B15 item grids, history, and evidence attachment rows are present. | Representative adjustment deep links such as `#/workspace/ws/adj/B12` reload. |
| ws-form | FORM3, FORM15, FORM22, FORM32, FORM50, ATT01-ATT10 are generated. | FORM3 preview and individual generation return 200. |
| ws-val | Validation rules >=50 and validation run returns issues with `error_count=0`. | Dismiss action can mark an issue as dismissed. |
| ws-appr | Review queue and workflow events are populated. | Approve changes active context year to APPROVED. |
| ws-print | Attachment list and print-history rows are populated after PDF generation. | PDF bundle download returns non-empty bytes and records print history. |
| ws-file | Format spec and precheck are populated, and precheck is valid. | E-filing create queues a job; 2FA users must provide OTP for step-up submission. |
| post-hist/post-amend | Business years, e-filing history, and amendment preview differences are populated. | Unlock can move a filed year into amendment mode. |
| reports | Year comparison, tax burden, reserve trend, loss expiry, industry stats, and custom report data are populated. | User-defined loss-expiry report create returns 201. |
| admin | Tenants, customers, users, roles, menus, menu-functions, access delegation, law/form metadata, and audit logs are populated. | Menu feature flag update returns 200 and menu-functions remain populated for all leaves. |

External ERP/e-Tax direct submission and Redis cache invalidation are outside this smoke checklist and remain P3 roadmap items.
