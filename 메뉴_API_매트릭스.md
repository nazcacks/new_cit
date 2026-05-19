# 메뉴 API 매트릭스

작성일: 2026-05-18

## 범위

`2차구현계획.md`의 Phase 0 ~ Phase 12 기준으로 구현된 prototype IA 메뉴와 실제 백엔드 API 연결 목록이다.

| 메뉴 코드 | 화면 | 주요 API | 컨텍스트 |
|---|---|---|---|
| `dashboard` | 대시보드 | `GET /api/tenants/:tc/dashboard`, `GET /notifications`, `GET /workflow/queue`, `GET /audit-logs` | 불필요 |
| `ws-start` | 작업 시작 | `GET /customers`, `GET/POST /business-years`, `GET /business-years/:by_id/snapshot` | 설정 |
| `ws-info` | 세무정보 입력 | `GET /tax-data/financial-statements`, `GET /tax-data/assets`, `GET /tax-data/transactions`, `GET/POST /tax-data/validation`, `POST /tax-data/:data_type/import`, `GET /vehicle-usage-logs` | 필요 |
| `ws-adj` | 세무조정 | `GET/POST /adjustments`, `GET/POST /adjustments/income`, `GET/POST /adjustments/assets/:module_code`, `GET/POST /adjustments/transactions/:module_code`, `GET/POST /adjustments/evaluation/:module_code`, `GET/POST /adjustments/tax/:module_code`, `GET/POST /adjustments/special/:module_code`, `GET /adjustments/history`, `GET/POST /adjustments/items/:adjustment_item_id/attachments`, `GET /reserves` | 필요 |
| `ws-form` | 서식 작성 | `GET/POST /forms/:form_code`, `GET /forms/:form_code/preview`, `GET /forms/:form_code/pdf`, `GET /forms/attachments`, `GET /forms/pdf-bundle/download` | 필요 |
| `ws-val` | 검증 | `GET /validation/rules`, `POST /validation/run`, `POST /validation/issues/:issue_id/dismiss`, `GET /efilings/precheck` | 필요 |
| `ws-appr` | 결재 | `GET /workflow/queue`, `GET /business-years/:by_id/workflow`, `POST /business-years/:by_id/workflow/events`, `POST /business-years/:by_id/status` | 필요 |
| `ws-print` | 출력 | `GET /forms/attachments`, `GET /forms/:form_code/pdf`, `GET /forms/pdf-bundle/download`, `GET /forms/print-history` | 필요 |
| `ws-file` | 전자신고 | `GET /efilings/format-spec`, `GET /efilings/precheck`, `GET/POST /efilings`, `GET /efilings/:efiling_id/file` | 필요 |
| `post-hist` | 신고 이력 | `GET /business-years`, `GET /business-years/:by_id/efilings` | 선택 |
| `post-amend` | 수정신고/경정청구 | `GET /business-years/:by_id/amendment-preview`, `POST /business-years/:by_id/unlock` | 필요 |
| `rp-alerts` | 알림 센터 | `GET /notifications`, `PATCH /notifications/:notification_id` | 불필요 |
| `rp-compare` | 사업연도 비교 | `GET /reports/year-comparison` | 불필요 |
| `rp-burden` | 세부담 분석 | `GET /reports/tax-burden`, `GET /reports/industry-statistics` | 불필요 |
| `rp-reserve` | 유보 잔액 추이 | `GET /reports/reserve-trend`, `GET /reports/loss-expiry`, `GET/POST /reports/user-defined`, `GET /reports/user-defined/:report_id/run` | 불필요 |
| `ad-tenant` | 테넌트 관리 | `GET/POST /api/tenants` | 불필요 |
| `ad-cust` | 고객사 관리 | `GET/POST /api/tenants/:tc/customers` | 불필요 |
| `ad-user-list` | 사용자 관리 | `GET/POST /api/admin/tenants/:tc/users`, `PUT /users/:login_id`, `POST /users/:login_id/status` | 불필요 |
| `ad-role` | 역할/권한 | `GET /api/admin/roles`, `GET /api/admin/role-permissions`, `PUT /api/admin/roles/:role_code/permissions`, `GET /api/admin/function-codes`, `GET /api/admin/role-menu-functions`, `PUT /api/admin/roles/:role_code/menu-functions` | 불필요 |
| `ad-menu-fn` | 메뉴/기능 관리 | `GET /api/admin/menus`, `PUT /api/admin/menus/:menu_key`, `GET /api/admin/menu-functions`, `PUT /api/admin/menus/:menu_key/functions` | 불필요 |
| `ad-cacc` | 담당 법인 권한 | `GET /api/admin/tenants/:tc/users`, `GET /api/tenants/:tc/customers`, `GET/POST /api/tenants/:tc/access-delegations` | 불필요 |
| `ad-law` | 법령/세율 버전 | `GET/POST /api/tax-laws`, `GET/POST /api/tax-rates`, `GET/POST /api/tax-limits`, `GET /api/law-versioning/summary` | 불필요 |
| `ad-form` | 서식 버전 | `GET/POST /api/form-versioning/forms`, `GET/POST /api/form-versioning/versions`, `GET/POST /api/form-versioning/relationships`, `GET /api/form-versioning/cycle-check`, `POST /api/form-versioning/migrations/{dry-run,execute,rollback}` | 불필요 |
| `ad-audit` | 감사/로그 | `GET /api/tenants/:tc/audit-logs`, `GET /api/tenants/:tc/audit-logs/verify` | 불필요 |

## 추가 백엔드 계약

- `/api/modules/tree`: prototype 5대 영역 메뉴 반환
- `/api/modules/legacy-tree`: 1차 모듈 트리 감사용 반환
- `menu_nodes`: 메뉴/기능 관리용 RBAC 메타데이터 테이블
- `validation_rules`: 50개 이상 통합 검증 규칙 카탈로그
- `validation_issues`: 테넌트별 검증 실행 결과와 dismiss 상태 저장
- P3 보류: e-Tax 직접 제출, ERP 커넥터, Redis pub/sub 캐시 무효화는 외부 자격/인프라 확정 후 별도 매트릭스로 확장한다.
