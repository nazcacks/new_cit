# CIT Corporate Income Tax Adjustment System

새로운 법인세개발

Rust implementation of the 법인세 세무조정계산서 system described in `법인세_세무조정계산서_시스템_설계서.md`.

## What Is Implemented

- PostgreSQL-backed multi-tenant API with one schema per tenant.
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
  -Body '{"customer_code":"CUST001","customer_name":"서울테크 주식회사","biz_reg_no":"2208112345","corp_reg_no":"1101111234567","industry_code":"62010","is_sme":true}'
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
- `src/tax.rs`: law snapshots, tax calculation, adjustment persistence, form generation.
- `src/efiling.rs`: fixed-width Windows-949 e-filing generation.
- `src/queue.rs`: durable retry/DLQ worker.
- `migrations/`: PostgreSQL schema and seed tax/form/e-file metadata.
- `tests/integration_flow.rs`: full PostgreSQL integration test.
- `examples/`: sample API payloads and PowerShell flow.
