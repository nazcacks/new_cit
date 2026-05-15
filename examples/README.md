# Examples

Use these examples after starting the stack:

```powershell
docker compose up --build -d postgres api
```

- `flow.ps1`: end-to-end API flow.
- `adjustment_request.json`: sample tax adjustment request body.
- `dead_letter_job.json`: payload that intentionally moves to the DLQ.

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
## 테넌트별 고객사 대상 업무 구분 보완 (2026-05-15)

- 고객사마다 수행 가능한 대상 업무 범위를 별도로 둔다.
- 사용자에게 부여하는 고객사별 업무 권한은 해당 고객사의 대상 업무 범위 안에서만 선택한다.
- 권한 검증은 `테넌트 -> 고객사 -> 고객사 대상 업무 -> 사용자 업무 권한 -> 역할 권한` 순서로 적용한다.

## Phase 7 세무정보 입력 예시 (2026-05-15)

- `GET /api/tenants/{tenant_code}/tax-data/templates/financial-statements`로 표준 템플릿을 내려받는다.
- `POST /api/tenants/{tenant_code}/business-years/{by_id}/tax-data/financial-statements/import`에 multipart `file` 필드로 CSV/XLSX를 업로드한다.
- 자산대장은 `/tax-data/assets/import`, 거래 명세는 `/tax-data/transactions/import`를 사용한다.
- `/tax-data/validation`으로 차변/대변 일치, 미매핑, 자산/거래 건수, 임포트 오류 수를 확인한다.

## Phase 8 B-1 소득금액조정 예시 (2026-05-15)

- `POST /api/tenants/{tenant_code}/business-years/{by_id}/adjustments/income`에 B-1 조정 항목을 전달한다.
- `accounting_income`을 생략하면 재무제표의 `NET_INCOME` 표준계정에서 자동 산출한다.
- `temporary=true` 또는 `disposition=RESERVE`인 항목은 `/reserves` 조회 결과에 자동 유보로 나타난다.

## Phase 9 자산 기반 세무조정 예시 (2026-05-15)

- `POST /adjustments/assets/B4`로 자산대장 기반 감가상각 조정을 계산한다.
- `POST /vehicle-usage-logs`로 업무용승용차 월별 운행기록을 등록한다.
- `POST /adjustments/assets/B5`, `B6`, `B10`으로 퇴직급여충당금, 대손충당금, 업무용승용차 조정을 계산한다.

## Phase 10 거래 기반 세무조정 예시 (2026-05-15)

- `POST /adjustments/transactions/B2`로 거래 명세의 기부금을 특례/일반 한도와 10년 이월 기준으로 계산한다.
- `POST /adjustments/transactions/B3`에 수입금액 명세를 전달해 접대비 한도와 무증빙 손금불산입액을 계산한다.
- `POST /adjustments/transactions/B9`에 가지급금 적수/평균잔액과 이자율을 전달해 지급이자 손금불산입액을 계산한다.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/transactions/B2 `
  -ContentType "application/json" `
  -Body '{"taxable_income_before_donation":500000000}'
```

## Phase 11 평가·이월·유보 예시 (2026-05-15)

- `POST /adjustments/evaluation/B7`과 `B8`로 평가 포지션별 조정과 유보를 계산한다.
- `POST /adjustments/evaluation/B11`로 이월결손금 공제와 만료 알림을 확인한다.
- `POST /adjustments/evaluation/B15`로 자본 변동을 저장하고 전체 유보를 집계한다.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/evaluation/B11 `
  -ContentType "application/json" `
  -Body '{"taxable_income_before_loss":300000000,"loss_carryforwards":[{"origin_year":2025,"original_amount":400000000,"remaining_amount":400000000,"expires_year":2026}]}'
```

## Phase 12 세액 예시 (2026-05-15)

- `POST /adjustments/tax/B12`로 세액공제·감면을 계산한다.
- `POST /adjustments/tax/B13`으로 최저한세 추가세액을 계산한다.
- `POST /adjustments/tax/B14`로 감면율 적용 가산세를 계산한다.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/tax/B12 `
  -ContentType "application/json" `
  -Body '{"tax_base":500000000,"calculated_tax":70000000,"credits":[{"credit_type":"RND","base_amount":100000000,"rate_bps":2500}]}'
```

## 사용자-고객사 업무 권한 예시 (2026-05-15)

- 고객사 생성 시 `work_scopes`로 그 고객사의 대상 업무를 지정한다.
- 사용자 생성/수정 시 `customer_access[].work_scopes`는 해당 고객사의 `work_scopes` 안에서만 지정한다.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/customers `
  -ContentType "application/json" `
  -Body '{"customer_code":"CUST_SCOPE","customer_name":"업무범위 고객사","biz_reg_no":"2208112345","is_sme":true,"work_scopes":["INFO","VALIDATE","APPROVE","POST"]}'

Invoke-RestMethod -Method Post http://localhost:8080/api/admin/tenants/demo/users `
  -ContentType "application/json" `
  -Body '{"login_id":"review01","password":"ChangeMe123!","user_name":"검토 사용자","roles":["TAX_REVIEWER"],"customer_access":[{"customer_id":1,"access_level":"REVIEWER","is_primary":true,"work_scopes":["VALIDATE","APPROVE"]}]}'
```

## Phase 13 외국법인/연결 예시 (2026-05-15)

- `POST /adjustments/special/B16`으로 외국법인 국내원천 소득을 계산한다.
- `POST /adjustments/special/B17`로 연결 대상 법인과 내부거래 제거를 반영한다.

```powershell
Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/special/B16 `
  -ContentType "application/json" `
  -Body '{"foreign_incomes":[{"income_type":"INTEREST","gross_amount":100000000,"attributable_expense":20000000,"pe_allocation_bps":10000,"withholding_tax":5000000}]}'

Invoke-RestMethod -Method Post http://localhost:8080/api/tenants/demo/business-years/1/adjustments/special/B17 `
  -ContentType "application/json" `
  -Body '{"consolidated_entities":[{"entity_code":"PARENT","entity_name":"Parent Co","ownership_bps":10000,"taxable_income":100000000},{"entity_code":"SUBA","entity_name":"Sub A","ownership_bps":10000,"taxable_income":200000000},{"entity_code":"SUBB","entity_name":"Sub B","ownership_bps":10000,"taxable_income":300000000}],"eliminations":[{"elimination_type":"INTERCOMPANY_PROFIT","amount":50000000,"direction":"DEDUCT","description":"내부거래 이익 제거"}]}'
```
