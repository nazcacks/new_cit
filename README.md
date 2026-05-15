# CIT Corporate Income Tax Adjustment System

새로운 법인세개발

Rust implementation of the 법인세 세무조정계산서 system described in `법인세_세무조정계산서_시스템_설계서.md`.

## What Is Implemented

- PostgreSQL-backed multi-tenant API with one schema per tenant.
- DB-backed user, role, permission, customer-access, and customer-work-scope administration.
- Tax-law/version snapshots for each business year.
- Corporate income tax adjustment calculation with persisted adjustment rows.
- Form generation for core forms (`FORM3`, `FORM15`, `FORM22`) using versioned form metadata.
- Windows-949 fixed-width e-filing file generation.
- Durable job queue with retry, exponential backoff, and dead-letter status.
- Docker Compose for PostgreSQL, API, test, and clippy services.
- Integration test covering tenant/customer/business-year setup, calculation, form generation, e-filing generation, and DLQ retry.

## Quick Start

```powershell
Copy-Item .env.example .env
docker compose up --build -d postgres api
```

The API listens on `http://localhost:8080`.

```powershell
Invoke-RestMethod http://localhost:8080/health
```

The web UI is available in a browser at:

```text
http://localhost:8080/
```

The UI calls the live API endpoints directly and shows `/health` status, tenant count, job queue status, and an end-to-end demo tax adjustment flow.

Development login:

```text
Tenant: demo
Login ID: admin
Password: admin123!
```

## Run Tests And Lints

Inside Docker:

```powershell
docker compose run --rm test
docker compose run --rm clippy
```

With a local Rust toolchain:

```powershell
$env:DATABASE_URL = "postgres://cit:cit@localhost:5432/cit"
cargo test --all --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

## Core API Flow

## Admin User / Access API

Create a tenant-scoped user with customer and work-scope access:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/admin/tenants/demo/users `
  -ContentType "application/json" `
  -Body '{"login_id":"tax01","password":"ChangeMe123!","user_name":"Tax User","roles":["TAX_EXPERT"],"customer_access":[{"customer_id":1,"access_level":"OWNER","work_scopes":["INFO","ADJUST","FORM"]}]}'
```

The embedded Admin user screen now manages multiple customer access rows per user. Each row stores an access level plus `work_scopes`, and the selectable scopes are constrained by the target customer's own `work_scopes`.

Update role permissions:

```powershell
Invoke-RestMethod -Method Put http://localhost:8080/api/admin/roles/TAX_EXPERT/permissions `
  -ContentType "application/json" `
  -Body '{"permissions":[{"module_code":"adjustment","function_code":"READ","effect":"ALLOW"},{"module_code":"efiling","function_code":"EFILE","effect":"ALLOW"}]}'
```

Create a tenant:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants `
  -ContentType "application/json" `
  -Body '{"tenant_code":"demo","tenant_name":"Demo Tax Firm","biz_reg_no":"1234567890","contract_start":"2026-01-01","max_users":20}'
```

Create a customer:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/customers `
  -ContentType "application/json" `
  -Body '{"customer_code":"CUST001","customer_name":"서울테크 주식회사","biz_reg_no":"2208112345","corp_reg_no":"1101111234567","industry_code":"62010","is_sme":true,"work_scopes":["INFO","ADJUST","FORM","VALIDATE","PRINT"]}'
```

Create a business year:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years `
  -ContentType "application/json" `
  -Body '{"customer_id":1,"year_label":2026,"start_date":"2026-01-01","end_date":"2026-12-31"}'
```

Calculate adjustments:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments `
  -ContentType "application/json" `
  -Body '{"accounting_income":500000000,"gross_revenue":3000000000,"donations":70000000,"entertainment_expense":30000000,"depreciation_book":90000000,"depreciation_tax_limit":65000000,"carryforward_loss":50000000,"tax_credits":3000000}'
```

Generate the primary tax form:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM3
```

Enqueue e-filing generation:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/efilings `
  -ContentType "application/json" `
  -Body '{"max_attempts":3}'
```

Download the generated file after the job succeeds:

```powershell
Invoke-WebRequest http://localhost:8080/api/tenants/demo/efilings/1/file -OutFile efile.txt
```

## Retry And Dead-Letter Queue

Jobs live in the `jobs` table. Failed jobs are retried with exponential backoff until `max_attempts`; then they move to `dead_letter`.

```powershell
Invoke-RestMethod "http://localhost:8080/api/jobs?status=dead_letter"
Invoke-RestMethod -Method Post http://localhost:8080/api/jobs/{job_id}/retry
```

The integration test also enqueues an unsupported job type to verify dead-letter behavior and retry requeueing.

## Repository Layout

- `src/api.rs`: Axum routes and HTTP handlers.
- `src/web.rs` and `frontend/`: built-in web UI served by the API process.
- `src/tenant.rs`: tenant provisioning and tenant-scoped persistence.
- `src/tax_data.rs`: financial statement, account mapping, asset, and transaction import.
- `src/tax.rs`: law snapshots, tax calculation, adjustment persistence, form generation.
- `src/efiling.rs`: fixed-width Windows-949 e-filing generation.
- `src/queue.rs`: durable retry/DLQ worker.
- `migrations/`: PostgreSQL schema and seed tax/form/e-file metadata.
- `tests/integration_flow.rs`: full PostgreSQL integration test.
- `examples/`: sample API payloads and PowerShell flow.

## Multi-Tenant 테넌트-고객사 관계 보완 (2026-05-15)

- 대상 사용자 조직은 `회계법인`, `세무법인`, `세무사사무소`이며, 시스템에서는 각각 독립 테넌트로 관리한다.
- 관계 모델은 `테넌트 1 : N 고객사`이다. 하나의 테넌트는 여러 고객사를 관리하고, 각 고객사는 반드시 하나의 테넌트에만 소속된다.
- UI 기준으로 `prototype/index.html`의 `5-0 테넌트 관리` 화면에서 테넌트 유형, 계약, 상태, 사용자 수, 고객사 수, 소속 고객사 샘플을 확인한다.
- `5-A 고객사 관리` 화면은 고객사 목록에 소속 테넌트 컬럼과 테넌트 필터를 제공하고, 신규 등록/편집 시 소속 테넌트를 선택한다.
- 사용자 권한은 테넌트 범위 안에서 적용되며, 특정 고객사 접근은 `5-E 담당 법인 권한`에서 사용자-고객사 단위로 추가 제한한다.
- DB/도메인 설계 시 고객사 테이블은 `tenant_id` 또는 `tenant_code` 외래키를 가져야 하며, 고객사 코드는 `tenant_id + customer_code` 범위에서 유일해야 한다.
## 사용자-테넌트-고객사 관리 보완 (2026-05-15)

- 사용자는 반드시 하나의 소속 테넌트(`tenant_id`)를 가진다. 테넌트는 회계법인, 세무법인, 세무사사무소 단위 조직이다.
- 사용자의 고객사 접근 범위는 `user_customer_access` 또는 동등한 매핑으로 관리하며, 한 사용자는 소속 테넌트 안의 여러 고객사에 접근할 수 있다.
- `prototype/index.html`의 `5-B 사용자 관리` 화면은 테넌트 필터, 고객사 필터, 소속 테넌트 컬럼, 고객사 권한 요약을 제공한다.
- 사용자 등록/편집 시 소속 테넌트를 먼저 선택하고, 선택한 테넌트에 속한 고객사만 접근 범위로 선택할 수 있다.
- 역할별 기능 권한은 `5-C 역할 / 권한 매트릭스`에서 관리하고, 특정 고객사 담당 등급 또는 차단 같은 예외는 `5-E 담당 법인 권한`에서 관리한다.
- 권한 판정 순서는 `테넌트 소속 확인 -> 고객사 접근 범위 확인 -> 역할/기능 권한 확인 -> 고객사별 예외 권한 확인`을 기본 원칙으로 한다.
## 고객사별 대상 업무 범위 보완 (2026-05-15)

- 사용자 권한은 `tenant_id + customer_id + work_scope` 조합으로 판정한다. 같은 테넌트와 고객사라도 사용자마다 허용 업무가 다를 수 있다.
- 대상 업무 코드는 `INFO`, `ADJUST`, `FORM`, `VALIDATE`, `APPROVE`, `PRINT`, `EFILE`, `POST`를 기본값으로 사용한다.
- 권한 저장은 `user_customer_work_scope` 테이블을 별도로 두거나, `user_customer_access`에 `work_scopes` JSON/배열 컬럼을 추가하는 방식으로 구현한다.
- `prototype/index.html`의 `5-B 사용자 관리` 화면은 대상 업무 필터와 고객사별 대상 업무 체크리스트를 제공한다.
- 사용자 등록/편집 시 소속 테넌트를 선택한 뒤, 해당 테넌트의 고객사를 선택하고, 선택된 고객사마다 허용 업무 범위를 별도로 지정한다.
- 업무 실행 전 권한 판정 순서는 `테넌트 소속 확인 -> 고객사 접근 확인 -> 고객사별 대상 업무 확인 -> 역할/기능 권한 확인 -> 예외 권한 확인`을 따른다.
- 예: 같은 `EY 회계법인 / ㈜OOO 제조` 고객사라도 A 사용자는 `ADJUST`, `FORM`, `VALIDATE`만 가능하고, B 사용자는 `APPROVE`, `PRINT`만 가능하게 분리할 수 있다.
## Tenant Customer Work Scope Boundary (2026-05-15)

- Each customer now has its own target work scope list (`work_scopes`) inside its tenant.
- User-level `user_customer_work_scope` grants are validated as a subset of the selected customer's target work scopes.
- Effective access is evaluated as tenant membership, customer access, customer target work scope, user customer work scope, role/function permission, then exception permission.
- The prototype and embedded admin UI distinguish customer target work scopes from user-granted work scopes.

## Form Versioning API (2026-05-15)

- Added DB-backed form metadata, form version, template, validation, relationship, and field-migration management.
- Added `/api/form-versioning/resolve` for business-year form version selection.
- Added dry-run, execute, and rollback endpoints for form data migrations.
- Added embedded UI screens for form versions, relationships, migrations, and resolver checks.

## Customer / Business Year Workflow (2026-05-15)

- Business year creation now automatically creates the applicable law, rate, form, and e-filing snapshot.
- Business years follow `DRAFT -> IN_REVIEW -> APPROVED -> FILED -> AMENDED`.
- Moving a business year to `FILED` locks the law snapshot.
- Added embedded UI screens for customer registration, business year creation, status changes, and snapshot inspection.

## Tax Data Input API (2026-05-15)

- Added tenant-scoped import batches, import errors, account mappings, asset ledger, and transaction detail storage.
- Added CSV/XLSX multipart import endpoints for financial statements, assets, and transactions.
- Financial statement imports validate debit/credit totals and store row-level errors when unbalanced.
- Account mappings are learned per customer and reused on later imports to calculate an automatic mapping rate.
- The embedded UI now has separated `tax-data` screens for financial statements, account mappings, transaction details, and assets.

Download a template:

```powershell
Invoke-WebRequest http://localhost:8080/api/tenants/demo/tax-data/templates/financial-statements -OutFile fs-template.csv
```

Upload a CSV file:

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/tax-data/financial-statements/import `
  -Form @{ file = Get-Item .\fs-template.csv }
```

Check validation:

```powershell
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/tax-data/validation
```

## B-1 Income Adjustment API (2026-05-15)

- Added a DB-backed B-1 income adjustment engine.
- The engine can read `NET_INCOME` from imported financial statement lines when `accounting_income` is omitted.
- B-1 sections cover gross income inclusion, gross income exclusion, deductible inclusion, and loss disallowance.
- Reserve dispositions automatically create `reserves` rows tied to the business year and snapshot.
- The embedded UI now has a separated `adjustment` workspace with an income adjustment grid and applicable-law banner.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/income `
  -ContentType "application/json" `
  -Body '{"items":[{"section":"GROSS_INCLUSION","item_code":"B1_TEMP_ADD","item_name":"Temporary addback","amount":10000000,"temporary":true},{"section":"GROSS_EXCLUSION","item_code":"B1_PERM_DEDUCT","item_name":"Permanent exclusion","amount":2000000,"disposition":"OTHER"}]}'

Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/adjustments/income
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/reserves
```

## Asset-Based Adjustment API (2026-05-15)

- Added B-4 depreciation, B-5 retirement reserve, B-6 bad debt reserve, and B-10 business vehicle adjustment endpoints.
- B-4 reads the tenant asset ledger and Phase 4 depreciation-life limits.
- B-10 supports monthly vehicle usage logs and applies business-use ratio plus annual vehicle limits.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/assets/B4 -Body '{}' -ContentType "application/json"
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/vehicle-usage-logs `
  -ContentType "application/json" `
  -Body '{"asset_id":1,"usage_month":"2026-01-01","total_distance_km":1000,"business_distance_km":700}'
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/assets/B10 -Body '{}' -ContentType "application/json"
```

## Transaction-Based Adjustment API (2026-05-15)

- Added B-2 donation, B-3 entertainment expense, and B-9 interest expense adjustment endpoints.
- B-2 reads transaction details, applies law-version donation limits, and creates 10-year donation carryforwards.
- B-3 stores revenue breakdowns and applies law-version entertainment base/revenue limits plus non-card evidence disallowance.
- B-9 classifies interest into unidentified creditor, unidentified recipient, construction financing, non-business asset, and weighted-loan deemed interest buckets.
- The embedded UI now opens the donation/entertainment adjustment menu as a separated transaction-based screen.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/transactions/B2 `
  -ContentType "application/json" `
  -Body '{"taxable_income_before_donation":500000000}'

Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/transactions/B3 `
  -ContentType "application/json" `
  -Body '{"revenue_breakdowns":[{"revenue_category":"PRODUCT","amount":2000000000},{"revenue_category":"SERVICE","amount":1000000000}]}'

Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/transactions/B9 `
  -ContentType "application/json" `
  -Body '{"weighted_average_loan_balance":100000000,"weighted_average_interest_rate_bps":460}'
```

## Evaluation / Carryforward / Reserve API (2026-05-15)

- Added B-7 foreign currency valuation, B-8 inventory/securities valuation, B-11 loss carryforward, and B-15 capital/reserve schedule endpoints.
- B-11 manages yearly loss balances, deduction use, expiration, and SME/general deduction limit rates.
- B-15 aggregates all module reserves for the business year and stores capital changes.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/evaluation/B11 `
  -ContentType "application/json" `
  -Body '{"taxable_income_before_loss":300000000,"loss_carryforwards":[{"origin_year":2025,"original_amount":400000000,"remaining_amount":400000000,"expires_year":2026}]}'

Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/evaluation/B15 `
  -ContentType "application/json" `
  -Body '{"capital_changes":[{"change_date":"2026-06-30","change_type":"PAID_IN_CAPITAL","amount":50000000,"description":"Paid-in capital increase"}]}'
```

## Tax Amount Adjustment API (2026-05-15)

- Added B-12 tax credits/reductions, B-13 minimum tax, and B-14 penalty tax endpoints.
- B-12 returns the calculated-tax to determined-tax flow after allowed credits.
- B-13 stores minimum tax comparison results and additional tax.
- B-14 stores penalty tax items after reduction rates.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/tax/B12 `
  -ContentType "application/json" `
  -Body '{"tax_base":500000000,"calculated_tax":70000000,"credits":[{"credit_type":"RND","base_amount":100000000,"rate_bps":2500}]}'
```

## Special Tax Adjustment API (2026-05-15)

- Added B-16 foreign corporation and B-17 consolidated tax endpoints.
- B-16 stores foreign-source income lines and calculates taxable income, attributable expense, PE allocation, and withholding tax totals.
- B-17 stores consolidated entities plus eliminations and calculates the consolidated tax base.
- The Admin user UI was also updated so tenant/customer/work-scope access can be edited per user, with user scopes limited to each customer's target work scopes.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/special/B16 `
  -ContentType "application/json" `
  -Body '{"foreign_incomes":[{"income_type":"INTEREST","gross_amount":100000000,"attributable_expense":20000000,"pe_allocation_bps":10000,"withholding_tax":5000000}]}'

Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/special/B17 `
  -ContentType "application/json" `
  -Body '{"consolidated_entities":[{"entity_code":"PARENT","entity_name":"Parent Co","ownership_bps":10000,"taxable_income":100000000},{"entity_code":"SUBA","entity_name":"Sub A","ownership_bps":10000,"taxable_income":200000000}],"eliminations":[{"elimination_type":"INTERCOMPANY_PROFIT","amount":50000000,"direction":"DEDUCT"}]}'
```

## Form Engine / FORM3 Preview API (2026-05-15)

- Added FORM3 preview data with field sources, validation issues, and change history.
- `POST /forms/FORM3` regenerates the form from tax adjustment data and applies active form relationships when source form data exists.
- `PUT /forms/FORM3` saves manual field overrides and records `form_data_history`.
- The embedded UI now opens `/modules/forms/form3` as a dedicated preview/edit screen with auto-linked fields visually separated from manual fields.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM15
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM3
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM3/preview
Invoke-RestMethod -Method Put http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM3 `
  -ContentType "application/json" `
  -Body '{"fields":{"tax_credits":3000001},"reason":"manual review adjustment","changed_by":"reviewer"}'
```

## Form Attachments / PDF API (2026-05-15)

- Added a dedicated `6.2 100여 종 부속서식` UI route for attachment status and output actions.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/forms/attachments` returns generated status, validation count, representative amount, and updated time for FORM3, FORM15, and FORM22.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/forms/{form_code}/pdf` generates a PDF with a DRAFT/APPROVED watermark.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/forms/pdf-bundle/download` returns a ZIP bundle of the main form PDFs.

```powershell
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/forms/attachments
Invoke-WebRequest http://localhost:8080/api/tenants/demo/business-years/1/forms/FORM3/pdf -OutFile FORM3.pdf
Invoke-WebRequest http://localhost:8080/api/tenants/demo/business-years/1/forms/pdf-bundle/download -OutFile forms.zip
```

## e-Filing Precheck / Fixed-Width Format API (2026-05-15)

- Added fixed-width e-filing field metadata for the 2026 CIT text format.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/efilings/precheck` returns validation issues, record count, and checksum preview before file generation.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/efilings/format-spec` returns record type, field position, byte length, data type, padding, and source path.
- The embedded UI now opens the `8.1`, `8.2`, and `8.3` e-filing menus as separated screens.

```powershell
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/efilings/precheck
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/efilings/format-spec
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/efilings `
  -ContentType "application/json" `
  -Body '{"max_attempts":3}'
```

## Business-Year Workflow API (2026-05-15)

- Added workflow event and approval-line tracking for business-year status changes.
- Invalid transitions are rejected, while valid transitions record actor/comment metadata.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/workflow` returns status events and approval lines.
- `GET /api/tenants/{tenant_code}/business-years/{by_id}/amendment-preview` returns amendment-mode differences.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/status `
  -ContentType "application/json" `
  -Body '{"status":"IN_REVIEW","actor":"writer01","approver":"reviewer01","comment":"submit"}'

Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/workflow
Invoke-RestMethod http://localhost:8080/api/tenants/demo/business-years/1/amendment-preview
```
