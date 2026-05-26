# CIT 법인세 세무조정 시스템

CIT는 법인세 세무조정계산서 작성, 기초자료 수집, 세무조정 계산, 검증, 결재, 출력, 전자신고 파일 생성, 신고 후 관리를 하나의 흐름으로 처리하는 멀티테넌트 업무 시스템입니다. Rust/Axum API 서버가 PostgreSQL에 업무 데이터를 저장하고, 별도 번들러 없이 내장 SPA 프론트엔드를 같은 HTTP 서버에서 제공합니다.

최종 정리일: 2026-05-26

## 1. 시스템 목적

이 시스템은 회계법인, 세무법인, 세무대리인이 여러 고객 법인의 법인세 신고 업무를 표준화된 절차로 관리하도록 설계되었습니다.

- 테넌트별 사용자, 역할, 권한, 메뉴, 고객 접근 범위를 분리합니다.
- 고객별 사업연도를 생성하고 법령, 세율, 서식, 전자신고 기준을 사업연도 단위로 고정합니다.
- 재무제표, 자산, 거래명세, 계정 매핑, 차량 운행기록 등 세무조정 기초자료를 입력하거나 가져옵니다.
- B1부터 B17까지의 세무조정 모듈을 계산하고 조정 항목, 이력, 증빙 메타데이터를 저장합니다.
- 주요 신고 서식을 자동 생성하고 수동 보정, 검증, PDF 출력, 출력 이력을 관리합니다.
- 검토와 결재, 신고 완료, 수정신고, 경정청구 흐름을 업무 상태로 추적합니다.
- 관리자 화면에서 보안, 권한, 메뉴, 고객 접근, 법령/세율, 서식 버전, 감사 로그를 운영합니다.

## 2. 전체 아키텍처

```text
Browser
  |
  | HTML / CSS / native ES modules
  v
Embedded SPA frontend
  |
  | /api/*
  v
Rust Axum API server
  |
  | SQLx
  v
PostgreSQL

Rust Axum API server
  |
  | ENABLE_WORKER=true
  v
Durable job worker
```

| 구성 요소 | 책임 |
|---|---|
| `api` | Rust 바이너리 `cit-system`입니다. HTTP API, 정적 UI, 선택적 background worker를 함께 실행합니다. |
| `postgres` | 공통 마스터 데이터, 테넌트별 업무 스키마, 권한, 로그, job queue를 저장합니다. |
| `frontend` | 브라우저 native ES module 기반 SPA입니다. `src/web.rs`에서 `include_str!`로 Rust 바이너리에 내장합니다. |
| `worker` | 별도 서비스가 아니라 API 프로세스 내부 task입니다. `ENABLE_WORKER=true`이면 durable job을 polling합니다. |
| `seed-demo` | 데모 테넌트, 사용자, 고객, 사업연도, 샘플 업무 데이터를 생성하는 CLI입니다. |
| `test` / `clippy` | Docker Compose 기반 Rust 테스트와 정적 검증 서비스입니다. |

프론트엔드 파일은 빌드 산출물로 복사하지 않습니다. `frontend/*`를 수정한 뒤 Docker 환경의 API에 반영하려면 API 이미지를 다시 빌드하거나 Rust 서버를 재시작해야 합니다.

## 3. 업무 상태와 결재 흐름

기본 사업연도 상태는 다음 전이를 따릅니다.

```text
DRAFT -> IN_REVIEW -> APPROVED -> FILED -> AMENDED
```

| 상태 | 의미 |
|---|---|
| `DRAFT` | 기초자료 입력, 세무조정 계산, 서식 작성이 가능한 작성 중 상태입니다. |
| `IN_REVIEW` | 검토 또는 결재 대기 상태입니다. 승인자 라인과 workflow event가 생성됩니다. |
| `APPROVED` | 결재가 완료되어 출력과 전자신고 준비가 가능한 상태입니다. |
| `FILED` | 신고 완료 상태입니다. 주요 업무 데이터는 기본적으로 잠금 처리합니다. |
| `AMENDED` | 신고 후 수정신고 또는 경정청구를 위해 잠금을 해제한 상태입니다. |

대시보드 결재 대기 영역은 `workflow/queue` 데이터를 사용합니다. 승인 또는 반려 인라인 액션을 실행하면 기존 사업연도 상태 API가 호출되고, `business_years.status`, `approval_lines.status`, 업무현황 수치, 알림 목록이 함께 갱신됩니다.

## 4. 주요 도메인 모델

| 모델 | 설명 |
|---|---|
| `Tenant` | 회계법인, 세무법인, 세무대리인 조직 단위입니다. 각 테넌트는 자체 업무 스키마를 가집니다. |
| `User` | 테넌트에 속한 사용자입니다. 로그인, 2FA, 잠금, 역할, 고객 접근 범위가 연결됩니다. |
| `Role` / `Permission` | 기능 코드, 메뉴 기능, 데이터 범위, 필드 마스킹 권한을 제어합니다. |
| `Customer` | 테넌트가 관리하는 고객 법인입니다. 업종, 사업자번호, 업무 범위를 가집니다. |
| `BusinessYear` | 고객 법인의 신고 대상 사업연도입니다. 상태, 잠금, workflow, snapshot의 기준 단위입니다. |
| `Law/Form/E-file Snapshot` | 사업연도 생성 또는 진행 시점의 법령, 세율, 서식, 전자신고 기준을 고정합니다. |
| `TaxData` | 재무제표, 자산, 거래, 계정 매핑, 차량 운행기록 등 입력 데이터입니다. |
| `Adjustment` | B1부터 B17까지의 세무조정 계산 결과와 조정 항목입니다. |
| `FormData` | 신고 서식 자동 생성 필드, 수동 보정 필드, 검증 이력입니다. |
| `Workflow` | 결재 요청, 승인, 반려, 신고 완료, 수정신고 전환 이벤트입니다. |
| `Notification` | 마감 알림, 업무 알림, 읽음 상태, 관련 사업연도 메타데이터입니다. |
| `AuditLog` | 업무 데이터 변경과 운영 이벤트를 추적하는 감사 로그입니다. |
| `Job` | 전자신고 파일 생성 등 비동기 작업과 retry/dead-letter 상태입니다. |

## 5. 데이터베이스 설계

PostgreSQL은 공통 public 영역과 테넌트별 업무 스키마를 함께 사용합니다.

| 영역 | 주요 테이블 |
|---|---|
| 공통 마스터 | `tenants`, `users`, `auth_sessions`, `roles`, `user_roles`, `role_permissions`, `menu_nodes`, `function_codes` |
| 법령/서식 | `tax_law_versions`, `tax_rates`, `tax_limits`, `tax_forms`, `form_versions`, `form_relationships`, `efile_masters`, `efile_record_fields` |
| 운영/보안 | `login_history`, `admin_audit_events`, `access_delegations`, `field_masking_policies`, `jobs` |
| 테넌트 업무 | `{tenant_schema}.customers`, `business_years`, `workflow_events`, `approval_lines`, `audit_logs`, `notifications` |
| 입력자료 | `financial_statements`, `assets`, `transactions`, `import_batches`, `import_errors`, `account_mappings`, `vehicle_usage_logs` |
| 세무조정 | `tax_adjustments`, `adjustment_items`, `adjustment_items_history`, `reserves`, `donation_carryforwards`, `depreciation` |
| 서식/전자신고 | `form_data`, `form_data_history`, `print_history`, `efiling_history`, `efiling_files` |

마이그레이션은 `migrations/`에 날짜 순서로 배치되어 있고, 서버 시작 시 `sqlx::migrate!` 흐름으로 적용됩니다. 테넌트별 스키마는 테넌트 생성 또는 마이그레이션 과정에서 보강됩니다.

## 6. 백엔드 구성

| 파일 | 책임 |
|---|---|
| `src/main.rs` | 환경 설정 로드, DB 연결, migration 실행, worker 시작, HTTP 서버 실행 |
| `src/config.rs` | `DATABASE_URL`, `APP_HOST`, `APP_PORT`, worker, CORS 설정 |
| `src/api.rs` | Axum route 정의와 HTTP handler |
| `src/auth.rs` | 로그인, 세션, 테넌트 전환, 2FA, IP allowlist, 계정 잠금 |
| `src/admin.rs` | 사용자, 역할, 권한, 메뉴, 관리자 기능 |
| `src/tenant.rs` | 테넌트, 고객, 사업연도, workflow, dashboard, 알림, 리포트 persistence |
| `src/tax_data.rs` | 기초자료 템플릿, CSV import, 계정 매핑, 입력 검증 |
| `src/tax.rs` | 법령/세율, snapshot, B1-B17 조정 계산, 서식 생성, 출력 |
| `src/forms.rs` | 서식 마스터, 버전, 필드, 검증 규칙, migration dry-run/execute/rollback |
| `src/efiling.rs` | 전자신고 사전검증, Windows-949 파일 생성, 파일 조회 |
| `src/queue.rs` | durable job queue, retry, backoff, dead-letter, worker loop |
| `src/permissions.rs` | 역할/메뉴/데이터 범위 기반 effective permission 계산 |
| `src/menu.rs` / `src/modules.rs` | 메뉴 트리와 모듈 정의 |
| `src/scheduler.rs` | 마감 알림 등 운영 batch 로직 |
| `src/validation_rules.rs` | 업무 검증 rule 실행 |
| `src/web.rs` | 내장 SPA 정적 파일 serving |
| `src/seed.rs` | 데모 테넌트와 샘플 업무 데이터 seed |

## 7. 프론트엔드 구성

프론트엔드는 별도 빌드 도구 없이 브라우저 native ES module로 동작합니다.

| 파일 | 책임 |
|---|---|
| `frontend/index.html` | 앱 shell과 로그인 화면 mount point |
| `frontend/app.css` | 전체 레이아웃, dashboard, workbench, grid, admin 화면 스타일 |
| `frontend/app.js` | 앱 부트스트랩, 세션 복원, route rendering, locale 전환 |
| `frontend/app/api.js` | API request wrapper, 인증 실패 처리 |
| `frontend/app/context.js` | 선택 고객과 사업연도 업무 context 저장 |
| `frontend/app/router.js` | hash route와 leaf key 변환 |
| `frontend/app/menu.js` | 메뉴 트리, tenant switcher, context badge, stepper 렌더링 |
| `frontend/app/i18n.js` | 한국어/영어 문구와 route label |
| `frontend/app/screens.js` | 주요 업무 화면 renderer, leaf route registry, dashboard, admin workbench |
| `frontend/app/components/grid.js` | 공통 grid/editor 컴포넌트 |

UI는 leaf route 중심으로 구성됩니다. 예를 들어 `ws/adj:B1`, `ws/form:preview`, `admin/sec:users` 같은 leaf key가 메뉴, API, 권한, 화면 renderer를 연결합니다.

## 8. 업무 화면 구성

| 영역 | 역할 |
|---|---|
| 대시보드 | 업무 상태 카드, 신고 마감 목록, 최근 알림, 결재 대기, 최근 활동, KPI 진입점을 제공합니다. |
| 작업 시작 | 고객사 선택, 신규 고객 등록, 사업연도 생성, 전년 이월, 적용 기준 확인을 처리합니다. |
| 세무정보 입력 | 재무제표, 계정 매핑, 자산, 거래, 차량 운행기록, 입력 검증을 처리합니다. |
| 세무조정 | B1-B17 조정 모듈별 입력, 계산, 결과, 이력, 증빙 흐름을 제공합니다. |
| 서식 작성 | 주요 신고 서식 생성, 미리보기, 자동/수동 필드, 서식 연결 검토를 제공합니다. |
| 검증 | 검증 규칙 실행, 오류/경고 triage, 원천 화면 이동, dismiss를 제공합니다. |
| 결재 | 결재 요청, 승인자 처리, 반려, 사업연도 상태 전이를 처리합니다. |
| 출력 | PDF 미리보기, 개별 PDF, 일괄 ZIP, 출력 이력을 관리합니다. |
| 전자신고 | 사전검증, 전자신고 파일 생성, 파일 다운로드, 제출 상태와 접수 이력을 관리합니다. |
| 신고 후 관리 | 신고 이력, 수정신고 잠금 해제, 버전 비교, 재제출을 처리합니다. |
| 분석/리포트 | 세부담 분석, 연도 비교, 충당금/결손금 리포트, 사용자 정의 리포트를 제공합니다. |
| 관리자 설정 | 테넌트, 고객, 사용자, 권한, 메뉴, 법령/세율, 서식 버전, 감사 로그를 관리합니다. |

## 9. 세무조정 모듈

세무조정은 B1부터 B17까지의 모듈 코드로 관리합니다.

| 코드 | 모듈 |
|---|---|
| B1 | 소득금액조정명세서 |
| B2 | 기부금 |
| B3 | 접대비 |
| B4 | 감가상각비 |
| B5 | 퇴직급여충당금과 퇴직연금 |
| B6 | 대손충당금과 대손금 |
| B7 | 통화자산/부채 평가 |
| B8 | 재고자산/유가증권 평가 |
| B9 | 지급이자 손금불산입 |
| B10 | 업무용승용차 관련 비용 |
| B11 | 이월결손금 |
| B12 | 세액공제와 감면 |
| B13 | 최저한세 |
| B14 | 가산세 |
| B15 | 자본금과 적립금 |
| B16 | 외국법인 세무조정 |
| B17 | 연결납세 |

각 모듈은 사업연도 context 안에서 입력 payload, 계산 결과, 조정 항목, 이력, 증빙 attachment metadata를 관리합니다.

## 10. 주요 API 영역

모든 `/api/*` 요청은 `/api/auth/login` 등 일부 public endpoint를 제외하고 Bearer token 인증을 사용합니다.

| 영역 | 대표 endpoint |
|---|---|
| 상태 확인 | `GET /health`, `GET /ready` |
| 인증 | `POST /api/auth/login`, `GET /api/auth/me`, `POST /api/auth/switch-tenant`, `POST /api/auth/logout` |
| 모듈/메뉴 | `GET /api/modules/tree`, `GET /api/modules/legacy-tree` |
| 테넌트 | `GET/POST /api/tenants`, `PATCH /api/tenants/{tenant}/status`, `PATCH /api/tenants/{tenant}/plan` |
| 고객 | `GET/POST /api/tenants/{tenant}/customers` |
| 사업연도 | `GET/POST /api/tenants/{tenant}/business-years`, `POST /business-years/{by_id}/status`, `GET /snapshot`, `GET /progress` |
| 대시보드 | `GET /api/tenants/{tenant}/dashboard`, `GET /api/tenants/{tenant}/dashboard/filing-deadlines` |
| 알림/감사 | `GET /api/tenants/{tenant}/notifications`, `PATCH /notifications/{id}`, `GET /audit-logs` |
| 기초자료 | `GET /tax-data/templates/{type}`, `POST /tax-data/{type}/import`, `GET /tax-data/validation` |
| 세무조정 | `GET/POST /adjustments`, `GET/POST /adjustments/{category}/{module_code}`, `GET /adjustments/history` |
| 서식 | `GET/POST/PUT /forms/{form_code}`, `GET /forms/{form_code}/preview`, `GET /forms/{form_code}/pdf` |
| 검증 | `POST /validation/run`, `GET /validation/issues`, `POST /validation/issues/{id}/dismiss` |
| 결재 | `GET /workflow/queue`, `GET/POST /workflow/events`, `POST /workflow/request` |
| 출력 | `GET /forms/attachments`, `GET /forms/pdf-bundle/download`, `GET /print/history` |
| 전자신고 | `GET /efilings/precheck`, `GET /efilings/format-spec`, `POST /efilings`, `POST /efilings/{id}/submit` |
| 신고 후 관리 | `GET /amendment-preview`, `POST /unlock`, `POST /resubmit`, `GET /amendment-version-mode` |
| 관리자 | `/api/admin/*`, `/api/tax-laws`, `/api/tax-rates`, `/api/tax-limits`, `/api/form-versioning/*` |
| 리포트/운영 | `/reports/*`, `/api/jobs`, `/api/operations/*` |

테넌트 업무 API 대부분은 `/api/tenants/{tenant_code}/...` 형태이며, 사업연도 업무는 추가로 `business-years/{by_id}` context를 요구합니다.

## 11. 보안과 권한

보안 모델은 인증, 테넌트 접근, 역할 권한, 메뉴 권한, 고객 접근 범위, 데이터 잠금을 함께 적용합니다.

- 로그인은 `tenant_code`, `login_id`, `password`를 기준으로 수행합니다.
- 세션은 Bearer token으로 `/api/*` 요청에 전달합니다.
- 2FA 설정 사용자는 TOTP challenge를 통과해야 합니다.
- 테넌트별 IP allowlist를 검증할 수 있습니다.
- 로그인 실패 횟수가 임계값을 넘으면 계정이 잠깁니다.
- 역할 권한은 module/function 단위의 ALLOW/DENY 정책을 사용합니다.
- 메뉴 기능 권한은 leaf route와 화면 action 노출을 제어합니다.
- 고객 접근 권한은 고객별 업무 범위, 위임, 예외 규칙을 함께 평가합니다.
- 필드 마스킹과 데이터 scope 정책은 관리자 설정으로 관리됩니다.
- `FILED` 사업연도는 신고 후 관리 흐름을 제외하고 주요 편집 작업을 제한합니다.

## 12. 알림, 감사, 배치 작업

운영성 기능은 업무 흐름과 분리해 저장하고 조회합니다.

- `notifications`는 마감 알림, 결재 알림, 업무 알림, 읽음 상태, 관련 사업연도 metadata를 저장합니다.
- `ensure_due_notifications`는 scheduler에서 신고 마감이 임박한 사업연도의 알림을 생성합니다.
- 결재 승인/반려는 workflow event, approval line, 사업연도 상태, dashboard 알림을 함께 갱신합니다.
- `audit_logs`는 테넌트 업무 데이터 변경과 주요 상태 전이를 기록합니다.
- `jobs`는 전자신고 파일 생성 등 비동기 작업을 저장하고 retry/dead-letter 상태를 관리합니다.
- worker는 `JOB_POLL_SECONDS` 간격으로 실행 가능한 job을 가져와 처리합니다.

## 13. 실행 방법

### Docker Compose

```powershell
Copy-Item .env.example .env
docker compose up --build -d postgres api
docker compose run --rm test cargo run --bin seed-demo -- --reset
```

API와 UI는 같은 주소에서 열립니다.

```text
http://localhost:8080/
```

상태 확인:

```powershell
Invoke-RestMethod http://localhost:8080/health
Invoke-RestMethod http://localhost:8080/ready
```

데모 로그인:

```text
Tenant: demo
ID: admin
Password: ChangeMe123!
```

### 로컬 Rust 실행

PostgreSQL이 실행 중이고 `.env` 또는 환경 변수에 `DATABASE_URL`이 설정되어 있어야 합니다.

```powershell
$env:DATABASE_URL = "postgres://cit:cit@localhost:5432/cit"
$env:APP_HOST = "127.0.0.1"
$env:APP_PORT = "8080"
$env:ENABLE_WORKER = "true"
cargo run --bin cit-system
```

데모 데이터 seed:

```powershell
cargo run --bin seed-demo -- --reset
```

### 주요 환경 변수

| 변수 | 기본값 예시 | 설명 |
|---|---|---|
| `DATABASE_URL` | `postgres://cit:cit@localhost:5432/cit` | PostgreSQL 연결 문자열입니다. 필수입니다. |
| `APP_HOST` | `127.0.0.1` 또는 `0.0.0.0` | HTTP bind host입니다. |
| `APP_PORT` | `8080` | HTTP port입니다. |
| `ENABLE_WORKER` | `true` | durable job worker 실행 여부입니다. |
| `JOB_POLL_SECONDS` | `2` | worker polling 간격입니다. |
| `ALLOWED_ORIGINS` | `http://localhost:8080,http://127.0.0.1:8080` | CORS 허용 origin 목록입니다. |
| `RUST_LOG` | `cit_system=info,tower_http=info` | tracing 로그 필터입니다. |

## 14. 테스트와 검증

프론트엔드 정적/계약 테스트:

```powershell
npm run test:frontend
```

Docker 기반 Rust 테스트와 포맷 검증:

```powershell
docker compose run --rm test cargo fmt --check
docker compose run --rm test cargo test --all --all-targets
docker compose run --rm clippy
```

주요 통합/회귀 테스트:

```powershell
docker compose run --rm test cargo test --test integration_flow -- --nocapture
docker compose run --rm test cargo test --test menu_smoke -- --nocapture
docker compose run --rm test cargo test --test adjustment_modules_all -- --nocapture
docker compose run --rm test cargo test --test lock_and_2fa -- --nocapture
docker compose run --rm test cargo test --test admin_tenant_routing -- --nocapture
docker compose run --rm test cargo test --test form_versioning_ui -- --nocapture
docker compose run --rm test cargo test --test dashboard_work_status -- --nocapture
docker compose run --rm test cargo test --test workflow_transition -- --nocapture
```

브라우저 smoke 테스트는 API가 실행 중이어야 하며 Microsoft Edge CDP를 사용합니다.

```powershell
node tests/frontend/browser_smoke_phase9_10.mjs
```

캡처 결과는 `docs/phase9_10/` 아래에 저장됩니다.

## 15. Repository Layout

```text
D:\NEW_CIT
  frontend/                 Embedded SPA source
  frontend/app/             ES module application code
  migrations/               PostgreSQL schema and seed migrations
  src/                      Rust backend source
  src/bin/seed_demo.rs      Demo data seed CLI
  tests/                    Rust integration and regression tests
  tests/frontend/           Frontend contract and smoke tests
  docs/                     Evidence, manuals, phase documents, runbooks
  examples/                 Sample API payloads and scripts
  prototype/                Standalone prototype artifact
  docker-compose.yml        Local PostgreSQL/API/test/clippy services
  Dockerfile                API container image
  Cargo.toml                Rust package definition
  package.json              Frontend test scripts
  .env.example              Local environment variable template
```

## 16. 개발 시 주의사항

- 프론트엔드는 Rust 바이너리에 내장되므로 `frontend/*` 수정 후 Docker API에 반영하려면 `docker compose up --build -d api`가 필요합니다.
- 모든 테넌트 업무 API는 테넌트와 사업연도 context에 민감합니다. UI는 `cit.work.context`에 선택 고객사와 사업연도를 저장합니다.
- `FILED` 상태의 사업연도는 잠금이 걸립니다. 수정신고 또는 경정청구는 전용 unlock/resubmit API로 진행해야 합니다.
- 법령, 세율, 서식, 전자신고 기준은 사업연도 snapshot으로 고정합니다. 진행 중인 사업연도에는 버전 migration 또는 clone 정책을 사용해야 합니다.
- 권한은 역할 권한, 메뉴 기능 권한, 고객 접근 권한, 데이터 scope, 필드 마스킹이 함께 적용됩니다.
- 비동기 job은 실패 시 retry 후 dead-letter로 이동합니다. 운영 중에는 `/api/jobs`와 worker 로그를 함께 확인해야 합니다.
- README는 전체 구조와 진입점 문서입니다. 상세 업무 규칙은 `법인세_세무조정계산서_시스템_설계서.md`, `DB설계.md`, `업무흐름.md`, `세부구현_v1.0.md`를 함께 참고합니다.
