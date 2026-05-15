# 법인세 세무조정계산서 시스템 — DB 설계서

> **문서 버전**: v1.0
> **작성일**: 2026-05-14
> **작성자**: 강수 (Kang-Soo.Cho@kr.ey.com)
> **연관 문서**: `법인세_세무조정계산서_시스템_설계서.md` (메인 설계서)
> **대상 DBMS**: PostgreSQL 16
> **멀티테넌트 전략**: Schema-per-Tenant (`public` + `tenant_NNN`)

---

## 목차

1. [DB 설계 원칙](#1-db-설계-원칙)
2. [스키마 구조](#2-스키마-구조)
3. [테이블 카테고리](#3-테이블-카테고리)
4. [핵심 ERD](#4-핵심-erd)
5. [테이블 상세 (DDL)](#5-테이블-상세-ddl)
    - 5.1 시스템/테넌트
    - 5.2 사용자 / 인증
    - 5.3 권한 / 메뉴 / 기능
    - 5.4 사용자별 담당 법인 권한
    - 5.5 고객사 / 사업연도
    - 5.6 세무정보 / 자산
    - 5.7 세무조정 / 유보
    - 5.8 서식
    - 5.9 법령·세율 버전 관리
    - 5.10 전자신고
    - 5.11 감사·이력
6. [뷰 / 함수 / 트리거](#6-뷰--함수--트리거)
7. [인덱스 · 파티셔닝 전략](#7-인덱스--파티셔닝-전략)
8. [보안 · 암호화](#8-보안--암호화)
9. [백업 / 운영](#9-백업--운영)
10. [부록 — 명명 규칙 / 데이터 타입](#10-부록--명명-규칙--데이터-타입)

---

## 1. DB 설계 원칙

| 원칙 | 적용 |
|------|------|
| 멀티 테넌트 격리 | 회계법인별 Schema 분리 (Schema-per-Tenant), `public` 스키마에 메타·시스템 테이블만 |
| 시점성 (Temporal) | 세법·세율·한도는 `effective_from / effective_to`로 시점별 보관, 신고분은 스냅샷 잠금 |
| 불변성 (Immutable) | 신고 완료된 사업연도의 데이터·법령 스냅샷은 사후 변경 불가 |
| 감사성 | 모든 변경(C/U/D)은 `audit_logs`에 기록, 권한·법령은 별도 이력 테이블 |
| 행 단위 보안 (RLS) | 사용자별 담당 법인 권한 뷰 + Hibernate Filter로 자동 WHERE 적용 |
| 컬럼 암호화 | 개인정보·세무정보 민감 컬럼은 AES-256-GCM 컬럼 암호화, 마스터키는 KMS |
| 명명 일관성 | snake_case, PK = `{entity}_id`, 외래키 = 참조 PK 동일 명, 시간 컬럼 = `_at` |
| 가변 스키마 | 매년 변경되는 서식 구조는 JSONB 컬럼(`template_json`, `data_json`)으로 흡수 |
| 추적성 | 모든 계산 결과 행에 `snapshot_id`(법령 스냅샷) 참조 → 사후 감사·재현 |

---

## 2. 스키마 구조

```
PostgreSQL Cluster
├── public (메타 스키마)
│   ├── tenants                       -- 회계법인/세무법인 마스터
│   ├── users                         -- 전체 사용자 (테넌트 매핑)
│   ├── login_history
│   ├── system_logs
│   ├── tax_law_versions, tax_rates, tax_limits, ...   -- 법령 마스터 (전 테넌트 공통)
│   ├── form_versions, efile_masters, efile_detail_forms, efile_record_layouts, efile_record_fields
│   └── code_groups, codes            -- 표준 코드
├── tenant_001 (A 회계법인 스키마)
│   ├── roles, menus, functions, role_menu_function, ...
│   ├── customers, business_years
│   ├── financial_statements, transactions, assets, depreciation
│   ├── tax_adjustments, reserves, carryforward_loss
│   ├── tax_forms, form_data
│   ├── efiling_history
│   ├── audit_logs
│   └── ...
├── tenant_002 (B 세무법인)
└── tenant_N
```

- 로그인 시 사용자 소속 테넌트 식별 → JWT에 tenant_id 포함
- Hibernate `CurrentTenantIdentifierResolver`가 자동으로 스키마 라우팅
- 테넌트 간 데이터 접근 원천 차단

---

## 3. 테이블 카테고리

| 카테고리 | 주요 테이블 |
|---------|------------|
| 시스템/공통 (public) | `tenants`, `users`, `login_history`, `system_logs` |
| 권한·메뉴·기능 (tenant) | `roles`, `menus`, `functions`, `menu_functions`, `user_role`, `role_menu_function`, `user_menu_function_override`, `user_data_scope`, `field_permissions`, `permission_change_history` |
| 사용자별 담당 법인 권한 | `user_customer_access`, `customer_groups`, `customer_group_members`, `user_customer_group_access`, `customer_access_rules`, `access_delegations`, `customer_access_history` |
| 고객사 | `customers`, `business_years`, `tax_agents`, `customer_users` |
| 세무정보 | `financial_statements`, `fs_lines`, `account_mappings`, `transactions`, `assets`, `depreciation` |
| 세무조정 | `tax_adjustments`, `adjustment_items`, `reserves`(유보), `carryforward_loss` |
| 서식 (★ 시점별 버전) | `tax_forms` (마스터), `form_versions` (시점별 버전), `form_templates` (메타 + PDF), `form_validations`, `form_field_references` (필드 참조), `form_field_migration` (전→신 매핑), `form_data` (사업연도별 데이터), `form_data_migration_history`, `form_relationships` (서식 간 연동) |
| 법령·세율 버전 (public) | `tax_law_versions`, `tax_rates`, `tax_limits`, `tax_credits_versions`, `depreciation_lives`, `sme_criteria`, `loss_carryforward_rules`, `form_versions`, `efile_masters`, `efile_detail_forms`, `efile_record_layouts`, `efile_record_fields`, `by_law_snapshot`, `law_amendment_history` |
| 전자신고 | `efiling_history`, `efiling_history_masters`, `efiling_files`, `efiling_validation` |
| 코드 | `code_groups`, `codes` (업종/계정/세율 등) |
| 감사·이력 | `audit_logs`, `customer_access_history`, `permission_change_history`, `law_amendment_history` |

---

## 4. 핵심 ERD

```
┌──────────┐ 1   N ┌──────────┐ N   M ┌──────────┐
│ tenants  │───────│  users   │───────│  roles   │
└──────────┘       └──────────┘       └──────────┘
     │ 1                │ N                │ M
     │                  │                  │
     │ N                ▼                  ▼
┌──────────┐    ┌─────────────┐    ┌──────────────┐
│customers │    │user_customer│    │role_menu_    │
│          │◀──▶│_access      │    │function      │
└──────────┘    └─────────────┘    └──────────────┘
     │ 1                                    │
     │ N                                    ▼
┌──────────────┐               ┌──────────────────┐
│business_years│               │ menus / functions│
└──────────────┘               └──────────────────┘
     │ 1
     ├──────────────┬───────────────┬───────────────┐
     ▼ N           ▼ N             ▼ N             ▼ N
┌──────────┐ ┌─────────────┐ ┌──────────┐  ┌──────────┐
│financial_│ │tax_         │ │tax_forms │  │ efiling_ │
│statements│ │adjustments  │ │ + form_  │  │ history  │
└──────────┘ └─────────────┘ │   data   │  └──────────┘
                    │ 1     └──────────┘
                    ▼ N             │
              ┌──────────┐          │
              │ reserves │          │
              └──────────┘          │
                                    ▼
                          ┌────────────────────┐
                          │ by_law_snapshot    │ ← tax_law_versions
                          │  (불변, 사업연도   │   tax_rates / tax_limits
                          │   적용 스냅샷)     │   tax_credits / ...
                          └────────────────────┘
```

---

## 5. 테이블 상세 (DDL)

### 5.1 시스템 / 테넌트

#### tenants

```sql
CREATE TABLE tenants (
    tenant_id       BIGSERIAL PRIMARY KEY,
    tenant_code     VARCHAR(20) UNIQUE NOT NULL,
    tenant_name     VARCHAR(200) NOT NULL,
    biz_reg_no      VARCHAR(13) NOT NULL,    -- 사업자등록번호 (암호화)
    contract_start  DATE NOT NULL,
    contract_end    DATE,
    schema_name     VARCHAR(50) UNIQUE NOT NULL,
    status          VARCHAR(20) DEFAULT 'ACTIVE',
    allowed_ips     TEXT,                    -- 허용 IP CIDR
    max_users       INT DEFAULT 10,
    created_at      TIMESTAMP DEFAULT NOW(),
    updated_at      TIMESTAMP DEFAULT NOW()
);
```

### 5.2 사용자 / 인증

#### users

```sql
CREATE TABLE users (
    user_id         BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT NOT NULL REFERENCES tenants,
    login_id        VARCHAR(50) NOT NULL,
    password_hash   VARCHAR(255) NOT NULL,   -- bcrypt
    user_name       VARCHAR(100) NOT NULL,
    email           VARCHAR(200),            -- 암호화
    phone           VARCHAR(20),             -- 암호화
    totp_secret     VARCHAR(255),            -- 2FA 시크릿 (암호화)
    use_2fa         BOOLEAN DEFAULT TRUE,
    pwd_changed_at  TIMESTAMP,
    pwd_fail_count  INT DEFAULT 0,
    locked          BOOLEAN DEFAULT FALSE,
    last_login_at   TIMESTAMP,
    last_login_ip   VARCHAR(45),
    status          VARCHAR(20) DEFAULT 'ACTIVE',
    created_at      TIMESTAMP DEFAULT NOW(),
    UNIQUE(tenant_id, login_id)
);
```

#### login_history

```sql
CREATE TABLE login_history (
    history_id      BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users,
    login_at        TIMESTAMP DEFAULT NOW(),
    ip_address      VARCHAR(45),
    user_agent      VARCHAR(500),
    success         BOOLEAN,
    fail_reason     VARCHAR(200),
    session_id      VARCHAR(100)
);
CREATE INDEX idx_login_history_user ON login_history(user_id, login_at DESC);
```

### 5.3 권한 / 메뉴 / 기능

#### menus

```sql
CREATE TABLE menus (
    menu_id         BIGSERIAL PRIMARY KEY,
    menu_code       VARCHAR(50) UNIQUE NOT NULL,
    menu_name       VARCHAR(200) NOT NULL,
    parent_menu_id  BIGINT REFERENCES menus,
    menu_url        VARCHAR(300),
    menu_icon       VARCHAR(100),
    sort_order      INT,
    is_active       BOOLEAN DEFAULT TRUE,
    description     TEXT,
    created_at      TIMESTAMP DEFAULT NOW()
);
```

#### functions (공통 기능 권한)

```sql
CREATE TABLE functions (
    function_id     BIGSERIAL PRIMARY KEY,
    function_code   VARCHAR(30) UNIQUE NOT NULL,  -- READ/CREATE/UPDATE/DELETE/PRINT/EXPORT/IMPORT/APPROVE/SUBMIT/UNLOCK/LOG_VIEW/MASK_OFF
    function_name   VARCHAR(100) NOT NULL,
    description     VARCHAR(500),
    is_system       BOOLEAN DEFAULT TRUE
);
```

#### menu_functions (메뉴별 활성 기능)

```sql
CREATE TABLE menu_functions (
    menu_function_id BIGSERIAL PRIMARY KEY,
    menu_id          BIGINT REFERENCES menus,
    function_id      BIGINT REFERENCES functions,
    is_enabled       BOOLEAN DEFAULT TRUE,
    UNIQUE(menu_id, function_id)
);
```

#### roles

```sql
CREATE TABLE roles (
    role_id         BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT REFERENCES tenants,
    role_code       VARCHAR(50) NOT NULL,
    role_name       VARCHAR(100) NOT NULL,
    description     VARCHAR(500),
    is_system_role  BOOLEAN DEFAULT FALSE,
    created_at      TIMESTAMP DEFAULT NOW(),
    UNIQUE(tenant_id, role_code)
);
```

#### user_role

```sql
CREATE TABLE user_role (
    user_id         BIGINT REFERENCES users,
    role_id         BIGINT REFERENCES roles,
    assigned_by     BIGINT REFERENCES users,
    assigned_at     TIMESTAMP DEFAULT NOW(),
    expires_at      TIMESTAMP,
    PRIMARY KEY (user_id, role_id)
);
```

#### role_menu_function ★ 핵심

```sql
CREATE TABLE role_menu_function (
    rmf_id          BIGSERIAL PRIMARY KEY,
    role_id         BIGINT REFERENCES roles,
    menu_id         BIGINT REFERENCES menus,
    function_id     BIGINT REFERENCES functions,
    permission      VARCHAR(10) NOT NULL,    -- ALLOW / DENY
    created_at      TIMESTAMP DEFAULT NOW(),
    UNIQUE(role_id, menu_id, function_id)
);
CREATE INDEX idx_rmf_role ON role_menu_function(role_id);
```

#### user_menu_function_override

```sql
CREATE TABLE user_menu_function_override (
    override_id     BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users,
    menu_id         BIGINT REFERENCES menus,
    function_id     BIGINT REFERENCES functions,
    permission      VARCHAR(10) NOT NULL,    -- DENY 우선
    reason          VARCHAR(500),
    granted_by      BIGINT REFERENCES users,
    granted_at      TIMESTAMP DEFAULT NOW(),
    expires_at      TIMESTAMP,
    UNIQUE(user_id, menu_id, function_id)
);
```

#### user_data_scope

```sql
CREATE TABLE user_data_scope (
    scope_id        BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users,
    scope_type      VARCHAR(20),             -- ALL / ASSIGNED / OWNED / NONE
    customer_ids    BIGINT[],
    updated_by      BIGINT REFERENCES users,
    updated_at      TIMESTAMP DEFAULT NOW()
);
```

#### field_permissions

```sql
CREATE TABLE field_permissions (
    field_perm_id   BIGSERIAL PRIMARY KEY,
    role_id         BIGINT REFERENCES roles,
    table_name      VARCHAR(100),
    column_name     VARCHAR(100),
    can_view_raw    BOOLEAN DEFAULT FALSE,
    mask_pattern    VARCHAR(100),
    UNIQUE(role_id, table_name, column_name)
);
```

#### permission_change_history

```sql
CREATE TABLE permission_change_history (
    history_id      BIGSERIAL PRIMARY KEY,
    target_type     VARCHAR(20),             -- ROLE / USER
    target_id       BIGINT,
    menu_id         BIGINT,
    function_id     BIGINT,
    before_perm     VARCHAR(10),
    after_perm      VARCHAR(10),
    changed_by      BIGINT REFERENCES users,
    changed_at      TIMESTAMP DEFAULT NOW(),
    reason          VARCHAR(500)
);
```

### 5.4 사용자별 담당 법인 권한

#### user_customer_access ★

```sql
CREATE TABLE user_customer_access (
    uca_id          BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users,
    customer_id     BIGINT REFERENCES customers,
    access_level    VARCHAR(20) NOT NULL,    -- OWNER/CO_WORKER/REVIEWER/ASSISTANT/VIEWER/BLOCKED
    fiscal_year_from INT,
    fiscal_year_to   INT,
    valid_from      DATE,
    valid_to        DATE,
    assigned_by     BIGINT REFERENCES users,
    assigned_at     TIMESTAMP DEFAULT NOW(),
    reason          VARCHAR(500),
    is_primary      BOOLEAN DEFAULT FALSE,
    UNIQUE(user_id, customer_id)
);
CREATE INDEX idx_uca_user ON user_customer_access(user_id);
CREATE INDEX idx_uca_customer ON user_customer_access(customer_id);
```

#### customer_groups / customer_group_members

```sql
CREATE TABLE customer_groups (
    group_id        BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT REFERENCES tenants,
    group_code      VARCHAR(50),
    group_name      VARCHAR(200),
    description     VARCHAR(500),
    UNIQUE(tenant_id, group_code)
);

CREATE TABLE customer_group_members (
    group_id        BIGINT REFERENCES customer_groups,
    customer_id     BIGINT REFERENCES customers,
    PRIMARY KEY (group_id, customer_id)
);
```

#### user_customer_group_access

```sql
CREATE TABLE user_customer_group_access (
    ucga_id         BIGSERIAL PRIMARY KEY,
    user_id         BIGINT REFERENCES users,
    group_id        BIGINT REFERENCES customer_groups,
    access_level    VARCHAR(20),
    valid_from      DATE,
    valid_to        DATE,
    assigned_by     BIGINT REFERENCES users,
    assigned_at     TIMESTAMP DEFAULT NOW(),
    UNIQUE(user_id, group_id)
);
```

#### customer_access_rules (조건 기반 자동 할당)

```sql
CREATE TABLE customer_access_rules (
    rule_id         BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT REFERENCES tenants,
    user_id         BIGINT REFERENCES users,
    rule_type       VARCHAR(30),             -- INDUSTRY/REGION/CORP_TYPE
    rule_value      VARCHAR(100),
    access_level    VARCHAR(20),
    priority        INT DEFAULT 100,
    is_active       BOOLEAN DEFAULT TRUE
);
```

#### access_delegations (위임)

```sql
CREATE TABLE access_delegations (
    delegation_id   BIGSERIAL PRIMARY KEY,
    delegator_id    BIGINT REFERENCES users,
    delegatee_id    BIGINT REFERENCES users,
    customer_id     BIGINT REFERENCES customers,  -- NULL=전체
    access_level    VARCHAR(20),
    valid_from      TIMESTAMP NOT NULL,
    valid_to        TIMESTAMP NOT NULL,
    reason          VARCHAR(500),
    status          VARCHAR(20) DEFAULT 'ACTIVE',
    created_at      TIMESTAMP DEFAULT NOW()
);
```

#### customer_access_history

```sql
CREATE TABLE customer_access_history (
    history_id      BIGSERIAL PRIMARY KEY,
    user_id         BIGINT,
    customer_id     BIGINT,
    before_level    VARCHAR(20),
    after_level     VARCHAR(20),
    source_type     VARCHAR(20),             -- DIRECT/GROUP/RULE/DELEGATION
    changed_by      BIGINT,
    changed_at      TIMESTAMP DEFAULT NOW(),
    reason          VARCHAR(500)
);
```

### 5.5 고객사 / 사업연도

#### customers

```sql
CREATE TABLE customers (
    customer_id     BIGSERIAL PRIMARY KEY,
    customer_code   VARCHAR(20) NOT NULL,
    corp_name       VARCHAR(200) NOT NULL,
    corp_reg_no     VARCHAR(13) NOT NULL,    -- 법인등록번호 (암호화)
    biz_reg_no      VARCHAR(13) NOT NULL,    -- 사업자등록번호 (암호화)
    ceo_name        VARCHAR(100),
    address         VARCHAR(500),
    industry_code   VARCHAR(10),
    corp_type       VARCHAR(20),             -- GENERAL/FOREIGN/CONSOLIDATED
    fiscal_month    INT DEFAULT 12,
    created_at      TIMESTAMP DEFAULT NOW(),
    UNIQUE(customer_code)
);
```

#### business_years

```sql
CREATE TABLE business_years (
    by_id           BIGSERIAL PRIMARY KEY,
    customer_id     BIGINT REFERENCES customers,
    fiscal_year     INT NOT NULL,
    start_date      DATE NOT NULL,
    end_date        DATE NOT NULL,
    months          INT NOT NULL,
    status          VARCHAR(20),             -- DRAFT/IN_REVIEW/FILED
    filed_at        TIMESTAMP,
    UNIQUE(customer_id, fiscal_year)
);
```

### 5.6 세무정보 / 자산

#### financial_statements / fs_lines

```sql
CREATE TABLE financial_statements (
    fs_id           BIGSERIAL PRIMARY KEY,
    by_id           BIGINT REFERENCES business_years,
    fs_type         VARCHAR(20),             -- BS/IS/CF/EQUITY
    import_source   VARCHAR(50),             -- EXCEL/ERP/MANUAL
    imported_at     TIMESTAMP,
    imported_by     BIGINT REFERENCES users
);

CREATE TABLE fs_lines (
    line_id         BIGSERIAL PRIMARY KEY,
    fs_id           BIGINT REFERENCES financial_statements,
    account_code    VARCHAR(20),
    account_name    VARCHAR(200),
    debit           NUMERIC(18,0),
    credit          NUMERIC(18,0),
    balance         NUMERIC(18,0)
);
```

#### account_mappings

```sql
CREATE TABLE account_mappings (
    mapping_id      BIGSERIAL PRIMARY KEY,
    customer_id     BIGINT REFERENCES customers,
    src_account     VARCHAR(50),
    std_account     VARCHAR(50),
    learned         BOOLEAN DEFAULT FALSE,
    UNIQUE(customer_id, src_account)
);
```

#### assets / depreciation

```sql
CREATE TABLE assets (
    asset_id        BIGSERIAL PRIMARY KEY,
    customer_id     BIGINT REFERENCES customers,
    asset_code      VARCHAR(50),
    asset_name      VARCHAR(200),
    asset_category  VARCHAR(50),             -- 건물/기계장치/차량운반구/SW 등
    acquire_date    DATE,
    acquire_amount  NUMERIC(18,0),
    salvage_value   NUMERIC(18,0) DEFAULT 0,
    useful_life     INT,                     -- 신고내용연수
    method          VARCHAR(20),             -- STRAIGHT/DECLINING
    status          VARCHAR(20)
);

CREATE TABLE depreciation (
    dep_id          BIGSERIAL PRIMARY KEY,
    asset_id        BIGINT REFERENCES assets,
    by_id           BIGINT REFERENCES business_years,
    book_dep        NUMERIC(18,0),           -- 회계상 상각비
    tax_dep_limit   NUMERIC(18,0),           -- 세무상 상각범위액
    excess          NUMERIC(18,0),           -- 한도초과(유보)
    shortfall       NUMERIC(18,0),           -- 시인부족
    UNIQUE(asset_id, by_id)
);
```

### 5.7 세무조정 / 유보

#### tax_adjustments

```sql
CREATE TABLE tax_adjustments (
    adj_id          BIGSERIAL PRIMARY KEY,
    by_id           BIGINT REFERENCES business_years,
    adj_category    VARCHAR(50),             -- 익금산입/손금불산입 등
    adj_code        VARCHAR(20),             -- 표준코드 (홈택스 매핑)
    adj_name        VARCHAR(200),
    amount          NUMERIC(18,0),
    reserve_type    VARCHAR(20),             -- 유보/△유보/기타사외유출
    description     TEXT,
    source_form     VARCHAR(20),
    snapshot_id     BIGINT REFERENCES by_law_snapshot,  -- 적용 법령 스냅샷
    created_by      BIGINT REFERENCES users,
    created_at      TIMESTAMP DEFAULT NOW()
);
```

#### reserves (유보 관리)

```sql
CREATE TABLE reserves (
    reserve_id      BIGSERIAL PRIMARY KEY,
    customer_id     BIGINT REFERENCES customers,
    by_id           BIGINT REFERENCES business_years,
    item_code       VARCHAR(20),
    item_name       VARCHAR(200),
    begin_balance   NUMERIC(18,0),
    add_amount      NUMERIC(18,0),
    reduce_amount   NUMERIC(18,0),
    end_balance     NUMERIC(18,0)
);
```

#### carryforward_loss (이월결손금)

```sql
CREATE TABLE carryforward_loss (
    loss_id         BIGSERIAL PRIMARY KEY,
    customer_id     BIGINT REFERENCES customers,
    origin_year     INT,                     -- 결손금 발생 사업연도
    origin_amount   NUMERIC(18,0),
    used_amount     NUMERIC(18,0) DEFAULT 0,
    remain_amount   NUMERIC(18,0),
    expire_year     INT,                     -- 10년 또는 15년 후 만료
    status          VARCHAR(20)              -- ACTIVE/EXPIRED/EXHAUSTED
);
```

### 5.8 서식 (★ 시점별 버전 관리)

> 서식은 매년 법령 개정에 따라 항목 추가·삭제·재배치, 코드값 변경, 계산식 변경이 발생한다. 사업연도별로 그 시점의 정식 서식으로 작성·출력·전자신고되어야 하므로, 서식 식별자(`form_code`)와 버전(`form_version`)을 분리하여 시점별로 관리한다.

#### tax_forms (서식 마스터 — 영구 식별자)

```sql
CREATE TABLE tax_forms (
    form_id         BIGSERIAL PRIMARY KEY,
    form_code       VARCHAR(20) UNIQUE,      -- BJ_03 (별지 제3호) 등 영구 코드
    form_name       VARCHAR(200),
    parent_form_code VARCHAR(20),            -- 부속 서식 관계
    description     TEXT,
    is_active       BOOLEAN DEFAULT TRUE,
    created_at      TIMESTAMP DEFAULT NOW()
);
```

#### form_versions (서식 시점별 버전 — public 스키마 / 5.9에서도 참조)

```sql
-- public 스키마 (전 테넌트 공통 마스터)
CREATE TABLE form_versions (
    form_ver_id      BIGSERIAL PRIMARY KEY,
    form_code        VARCHAR(20) REFERENCES tax_forms(form_code),
    form_version     VARCHAR(20),            -- '2024-01', '2025-02' 등
    revision_type    VARCHAR(20),            -- MAJOR/MINOR/PATCH
    effective_from   DATE NOT NULL,
    effective_to     DATE,
    statute_ref      VARCHAR(200),           -- 근거 법조항
    promulgation_no  VARCHAR(50),
    summary          TEXT,
    status           VARCHAR(20) DEFAULT 'DRAFT',  -- DRAFT/REVIEWED/ACTIVE/RETIRED
    published_at     TIMESTAMP,              -- 동일 시행일 복수 버전 시 최신 결정
    created_by       BIGINT,
    approved_by      BIGINT,
    created_at       TIMESTAMP DEFAULT NOW(),
    UNIQUE(form_code, form_version)
);
CREATE INDEX idx_fv_code_effdate ON form_versions(form_code, effective_from);
```

#### form_templates (서식 메타 + PDF 템플릿 바이너리)

```sql
CREATE TABLE form_templates (
    template_id      BIGSERIAL PRIMARY KEY,
    form_ver_id      BIGINT REFERENCES form_versions,
    template_json    JSONB,                  -- 항목 구조/타입/라벨 (필드 참조는 form_field_references로 정규화)
    pdf_template_path VARCHAR(500),          -- JasperReports .jrxml 경로 (S3)
    pdf_template_hash VARCHAR(64),
    nts_efile_mapping JSONB,                 -- 항목코드 → 자료구분/서식코드/필드순번/byte 길이 매핑
    layout_meta      JSONB,                  -- 용지/방향/여백 등
    UNIQUE(form_ver_id)
);
```

#### form_validations (검증룰, 버전별)

```sql
CREATE TABLE form_validations (
    val_id           BIGSERIAL PRIMARY KEY,
    form_ver_id      BIGINT REFERENCES form_versions,
    rule_code        VARCHAR(50),
    rule_expression  TEXT,                   -- 예: "{04} = {01}+{02}-{03}"
    severity         VARCHAR(20),            -- ERROR/WARN/INFO
    message          TEXT,
    is_active        BOOLEAN DEFAULT TRUE
);
```

#### form_field_references (필드 참조/의존성)

```sql
CREATE TABLE form_field_references (
    ref_id            BIGSERIAL PRIMARY KEY,
    ref_group_code    VARCHAR(50),             -- 하나의 표현식에 속한 참조 묶음
    form_ver_id       BIGINT REFERENCES form_versions, -- 규칙 소유 서식 버전
    target_form_ver_id BIGINT REFERENCES form_versions,
    target_field      VARCHAR(50),             -- 결과가 반영되는 필드
    target_item_no    VARCHAR(30),
    source_form_ver_id BIGINT REFERENCES form_versions,
    source_field      VARCHAR(50),             -- 참조 원천 필드
    source_item_no    VARCHAR(30),
    reference_type    VARCHAR(20),             -- CALC/VALIDATION/COPY/DEFAULT/VISIBLE_IF/ENABLE_IF/LOOKUP
    expression        TEXT,                    -- {01}+{02}-{03}
    condition_expression TEXT,                 -- 조건부 참조식
    calc_order        INT DEFAULT 0,
    on_source_change  VARCHAR(30) DEFAULT 'AUTO_RECALC', -- AUTO_RECALC/WARN/MANUAL_CONFIRM
    null_handling     VARCHAR(20) DEFAULT 'ZERO',        -- ZERO/IGNORE/ERROR
    rounding_rule     VARCHAR(20) DEFAULT 'NONE',        -- NONE/ROUND/TRUNC/FLOOR
    status            VARCHAR(20) DEFAULT 'DRAFT',       -- DRAFT/REVIEWED/ACTIVE/RETIRED
    cycle_checked     BOOLEAN DEFAULT FALSE,
    last_checked_at   TIMESTAMP,
    note              TEXT,
    created_by        BIGINT REFERENCES users,
    created_at        TIMESTAMP DEFAULT NOW(),
    updated_by        BIGINT REFERENCES users,
    updated_at        TIMESTAMP
);
CREATE INDEX idx_ffr_target ON form_field_references(target_form_ver_id, target_field);
CREATE INDEX idx_ffr_source ON form_field_references(source_form_ver_id, source_field);
CREATE INDEX idx_ffr_form ON form_field_references(form_ver_id, status);
CREATE INDEX idx_ffr_group ON form_field_references(ref_group_code);
```

#### form_field_migration (버전 간 항목 매핑)

```sql
CREATE TABLE form_field_migration (
    migration_id     BIGSERIAL PRIMARY KEY,
    from_ver_id      BIGINT REFERENCES form_versions,
    to_ver_id        BIGINT REFERENCES form_versions,
    kind             VARCHAR(20),            -- SAME/RENUMBER/SPLIT/MERGE/NEW/DELETED/CODE_CHANGE/FORMULA
    from_field       VARCHAR(50),            -- 'A.08' 형식
    to_field         VARCHAR(50),            -- 단일 (SPLIT은 to_fields 사용)
    to_fields        VARCHAR(200),           -- 다수 매핑 시 '|' 구분
    split_ratio      JSONB,                  -- {"A.11":0.5,"A.12":0.5}
    default_value    NUMERIC(18,0),          -- NEW 항목 기본값
    manual_review    BOOLEAN DEFAULT FALSE,
    note             TEXT
);
CREATE INDEX idx_ffm_from ON form_field_migration(from_ver_id);
CREATE INDEX idx_ffm_to   ON form_field_migration(to_ver_id);
```

#### form_data (사업연도별 입력 데이터)

```sql
CREATE TABLE form_data (
    data_id         BIGSERIAL PRIMARY KEY,
    by_id           BIGINT REFERENCES business_years,
    form_code       VARCHAR(20),
    form_ver_id     BIGINT REFERENCES form_versions,  -- ★ 실제 적용된 서식 버전
    data_json       JSONB,                  -- 항목코드 → 값
    is_final        BOOLEAN DEFAULT FALSE,
    snapshot_id     BIGINT REFERENCES by_law_snapshot,
    updated_by      BIGINT REFERENCES users,
    updated_at      TIMESTAMP,
    UNIQUE(by_id, form_code)
);
```

#### form_data_migration_history (마이그레이션 실행 이력)

```sql
CREATE TABLE form_data_migration_history (
    history_id       BIGSERIAL PRIMARY KEY,
    migration_id     BIGINT REFERENCES form_field_migration,
    by_id            BIGINT REFERENCES business_years,
    from_ver_id      BIGINT,
    to_ver_id        BIGINT,
    before_data      JSONB,                  -- 변환 전 스냅샷 (롤백용)
    after_data       JSONB,                  -- 변환 후
    delta_amount     NUMERIC(18,0),
    status           VARCHAR(20),            -- DRY_RUN/EXECUTED/ROLLED_BACK
    executed_by      BIGINT REFERENCES users,
    executed_at      TIMESTAMP DEFAULT NOW(),
    note             TEXT
);
CREATE INDEX idx_fdmh_by ON form_data_migration_history(by_id);
```

#### form_relationships (서식 간 자동 연동)

```sql
-- 예: 별지 제3호 ②번 = SUM(별지 제15호 A섹션)
CREATE TABLE form_relationships (
    rel_id             BIGSERIAL PRIMARY KEY,
    rel_code           VARCHAR(50) UNIQUE,        -- 예: BJ03_02_FROM_BJ15_A
    parent_form_ver_id BIGINT REFERENCES form_versions,
    parent_field       VARCHAR(50),               -- 대상 항목코드/항목번호
    parent_item_no     VARCHAR(30),
    child_form_ver_id  BIGINT REFERENCES form_versions,
    child_section      VARCHAR(50),
    child_field        VARCHAR(50),
    child_item_no      VARCHAR(30),
    aggregate_type     VARCHAR(20),               -- SUM/SUBTOTAL/COUNT/COPY/FORMULA
    formula_expression TEXT,                      -- FORMULA 또는 복합 집계식
    filter_expression  TEXT,                      -- 처분구분/코드/법인유형/사업연도 조건
    sign_rule          VARCHAR(20) DEFAULT 'ADD', -- ADD/SUBTRACT/ABS/REVERSE
    null_handling      VARCHAR(20) DEFAULT 'ZERO',-- ZERO/IGNORE/ERROR
    rounding_rule      VARCHAR(20) DEFAULT 'NONE',-- NONE/ROUND/TRUNC/FLOOR
    calc_order         INT DEFAULT 0,
    validation_tolerance NUMERIC(18,0) DEFAULT 0,
    allow_manual_override BOOLEAN DEFAULT FALSE,
    status             VARCHAR(20) DEFAULT 'DRAFT', -- DRAFT/REVIEWED/ACTIVE/RETIRED
    is_required        BOOLEAN DEFAULT FALSE,
    note               TEXT,
    created_by         BIGINT REFERENCES users,
    created_at         TIMESTAMP DEFAULT NOW(),
    updated_by         BIGINT REFERENCES users,
    updated_at         TIMESTAMP
);
CREATE INDEX idx_form_relationships_parent ON form_relationships(parent_form_ver_id, parent_field);
CREATE INDEX idx_form_relationships_child ON form_relationships(child_form_ver_id, child_field);
```

**서식 버전 결정 함수 (사업연도 → 적용 서식 버전):**

```sql
CREATE OR REPLACE FUNCTION get_form_version_for(p_by_id BIGINT, p_form_code VARCHAR)
RETURNS BIGINT AS $$
DECLARE
  v_end_date DATE;
  v_ver_id   BIGINT;
BEGIN
  SELECT end_date INTO v_end_date FROM business_years WHERE by_id = p_by_id;

  SELECT form_ver_id INTO v_ver_id
    FROM form_versions
   WHERE form_code = p_form_code
     AND status = 'ACTIVE'
     AND effective_from <= v_end_date
     AND (effective_to IS NULL OR effective_to >= v_end_date)
   ORDER BY effective_from DESC, published_at DESC NULLS LAST
   LIMIT 1;

  RETURN v_ver_id;
END;
$$ LANGUAGE plpgsql;
```

### 5.9 법령·세율 버전 관리 (public 스키마)

#### tax_law_versions

```sql
CREATE TABLE tax_law_versions (
    version_id       BIGSERIAL PRIMARY KEY,
    law_code         VARCHAR(50),
    law_name         VARCHAR(200),
    statute_ref      VARCHAR(200),
    effective_from   DATE NOT NULL,
    effective_to     DATE,
    retroactive_from DATE,
    transitional_rule TEXT,
    summary          TEXT,
    status           VARCHAR(20) DEFAULT 'DRAFT',  -- DRAFT/REVIEWED/ACTIVE/RETIRED
    created_by       BIGINT REFERENCES users,
    approved_by      BIGINT REFERENCES users,
    created_at       TIMESTAMP DEFAULT NOW(),
    approved_at      TIMESTAMP
);
CREATE INDEX idx_tlv_code_effdate ON tax_law_versions(law_code, effective_from);
```

#### tax_rates / tax_limits / tax_credits_versions

```sql
CREATE TABLE tax_rates (
    rate_id          BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    rate_type        VARCHAR(30),             -- CIT_GENERAL/CIT_SME/MIN_TAX/SURTAX
    bracket_from     NUMERIC(18,0),
    bracket_to       NUMERIC(18,0),
    rate_pct         NUMERIC(7,4),
    deduction        NUMERIC(18,0) DEFAULT 0
);

CREATE TABLE tax_limits (
    limit_id         BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    limit_code       VARCHAR(50),             -- ENT_BASE_SME/DONATION_SPECIAL/VEHICLE_DEPR 등
    condition_key    VARCHAR(50),
    condition_value  VARCHAR(200),
    amount           NUMERIC(18,0),
    rate_pct         NUMERIC(9,6),
    unit             VARCHAR(20),
    formula          TEXT
);

CREATE TABLE tax_credits_versions (
    credit_id        BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    credit_code      VARCHAR(50),             -- RND/INTEG_INVEST/FOREIGN_TAX/SME_SPECIAL
    entity_type      VARCHAR(20),
    industry_filter  VARCHAR(200),
    rate_pct         NUMERIC(7,4),
    addl_rate_pct    NUMERIC(7,4),
    cap_amount       NUMERIC(18,0),
    carryforward_yrs INT,
    min_tax_applicable BOOLEAN
);
```

#### depreciation_lives / sme_criteria / loss_carryforward_rules

```sql
CREATE TABLE depreciation_lives (
    dep_id           BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    asset_category   VARCHAR(100),
    standard_years   INT,
    min_years        INT,
    max_years        INT,
    method_allowed   VARCHAR(50)
);

CREATE TABLE sme_criteria (
    criteria_id      BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    industry_code    VARCHAR(10),
    revenue_limit    NUMERIC(18,0),
    asset_limit      NUMERIC(18,0),
    extra_rule       TEXT
);

CREATE TABLE loss_carryforward_rules (
    rule_id          BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    entity_type      VARCHAR(20),             -- SME/GENERAL
    carry_years      INT,                     -- 10년 / 15년
    deduction_cap_pct NUMERIC(5,2)            -- 60 / 80 / 100
);
```

#### efile_masters

> 홈택스 법인세 전자신고는 XML이 아니라 파일설명서의 레코드별 필드를 byte 길이에 맞춰 이어 붙인 라인 시퀀셜 텍스트 파일이다. 전자신고 파일 생성 시에는 요청한 전자신고ID(`efile_id`)의 생성일 기준 최신 시행일을 찾고, 그 시행일에 속한 하위순번 마스터 전체를 적용한다.

```sql
CREATE TABLE efile_masters (
    efile_master_id BIGSERIAL PRIMARY KEY,
    tax_type        VARCHAR(30) NOT NULL,        -- CIT
    efile_id        VARCHAR(50) NOT NULL,        -- 홈택스 전자신고ID
    effective_date  DATE NOT NULL,               -- 시행일
    efile_sub_seq   INT NOT NULL,                -- 전자신고ID의 하위순번
    efile_name      VARCHAR(200) NOT NULL,       -- 전자신고명
    source_doc_name VARCHAR(300),                -- 홈택스 파일설명서 파일명
    charset_name    VARCHAR(30) DEFAULT 'MS949', -- KSC-5601/CP949 호환 검증
    line_ending     VARCHAR(10) DEFAULT 'CRLF',
    filename_rule   VARCHAR(300),
    change_log      TEXT,
    status          VARCHAR(20) DEFAULT 'DRAFT',
    created_at      TIMESTAMP DEFAULT NOW(),
    UNIQUE(tax_type, efile_id, effective_date, efile_sub_seq)
);
CREATE INDEX idx_efile_masters_effective
    ON efile_masters(tax_type, efile_id, effective_date DESC, efile_sub_seq);
```

#### efile_detail_forms

```sql
CREATE TABLE efile_detail_forms (
    detail_id       BIGSERIAL PRIMARY KEY,
    efile_master_id BIGINT REFERENCES efile_masters,
    detail_seq      INT NOT NULL,                -- 상세 신고목록 순번
    detail_code     VARCHAR(50),                 -- 내부 상세 신고목록 코드
    form_code       VARCHAR(20),                 -- 예: D100300
    form_name       VARCHAR(200),
    output_order    INT NOT NULL,
    is_required     BOOLEAN DEFAULT FALSE,
    include_rule    TEXT,                        -- 사업연도/법인유형/신고유형별 수록 여부
    note            TEXT,
    UNIQUE(efile_master_id, detail_seq)
);
CREATE INDEX idx_efile_detail_forms_master ON efile_detail_forms(efile_master_id, output_order);
```

#### efile_record_layouts

```sql
CREATE TABLE efile_record_layouts (
    record_layout_id BIGSERIAL PRIMARY KEY,
    detail_id        BIGINT REFERENCES efile_detail_forms,
    form_code        VARCHAR(20),                -- 예: D100300
    record_type      VARCHAR(2),                 -- 자료구분: 81/83/84/93/9A 등
    record_name      VARCHAR(200),
    record_order     INT,
    data_length      INT,                        -- 공란 제외 길이
    record_length    INT NOT NULL,               -- CR/LF 제외 전체 byte 길이
    is_multi_record  BOOLEAN DEFAULT FALSE,
    sequence_rule    TEXT,                       -- Multi Record 일련번호 부여 규칙
    include_rule     TEXT,                       -- 사업연도/조건별 수록 여부
    UNIQUE(detail_id, form_code, record_type, record_name)
);
CREATE INDEX idx_efile_record_layouts_detail ON efile_record_layouts(detail_id);
```

#### efile_record_fields

```sql
CREATE TABLE efile_record_fields (
    field_id         BIGSERIAL PRIMARY KEY,
    record_layout_id BIGINT REFERENCES efile_record_layouts,
    field_seq        INT NOT NULL,
    field_name       VARCHAR(200),
    source_item_no   VARCHAR(30),                -- 신고서 항목번호
    data_type        VARCHAR(20),                -- CHAR/NUMBER/RATE/DATE/SPACE/FIX
    byte_length      INT NOT NULL,
    cumulative_len   INT NOT NULL,
    required         BOOLEAN DEFAULT FALSE,
    default_value    VARCHAR(100),
    fixed_value      VARCHAR(100),
    align_type       VARCHAR(10),                -- LEFT/RIGHT
    pad_char         VARCHAR(1),                 -- ' ' / '0'
    validation_code  VARCHAR(50),                -- AMT/DATE/ID/LIST 등
    allowed_values   JSONB,
    note             TEXT,
    UNIQUE(record_layout_id, field_seq)
);
CREATE INDEX idx_efile_record_fields_layout ON efile_record_fields(record_layout_id);
```

#### by_law_snapshot ★ (사업연도별 적용 법령 스냅샷)

```sql
CREATE TABLE by_law_snapshot (
    snapshot_id      BIGSERIAL PRIMARY KEY,
    by_id            BIGINT REFERENCES business_years,
    rate_version_id  BIGINT REFERENCES tax_law_versions,
    limit_version_ids BIGINT[],
    credit_version_ids BIGINT[],
    dep_version_id   BIGINT REFERENCES tax_law_versions,
    loss_rule_id     BIGINT REFERENCES loss_carryforward_rules,
    form_version_set JSONB,                   -- {"별지제3호":"2025-01", ...}
    is_locked        BOOLEAN DEFAULT FALSE,
    created_at       TIMESTAMP DEFAULT NOW(),
    locked_at        TIMESTAMP
);
```

#### law_amendment_history

```sql
CREATE TABLE law_amendment_history (
    amend_id         BIGSERIAL PRIMARY KEY,
    version_id       BIGINT REFERENCES tax_law_versions,
    amendment_date   DATE,
    promulgation_no  VARCHAR(50),
    affected_modules VARCHAR(500),
    impact_summary   TEXT,
    notice_sent_at   TIMESTAMP,
    notified_users   INT
);
```

### 5.10 전자신고

#### efiling_history

```sql
CREATE TABLE efiling_history (
    efiling_id      BIGSERIAL PRIMARY KEY,
    by_id           BIGINT REFERENCES business_years,
    efile_id        VARCHAR(50),              -- 생성 당시 요청 전자신고ID
    effective_date  DATE,                     -- 생성 당시 적용 시행일
    file_name       VARCHAR(300),
    file_path       VARCHAR(500),
    file_hash       VARCHAR(64),             -- SHA-256
    file_size       BIGINT,
    charset_name    VARCHAR(30) DEFAULT 'MS949',
    record_count    INT,
    generated_at    TIMESTAMP DEFAULT NOW(),
    submitted_at    TIMESTAMP,
    nts_receipt_no  VARCHAR(50),
    status          VARCHAR(20),             -- GENERATED/SUBMITTED/ACCEPTED/REJECTED
    error_message   TEXT,
    submitted_by    BIGINT REFERENCES users
);
```

#### efiling_history_masters

```sql
CREATE TABLE efiling_history_masters (
    history_master_id BIGSERIAL PRIMARY KEY,
    efiling_id        BIGINT REFERENCES efiling_history,
    efile_master_id   BIGINT REFERENCES efile_masters,
    efile_sub_seq     INT NOT NULL,
    efile_name        VARCHAR(200) NOT NULL,
    UNIQUE(efiling_id, efile_master_id)
);
CREATE INDEX idx_efiling_history_masters_hist ON efiling_history_masters(efiling_id);
```

#### efiling_files

```sql
CREATE TABLE efiling_files (
    file_id         BIGSERIAL PRIMARY KEY,
    efiling_id      BIGINT REFERENCES efiling_history,
    file_kind       VARCHAR(20),              -- ORIGINAL/ZIP/ATTACHMENT
    file_name       VARCHAR(300),
    file_path       VARCHAR(500),
    file_hash       VARCHAR(64),
    file_size       BIGINT,
    created_at      TIMESTAMP DEFAULT NOW()
);
CREATE INDEX idx_efiling_files_efiling ON efiling_files(efiling_id);
```

#### efiling_validation

```sql
CREATE TABLE efiling_validation (
    val_id          BIGSERIAL PRIMARY KEY,
    efiling_id      BIGINT REFERENCES efiling_history,
    rule_code       VARCHAR(50),
    severity        VARCHAR(20),             -- ERROR/WARN/INFO
    message         TEXT,
    record_type     VARCHAR(2),
    form_code       VARCHAR(20),
    field_seq       INT,
    field_name      VARCHAR(200),
    field_path      VARCHAR(300)
);
```

### 5.11 감사·이력

#### audit_logs

```sql
CREATE TABLE audit_logs (
    log_id          BIGSERIAL PRIMARY KEY,
    tenant_id       BIGINT,
    user_id         BIGINT,
    action          VARCHAR(50),             -- LOGIN/LOGOUT/CREATE/UPDATE/DELETE/EXPORT
    target_type     VARCHAR(50),
    target_id       BIGINT,
    ip_address      VARCHAR(45),
    user_agent      VARCHAR(500),
    request_body    JSONB,
    response_status INT,
    occurred_at     TIMESTAMP DEFAULT NOW()
) PARTITION BY RANGE (occurred_at);

CREATE INDEX idx_audit_logs_occurred ON audit_logs(occurred_at);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
```

---

## 6. 뷰 / 함수 / 트리거

### 6.1 권한 평가 SQL (기능 권한)

```sql
-- 사용자 U가 메뉴 M의 기능 F를 사용할 수 있는지 평가
WITH user_perm AS (
  SELECT permission FROM user_menu_function_override
   WHERE user_id = :uid AND menu_id = :mid AND function_id = :fid
     AND (expires_at IS NULL OR expires_at > NOW())
),
role_perm AS (
  SELECT permission FROM role_menu_function rmf
   JOIN user_role ur ON ur.role_id = rmf.role_id
   WHERE ur.user_id = :uid AND rmf.menu_id = :mid AND rmf.function_id = :fid
)
SELECT CASE
  WHEN EXISTS (SELECT 1 FROM user_perm WHERE permission = 'DENY')  THEN 'DENY'
  WHEN EXISTS (SELECT 1 FROM role_perm WHERE permission = 'DENY')  THEN 'DENY'
  WHEN EXISTS (SELECT 1 FROM user_perm WHERE permission = 'ALLOW') THEN 'ALLOW'
  WHEN EXISTS (SELECT 1 FROM role_perm WHERE permission = 'ALLOW') THEN 'ALLOW'
  ELSE 'DENY'
END AS final_permission;
```

### 6.2 사용자별 접근 가능 법인 통합 뷰

```sql
CREATE OR REPLACE VIEW v_user_effective_customer_access AS
WITH direct AS (
  SELECT user_id, customer_id, access_level, 'DIRECT' AS source, 1 AS priority
    FROM user_customer_access
   WHERE (valid_to IS NULL OR valid_to >= CURRENT_DATE)
),
grouped AS (
  SELECT ucga.user_id, cgm.customer_id, ucga.access_level, 'GROUP' AS source, 2 AS priority
    FROM user_customer_group_access ucga
    JOIN customer_group_members cgm ON cgm.group_id = ucga.group_id
   WHERE (ucga.valid_to IS NULL OR ucga.valid_to >= CURRENT_DATE)
),
ruled AS (
  SELECT car.user_id, c.customer_id, car.access_level, 'RULE' AS source, 3 AS priority
    FROM customer_access_rules car
    JOIN customers c ON (
         (car.rule_type='INDUSTRY' AND c.industry_code = car.rule_value)
      OR (car.rule_type='CORP_TYPE' AND c.corp_type    = car.rule_value)
    )
   WHERE car.is_active = TRUE
),
delegated AS (
  SELECT delegatee_id AS user_id, customer_id, access_level, 'DELEGATION' AS source, 1 AS priority
    FROM access_delegations
   WHERE status = 'ACTIVE'
     AND valid_from <= NOW() AND valid_to >= NOW()
),
unioned AS (
  SELECT * FROM direct
  UNION ALL SELECT * FROM grouped
  UNION ALL SELECT * FROM ruled
  UNION ALL SELECT * FROM delegated
),
blocked AS (
  SELECT user_id, customer_id FROM unioned WHERE access_level = 'BLOCKED'
)
SELECT u.user_id,
       u.customer_id,
       CASE
         WHEN EXISTS (SELECT 1 FROM blocked b
                       WHERE b.user_id=u.user_id AND b.customer_id=u.customer_id)
              THEN 'BLOCKED'
         ELSE (SELECT access_level
                 FROM unioned x
                WHERE x.user_id=u.user_id AND x.customer_id=u.customer_id
                ORDER BY priority,
                  CASE access_level
                    WHEN 'OWNER'     THEN 1
                    WHEN 'CO_WORKER' THEN 2
                    WHEN 'REVIEWER'  THEN 3
                    WHEN 'ASSISTANT' THEN 4
                    WHEN 'VIEWER'    THEN 5
                    ELSE 9 END
                LIMIT 1)
       END AS final_level
  FROM (SELECT DISTINCT user_id, customer_id FROM unioned) u;
```

### 6.3 적용 세율 함수

```sql
CREATE OR REPLACE FUNCTION get_applicable_rates(p_by_id BIGINT)
RETURNS TABLE(bracket_from NUMERIC, bracket_to NUMERIC, rate_pct NUMERIC, deduction NUMERIC) AS $$
DECLARE
  v_ver_id  BIGINT;
BEGIN
  SELECT rate_version_id INTO v_ver_id
    FROM by_law_snapshot
   WHERE by_id = p_by_id;

  RETURN QUERY
    SELECT bracket_from, bracket_to, rate_pct, deduction
      FROM tax_rates WHERE version_id = v_ver_id
      ORDER BY bracket_from;
END;
$$ LANGUAGE plpgsql;
```

### 6.4 적용 전자신고 마스터 선택 함수

```sql
CREATE OR REPLACE FUNCTION get_effective_efile_masters(
    p_tax_type VARCHAR,
    p_efile_id VARCHAR,
    p_generate_date DATE DEFAULT CURRENT_DATE
)
RETURNS TABLE(
    efile_master_id BIGINT,
    efile_id VARCHAR,
    effective_date DATE,
    efile_sub_seq INT,
    efile_name VARCHAR
) AS $$
DECLARE
  v_effective_date DATE;
BEGIN
  SELECT max(m.effective_date) INTO v_effective_date
    FROM efile_masters m
   WHERE m.tax_type = p_tax_type
     AND m.efile_id = p_efile_id
     AND m.status = 'ACTIVE'
     AND m.effective_date <= p_generate_date;

  RETURN QUERY
    SELECT m.efile_master_id, m.efile_id, m.effective_date, m.efile_sub_seq, m.efile_name
      FROM efile_masters m
     WHERE m.tax_type = p_tax_type
       AND m.efile_id = p_efile_id
       AND m.status = 'ACTIVE'
       AND m.effective_date = v_effective_date
     ORDER BY m.efile_sub_seq;
END;
$$ LANGUAGE plpgsql;
```

### 6.5 Hibernate Filter (행 단위 보안)

```java
@Entity
@FilterDef(name = "customerAccess",
           parameters = @ParamDef(name = "userId", type = Long.class))
@Filter(name = "customerAccess",
        condition = "customer_id IN (SELECT customer_id FROM v_user_effective_customer_access " +
                                    "WHERE user_id = :userId AND final_level <> 'BLOCKED')")
public class TaxAdjustment { ... }
```

### 6.6 감사로그 트리거 (예시)

```sql
CREATE OR REPLACE FUNCTION fn_audit_trigger() RETURNS TRIGGER AS $$
BEGIN
  INSERT INTO audit_logs(tenant_id, user_id, action, target_type, target_id,
                         request_body, occurred_at)
  VALUES(current_setting('app.tenant_id')::BIGINT,
         current_setting('app.user_id')::BIGINT,
         TG_OP, TG_TABLE_NAME,
         COALESCE(NEW.id, OLD.id),
         to_jsonb(COALESCE(NEW, OLD)),
         NOW());
  RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- 주요 테이블에 일괄 적용
CREATE TRIGGER trg_audit_tax_adj
AFTER INSERT OR UPDATE OR DELETE ON tax_adjustments
FOR EACH ROW EXECUTE FUNCTION fn_audit_trigger();
```

---

## 7. 인덱스 · 파티셔닝 전략

| 테이블 | 전략 |
|--------|------|
| `audit_logs` | `occurred_at` 월별 RANGE 파티셔닝, 5년 후 콜드 스토리지 이동 |
| `form_data` | 사업연도(`by_id`) 기준 RANGE 파티셔닝 |
| `transactions` | (`by_id`, `customer_id`) 복합 인덱스 + 월별 파티셔닝 |
| `tax_adjustments` | (`by_id`, `adj_category`) 인덱스 |
| `assets` | (`customer_id`, `asset_category`) 인덱스 |
| JSONB 컬럼 (`template_json`, `data_json`) | **GIN 인덱스** |
| `user_customer_access` | (`user_id`), (`customer_id`) 인덱스 |
| `tax_law_versions` | (`law_code`, `effective_from`) 인덱스 |
| `by_law_snapshot` | `by_id` UNIQUE |

---

## 8. 보안 · 암호화

| 항목 | 처리 |
|------|------|
| TDE | PostgreSQL Cluster Level TDE 또는 디스크 암호화 |
| 컬럼 암호화 | `pgcrypto` AES-256-GCM, 마스터키 KMS 분리 |
| 암호화 대상 | 주민등록번호, 사업자등록번호, 법인등록번호, 계좌번호, 이메일, 전화번호, TOTP 시크릿 |
| 비밀번호 | bcrypt cost factor 12 |
| Row-Level Security | View + Hibernate Filter (코드 레벨) |
| 마스킹 | `field_permissions`에 정의된 정책에 따라 응답 시 자동 마스킹 |
| 감사로그 무결성 | 일별 해시 체인 (이전일 마지막 행 해시를 다음일 첫 행에 포함) |
| 백업 암호화 | pg_basebackup + GPG, 백업 키 별도 보관 |

---

## 9. 백업 / 운영

| 정책 | 내용 |
|------|------|
| 전체 백업 | 매일 02:00 (Cold) |
| WAL 아카이빙 | 5분 단위 |
| RPO | 5분 |
| RTO | 1시간 |
| 보존 기간 | 일별 30일 / 주별 12주 / 월별 24개월 / 연도별 5년 |
| 복구 검증 | 분기 1회 복구 리허설 |
| Standby | 다른 AZ에 Streaming Replica (Hot Standby) |
| 로그 보관 | `audit_logs` 5년 (이후 콜드 스토리지) |
| 데이터 파기 | 계약 종료 6개월 후, NIST SP 800-88 절차 |

---

## 10. 부록 — 명명 규칙 / 데이터 타입

### 10.1 명명 규칙

| 항목 | 규칙 | 예 |
|------|------|----|
| 테이블 | snake_case, 단수 또는 복수 (도메인 명사) | `users`, `tax_adjustments` |
| 컬럼 | snake_case | `created_at`, `customer_id` |
| PK | `{entity}_id` | `user_id` |
| FK | 참조 PK와 동일 명 | `customer_id` (FK to `customers.customer_id`) |
| 타임스탬프 | `_at` 접미사 | `created_at`, `submitted_at` |
| 불리언 | `is_*` / `has_*` / `can_*` | `is_active`, `can_view_raw` |
| 인덱스 | `idx_{table}_{column}` | `idx_audit_logs_user` |
| 뷰 | `v_*` | `v_user_effective_customer_access` |
| 함수 | `fn_*` / 동사형 | `fn_audit_trigger`, `get_applicable_rates` |

### 10.2 표준 데이터 타입

| 도메인 | 타입 |
|--------|------|
| PK | `BIGSERIAL` |
| 금액 | `NUMERIC(18,0)` (원 단위, 정수) |
| 비율 | `NUMERIC(9,6)` 또는 `NUMERIC(7,4)` (%) |
| 일자 | `DATE` |
| 시각 | `TIMESTAMP` (UTC 저장, 표시 시 Asia/Seoul 변환) |
| 코드 | `VARCHAR(20~50)` |
| 명칭 | `VARCHAR(100~200)` |
| 설명/메모 | `TEXT` |
| 구조화 데이터 | `JSONB` (가변 스키마 서식·룰) |
| 다중 ID | `BIGINT[]` (스냅샷의 다중 버전) |
| 통화 | KRW 단일 통화 가정 (외화법인은 별도 컬럼 추가 시 ISO 4217 코드) |

### 10.3 표준 상태 값

| 컬럼 | 가능 값 |
|------|---------|
| `status` (사업연도) | DRAFT / IN_REVIEW / FILED / AMENDED |
| `status` (법령) | DRAFT / REVIEWED / ACTIVE / RETIRED |
| `status` (전자신고) | GENERATED / SUBMITTED / ACCEPTED / REJECTED |
| `status` (사용자) | ACTIVE / LOCKED / WITHDRAWN |
| `permission` | ALLOW / DENY |
| `access_level` | OWNER / CO_WORKER / REVIEWER / ASSISTANT / VIEWER / BLOCKED |

---

## 문서 변경 이력

| 버전 | 일자 | 작성자 | 변경내용 |
|------|------|--------|----------|
| 1.0 | 2026-05-14 | 강수 | 메인 설계서에서 DB 설계 챕터 분리 신설 |

---

**문의**: Kang-Soo.Cho@kr.ey.com
**연관 문서**: `법인세_세무조정계산서_시스템_설계서.md`
