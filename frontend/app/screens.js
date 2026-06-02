import { request, downloadBinary, escapeHtml, money, statusClass, today, asArray } from "/app/api.js";
import { bindDataGridActions, renderDataGrid } from "/app/components/grid.js";
import { hasWorkContext, progressForStatus } from "/app/context.js";
import { fieldLabel, leafFocusText, localizeRouteMeta, routeKeyToLabelKey, statusLabel, t } from "/app/i18n.js";

const DASHBOARD_REFRESH_INTERVAL_MS = 30_000;
let dashboardRealtime = null;
let dashboardCacheVersion = 0;

function uiText(locale, ko, en) {
  return locale === "en" ? en : ko;
}

const KO_RENDERED_TEXT = Object.freeze({
  "Mapping rules": "매핑 규칙",
  "Adjustment History": "조정 이력",
  "Evidence Attachments": "증빙 첨부",
  "Print History": "출력 이력",
  "FORM3 preview": "별지 3호 미리보기",
  "FORM3 validation": "별지 3호 검증",
  "Form catalog": "서식 목록",
  "Preview and source": "미리보기 및 원천",
  "Manual overrides": "수동 수정",
  "Linkage differences": "연동 차이",
  "Selected form validation": "선택 서식 검증",
  "Run result": "실행 결과",
  "Gate summary": "게이트 요약",
  "Issue triage": "이슈 처리",
  "Rule catalog": "규칙 목록",
  "Request review": "검토 요청",
  "Workflow timeline": "워크플로 타임라인",
  "My queue": "내 대기함",
  "Decision": "결재 처리",
  "Return reasons": "반려 사유",
  "Next action": "다음 작업",
  "Form selector": "서식 선택",
  "Preview snapshot": "미리보기 스냅샷",
  "Bundle targets": "묶음 출력 대상",
  "Output readiness": "출력 준비 상태",
  "Print history": "출력 이력",
  "Industry Statistics": "업종별 통계",
  "Loss Expiry": "결손금 만료",
  "User Reports": "사용자 리포트",
  "Customer master": "고객사 마스터",
  "Register customer": "고객사 등록",
  "Maintain corporate taxpayer master data, registration numbers, SME flags, and enabled work scopes.": "법인 납세자 마스터, 등록번호, 중소기업 여부, 사용 업무범위를 관리합니다.",
  "Customers": "고객사",
  "Active": "활성",
  "SME": "중소기업",
  "Work scopes": "업무범위",
  "Tenant customer records and filing work scope coverage.": "테넌트 고객사 기록과 신고 업무범위 적용 현황입니다.",
  "Code": "코드",
  "Customer": "고객사",
  "Biz no.": "사업자번호",
  "Industry": "업종",
  "Status": "상태",
  "No customers registered.": "등록된 고객사가 없습니다.",
  "Create a customer master record with default corporate tax work scopes.": "법인세 신고 기본 업무범위로 고객사 마스터를 생성합니다.",
  "Name": "이름",
  "Business number": "사업자번호",
  "Industry code": "업종 코드",
  "Register customer": "고객사 등록",
  "Business-year registry": "사업연도 등록부",
  "Create business year": "사업연도 생성",
  "Create and review customer business-year workspaces, filing periods, statuses, and lock modes.": "고객사 사업연도 작업장, 신고 기간, 상태, 잠금 모드를 생성하고 검토합니다.",
  "Filed": "신고완료",
  "Locked": "잠김",
  "Filing workspace periods and current lifecycle state.": "신고 작업장 기간과 현재 수명주기 상태입니다.",
  "Year": "연도",
  "Period": "기간",
  "Lock": "잠금",
  "Updated": "수정일시",
  "No business years registered.": "등록된 사업연도가 없습니다.",
  "Open a new filing workspace for the selected customer.": "선택한 고객사의 새 신고 작업장을 엽니다.",
  "Start date": "시작일",
  "End date": "종료일",
  "Create business year": "사업연도 생성",
  "Contract registry": "계약 등록부",
  "Save tax agent contract": "세무대리 계약 저장",
  "Tax agent contracts": "세무대리 계약",
  "Track delegated tax agent contracts, customer assignments, contract periods, and active status.": "위임 세무대리 계약, 고객사 배정, 계약 기간, 활성 상태를 추적합니다.",
  "Agents": "세무대리인",
  "Delegations": "위임",
  "External or delegated tax agents by tenant and customer assignment.": "테넌트와 고객사 배정별 외부 또는 위임 세무대리인입니다.",
  "Agent": "세무대리인",
  "Tenant": "테넌트",
  "Contract period": "계약 기간",
  "Notes": "메모",
  "No tax agent contracts.": "등록된 세무대리 계약이 없습니다.",
  "Register or update delegated tax agent metadata for the tenant.": "테넌트의 위임 세무대리 정보를 등록하거나 수정합니다.",
  "Agent name": "세무대리인명",
  "Contract start": "계약 시작일",
  "Contract end": "계약 종료일",
  "Save contract": "계약 저장",
  "User registry": "사용자 등록부",
  "User registration": "사용자 등록",
  "Security and permission controls": "보안 및 권한 관리",
  "User management": "사용자 관리",
  "Register tenant users, assign tax roles, unlock accounts, and review 2FA status.": "테넌트 사용자를 등록하고 세무 역할을 배정하며 계정 잠금과 2FA 상태를 확인합니다.",
  "Users": "사용자",
  "Tenant login, status, assigned roles, and account recovery actions.": "테넌트 로그인, 상태, 배정 역할, 계정 복구 작업입니다.",
  "Roles": "역할",
  "No users found.": "사용자를 찾을 수 없습니다.",
  "Creates a TAX_EXPERT user and grants the first customer work scope.": "TAX_EXPERT 사용자를 만들고 첫 고객사 업무범위를 부여합니다.",
  "Password": "비밀번호",
  "Use 2FA": "2FA 사용",
  "TOTP Secret": "TOTP 비밀값",
  "Base32 or raw secret": "Base32 또는 원본 비밀값",
  "Register": "등록",
  "Unlock": "잠금 해제",
  "Role catalog": "역할 목록",
  "Function catalog": "기능 목록",
  "Module permission matrix": "모듈 권한 매트릭스",
  "Menu function grants": "메뉴 기능 권한",
  "Masking policies": "마스킹 정책",
  "Policy edit": "정책 편집",
  "Data scopes": "데이터 범위",
  "Scope edit": "범위 편집",
  "Menu registry": "메뉴 등록부",
  "Menu action coverage": "메뉴 액션 범위",
  "Menu function assignment": "메뉴 기능 배정",
  "Role to menu function matrix": "역할별 메뉴 기능 매트릭스",
  "Individual customer assignments": "사용자별 고객사 배정",
  "Customer work scope master": "고객사 업무범위 마스터",
  "Effective access preview": "유효 접근권한 미리보기",
  "Customer groups": "고객사 그룹",
  "Create group": "그룹 생성",
  "Access groups": "접근 그룹",
  "Manage customer groups used for bulk assignment and reusable access policies.": "일괄 배정과 재사용 접근 정책에 쓰는 고객사 그룹을 관리합니다.",
  "Groups": "그룹",
  "Grouped members": "그룹 구성원",
  "Reusable customer sets such as industry teams or VIP portfolios.": "업종 팀이나 VIP 포트폴리오 같은 재사용 고객사 묶음입니다.",
  "Members": "구성원",
  "Default access": "기본 접근권한",
  "No customer groups.": "등록된 고객사 그룹이 없습니다.",
  "Registers a reusable customer access group.": "재사용 가능한 고객사 접근 그룹을 등록합니다.",
  "Group name": "그룹명",
  "Seed customer": "초기 고객사",
  "Save group": "그룹 저장",
  "Automatic assignment rules": "자동 배정 규칙",
  "Create rule": "규칙 생성",
  "Access rules": "접근 규칙",
  "Define automatic assignment rules by customer attributes such as industry, region, and entity type.": "업종, 지역, 법인 유형 같은 고객사 속성으로 자동 배정 규칙을 정의합니다.",
  "Assignments": "배정",
  "Condition-based defaults before manual overrides are applied.": "수동 예외가 적용되기 전의 조건 기반 기본값입니다.",
  "Condition": "조건",
  "Access": "접근권한",
  "Priority": "우선순위",
  "No customer rules.": "등록된 고객사 규칙이 없습니다.",
  "Adds a condition-based default assignment rule.": "조건 기반 기본 배정 규칙을 추가합니다.",
  "Save rule": "규칙 저장",
  "Live tenant delegations": "유효 위임 현황",
  "Create delegation": "위임 생성",
  "Delegation": "위임",
  "Create temporary handoff access for vacation coverage or reviewer substitution.": "휴가 대체나 검토자 대체를 위한 임시 인수인계 접근권한을 생성합니다.",
  "Tenant delegations": "테넌트 위임",
  "Admin delegations": "관리자 위임",
  "Effective customer-level handoff windows.": "고객사 단위 인수인계 유효 기간입니다.",
  "Grantor": "위임자",
  "Delegatee": "수임자",
  "Scope": "범위",
  "No live delegations.": "유효한 위임이 없습니다.",
  "No admin delegation rules.": "관리자 위임 규칙이 없습니다.",
  "Delegates one customer work scope until the selected end date.": "선택한 종료일까지 고객사 업무범위 하나를 위임합니다.",
  "Valid to": "유효 종료일",
  "Reason": "사유",
  "Individual access overrides": "개별 접근 예외",
  "Create override": "예외 생성",
  "Access override": "접근 예외",
  "Manage per-customer exceptions such as conflict blocking or elevated owner access.": "이해상충 차단이나 소유자 권한 상향 같은 고객사별 예외를 관리합니다.",
  "Blocked overrides": "차단 예외",
  "Manual exceptions are applied after groups, rules, and delegations.": "수동 예외는 그룹, 규칙, 위임 이후 적용됩니다.",
  "No access overrides.": "등록된 접근 예외가 없습니다.",
  "Records a customer-level exception for the access evaluator.": "접근 평가기에 적용할 고객사 단위 예외를 기록합니다.",
  "Save override": "예외 저장",
  "Law versions": "법령 버전",
  "Create amendment draft": "개정 초안 생성",
  "Law and rate version control": "법령 및 세율 버전 관리",
  "Law version master": "법령 버전 마스터",
  "Register, review, approve, and retire temporal corporate tax law versions.": "기간별 법인세 법령 버전을 등록, 검토, 승인, 폐기합니다.",
  "Laws": "법령",
  "Active version": "활성 버전",
  "Rates": "세율",
  "Limits": "한도",
  "Temporal versions used by calculation snapshots.": "계산 스냅샷에 사용하는 기간별 버전입니다.",
  "Law": "법령",
  "Law name": "법령명",
  "No law versions.": "등록된 법령 버전이 없습니다.",
  "Creates a DRAFT law version for later data entry and impact simulation.": "추후 데이터 입력과 영향 시뮬레이션을 위한 DRAFT 법령 버전을 만듭니다.",
  "Version code": "버전 코드",
  "Rate brackets": "세율 구간",
  "Add rate bracket": "세율 구간 추가",
  "Corporate tax rates": "법인세율",
  "Maintain taxable income brackets, statutory rates, and progressive deductions by effective date.": "적용일자별 과세표준 구간, 법정세율, 누진공제를 관리합니다.",
  "Rate rows": "세율 행",
  "All rates": "전체 세율",
  "Corporate tax, minimum tax, special rural tax, and penalty rate rows.": "법인세, 최저한세, 농어촌특별세, 가산세 세율 행입니다.",
  "Taxable from": "과세표준 시작",
  "Taxable to": "과세표준 종료",
  "Rate": "세율",
  "Deduction": "공제",
  "No tax rates for the selected version.": "선택한 버전에 등록된 세율이 없습니다.",
  "Creates a temporal rate row for the selected law version.": "선택한 법령 버전에 기간별 세율 행을 생성합니다.",
  "Law version": "법령 버전",
  "Item code": "항목 코드",
  "Rate bps": "세율 bps",
  "Progressive deduction": "누진공제",
  "Save rate": "세율 저장",
  "Add parameter": "파라미터 추가",
  "Business years": "사업연도",
  "Selected snapshot": "선택 스냅샷",
  "Simulation target": "시뮬레이션 대상",
  "Tenant impact": "테넌트 영향",
  "Law amendment history": "법령 개정 이력",
  "Record amendment note": "개정 메모 기록",
  "Amendment history": "개정 이력",
  "Review approval notes, legal change summaries, and amendment audit trail entries.": "승인 메모, 법령 변경 요약, 개정 감사 추적 항목을 확인합니다.",
  "History rows": "이력 행",
  "Latest version": "최신 버전",
  "Approvers": "승인자",
  "Tracked legal changes and approval notes.": "추적된 법령 변경과 승인 메모입니다.",
  "Summary": "요약",
  "Approved by": "승인자",
  "Approved at": "승인일시",
  "No law amendment history.": "등록된 법령 개정 이력이 없습니다.",
  "Adds a history row to the selected law version.": "선택한 법령 버전에 이력 행을 추가합니다.",
  "Save history": "이력 저장",
  "Master forms": "서식 마스터",
  "Register form": "서식 등록",
  "Form master": "서식 마스터",
  "Register master tax forms and keep their active status separate from version metadata.": "세무 서식 마스터를 등록하고 활성 상태를 버전 메타데이터와 분리해 관리합니다.",
  "Forms": "서식",
  "Versions": "버전",
  "Current version": "현재 버전",
  "Reusable form codes used by business-year snapshots and e-filing mappings.": "사업연도 스냅샷과 전자신고 매핑에서 사용하는 재사용 서식 코드입니다.",
  "No forms.": "등록된 서식이 없습니다.",
  "Creates or updates a master form without creating a version row.": "버전 행을 만들지 않고 마스터 서식을 생성하거나 수정합니다.",
  "Form code": "서식 코드",
  "Form name": "서식명",
  "Group": "그룹",
  "Description": "설명",
  "Save form": "서식 저장",
  "Version registry": "버전 등록부",
  "Create version": "버전 생성",
  "Form versions": "서식 버전",
  "Create temporal form versions, review effective windows, and manage status changes.": "기간별 서식 버전을 만들고 적용 기간과 상태 변경을 관리합니다.",
  "Latest": "최신",
  "Effective form templates used by snapshots and migration checks.": "스냅샷과 마이그레이션 점검에서 사용하는 적용 서식 템플릿입니다.",
  "ID": "ID",
  "Form": "서식",
  "Version": "버전",
  "Effective": "적용 기간",
  "No versions.": "등록된 버전이 없습니다.",
  "Creates a draft template version and keeps the master form synchronized.": "초안 템플릿 버전을 만들고 마스터 서식과 동기화합니다.",
  "Version no": "버전 번호",
  "Effective from": "적용 시작일",
  "Effective to": "적용 종료일",
  "Version fields": "버전 항목",
  "Add field": "항목 추가",
  "Field definitions": "필드 정의",
  "Inspect and update template fields for the selected form version.": "선택한 서식 버전의 템플릿 필드를 확인하고 수정합니다.",
  "Fields": "필드",
  "Selected": "선택됨",
  "Field": "필드",
  "Label": "라벨",
  "No fields for the selected version.": "선택한 버전에 등록된 필드가 없습니다.",
  "Updates the selected version field list through the versioning API.": "버전 관리 API로 선택한 버전의 필드 목록을 수정합니다.",
  "Field path": "필드 경로",
  "Save fields": "필드 저장",
  "Version validation rules": "버전 검증 규칙",
  "Add validation": "검증 추가",
  "Validation rules": "검증 규칙",
  "Maintain field-level validation rules attached to each form version.": "각 서식 버전에 연결된 필드 단위 검증 규칙을 관리합니다.",
  "Rules": "규칙",
  "Errors": "오류",
  "Rule": "규칙",
  "Severity": "심각도",
  "Message": "메시지",
  "No validations for the selected version.": "선택한 버전에 등록된 검증 규칙이 없습니다.",
  "Saves rule metadata used by form validation and filing readiness checks.": "서식 검증과 신고 준비 점검에 사용하는 규칙 메타데이터를 저장합니다.",
  "Rule code": "규칙 코드",
  "Save validations": "검증 규칙 저장",
  "Relationship graph": "연동 관계도",
  "Add linkage": "연동 추가",
  "Linkage rules": "연동 규칙",
  "Maintain cross-form field dependencies and check the relationship graph for cycles.": "서식 간 필드 의존성을 관리하고 관계 그래프 순환 여부를 점검합니다.",
  "Relationships": "연동 관계",
  "Cycle check": "순환 점검",
  "References": "참조",
  "Source": "원천",
  "Target": "대상",
  "No relationships.": "등록된 연동 관계가 없습니다.",
  "Reference": "참조",
  "Cycle": "순환",
  "No field references.": "등록된 필드 참조가 없습니다.",
  "Creates a relationship used by automatic form propagation.": "서식 자동 전파에 사용하는 연동 관계를 생성합니다.",
  "Source form": "원천 서식",
  "Source field": "원천 필드",
  "Target form": "대상 서식",
  "Target field": "대상 필드",
  "Save relationship": "연동 관계 저장",
  "Migration target": "마이그레이션 대상",
  "Available form versions": "사용 가능 서식 버전",
  "Form migration": "서식 마이그레이션",
  "Dry-run, execute, or roll back business-year form data migrations to a target version.": "사업연도 서식 데이터를 대상 버전으로 사전 점검, 실행, 롤백합니다.",
  "Target versions": "대상 버전",
  "Default form": "기본 서식",
  "Select a business year and target version before running migration.": "마이그레이션 실행 전 사업연도와 대상 버전을 선택합니다.",
  "Business year": "사업연도",
  "Target version": "대상 버전",
  "Mode": "모드",
  "Dry run": "사전 점검",
  "Execute": "실행",
  "Rollback": "롤백",
  "Run migration": "마이그레이션 실행",
  "No business year available": "사용 가능한 사업연도가 없습니다.",
  "Outbound file mapping": "전자신고 파일 매핑",
  "E-filing map": "전자신고 매핑",
  "Inspect outbound record mappings between form fields and electronic filing records.": "서식 필드와 전자신고 레코드 간 송신 매핑을 확인합니다.",
  "Mapping rows": "매핑 행",
  "Mapped records": "매핑된 레코드",
  "Record type, field path, target field, and fixed-length metadata used by filing generation.": "신고파일 생성에 사용하는 레코드 유형, 필드 경로, 대상 필드, 고정길이 메타데이터입니다.",
  "Record": "레코드",
  "Length": "길이",
  "No e-file map rows.": "등록된 전자신고 매핑 행이 없습니다.",
  "Business-year form sets": "사업연도 서식 세트",
  "Review which form version set is applied to each business-year context.": "각 사업연도 컨텍스트에 적용된 서식 버전 세트를 확인합니다.",
  "Form sets": "서식 세트",
  "Version set mapping": "버전 세트 매핑",
  "Tenant business years": "테넌트 사업연도",
  "Configured or resolved form sets by business year.": "사업연도별 설정 또는 해석된 서식 세트입니다.",
  "Set": "세트",
  "No business-year form sets.": "등록된 사업연도 서식 세트가 없습니다.",
  "Available tenant years that can receive locked form snapshots.": "잠긴 서식 스냅샷을 받을 수 있는 테넌트 사업연도입니다.",
  "No business years.": "사업연도가 없습니다.",
  "Current impact snapshot": "현재 영향 스냅샷",
  "Impact simulation": "영향 시뮬레이션",
  "Estimate affected forms and open business years before activating a form version change.": "서식 버전 변경 활성화 전에 영향받는 서식과 열린 사업연도를 추정합니다.",
  "Affected years": "영향 사업연도",
  "Affected forms": "영향 서식",
  "Risk": "위험도",
  "Runs a server-side form-versioning impact check for the selected target version.": "선택한 대상 버전에 대해 서버 측 서식 버전 영향 점검을 실행합니다.",
  "Include locked years": "잠긴 사업연도 포함",
  "Run simulation": "시뮬레이션 실행",
  "Latest backend impact summary for form version administration.": "서식 버전 관리를 위한 최신 백엔드 영향 요약입니다.",
  "Item": "항목",
  "Value": "값",
  "Affected business years": "영향받는 사업연도",
  "Known forms": "알려진 서식",
  "Customer registered.": "고객사가 등록되었습니다.",
  "Business year created.": "사업연도가 생성되었습니다.",
  "Tax agent contract saved.": "세무대리 계약이 저장되었습니다.",
  "Menu node saved.": "메뉴 노드가 저장되었습니다.",
  "Menu functions saved.": "메뉴 기능이 저장되었습니다.",
  "Form master saved.": "서식 마스터가 저장되었습니다.",
  "Form version created.": "서식 버전이 생성되었습니다.",
  "Field definitions saved.": "필드 정의가 저장되었습니다.",
  "Validation rules saved.": "검증 규칙이 저장되었습니다.",
  "Relationship saved.": "연동 관계가 저장되었습니다.",
  "migration completed": "마이그레이션이 완료되었습니다.",
  "Audit log chain": "감사 로그 체인",
  "Hash verification": "해시 검증",
  "Authentication events": "인증 이벤트",
  "Review focus": "검토 초점",
  "Permission events": "권한 이벤트",
  "Current permission catalog": "현재 권한 목록",
  "System settings snapshot": "시스템 설정 스냅샷",
  "Audit posture": "감사 상태",
  "Tenant code registry": "테넌트 코드 등록부",
  "Function codes": "기능 코드",
  "Tenant code add": "테넌트 코드 추가",
  "Function Codes": "기능 코드",
  "Role Menu Functions": "역할별 메뉴 기능",
  "Menu Functions": "메뉴 기능",
  "Delegations": "위임",
  "Create Delegation": "위임 생성",
  "Imported balance sheet, income statement, and mapped statement lines.": "재무상태표, 손익계산서, 매핑된 명세 행입니다.",
  "Map customer accounts to standard accounts used by adjustment and forms.": "고객사 계정을 세무조정과 서식에서 사용하는 표준계정에 매핑합니다.",
  "Existing customer-to-standard account mappings.": "현재 고객사-표준계정 매핑입니다.",
  "Asset register, depreciation source rows, and business vehicle markers.": "자산대장, 감가상각 원천 행, 업무용 차량 표시입니다.",
  "Donation, entertainment, interest, and other adjustment source transactions.": "기부금, 접대비, 이자 등 세무조정 원천 거래입니다.",
  "Monthly business-use mileage used by business vehicle adjustment.": "업무용 차량 조정에 사용하는 월별 업무사용 주행거리입니다.",
  "Validation results tied to the main form.": "주요 서식과 연결된 검증 결과입니다.",
  "Generated status, validation count, and amount by form.": "서식별 생성 상태, 검증 건수, 금액입니다.",
  "Editable preview fields write back as manual overrides.": "편집 가능한 미리보기 항목은 수동 수정으로 저장됩니다.",
  "Cross-form deltas that affect validation readiness.": "검증 준비 상태에 영향을 주는 서식 간 차이입니다.",
  "Validation issues from the active form preview.": "활성 서식 미리보기에서 나온 검증 이슈입니다.",
  "Tax data consistency and filing readiness.": "세무 데이터 일관성과 신고 준비 상태입니다.",
  "Approval and filing depend on these checks.": "승인과 신고는 이 점검 결과에 따라 결정됩니다.",
  "Dismiss non-blocking issues or jump to the source screen.": "차단하지 않는 이슈를 해제하거나 원천 화면으로 이동합니다.",
  "Active and inactive validation rules for this tenant.": "이 테넌트의 활성/비활성 검증 규칙입니다.",
  "Creates approval lines and moves the business year into review.": "결재선을 만들고 사업연도를 검토 단계로 이동합니다.",
  "Current business year approval events.": "현재 사업연도의 결재 이벤트입니다.",
  "Business years currently waiting for my review.": "내 검토를 기다리는 사업연도입니다.",
  "Approve this work or return it to draft with a review comment.": "작업을 승인하거나 검토 의견과 함께 초안으로 반려합니다.",
  "Request and decision audit trail.": "요청과 결재 판단 감사 추적입니다.",
  "Events that moved the work back for correction.": "작업을 수정 단계로 되돌린 이벤트입니다.",
  "Use the related stage once corrections are complete.": "수정이 완료되면 관련 단계를 사용합니다.",
  "Choose the form used by the preview panel.": "미리보기 패널에서 사용할 서식을 선택합니다.",
  "Generated forms included in bulk output.": "일괄 출력에 포함되는 생성 서식입니다.",
  "Bulk output depends on approval and generated form state.": "일괄 출력은 승인과 생성 서식 상태에 따라 결정됩니다.",
  "Audit trail of downloaded output files.": "다운로드한 출력 파일 감사 추적입니다.",
  "Executed rules": "실행 규칙",
  "Warnings": "경고",
  "Infos": "정보",
  "Severity": "심각도",
  "Validation status: PASS": "검증 상태: 통과",
  "Validation status: ACTION REQUIRED": "검증 상태: 조치 필요",
  "Validation blocked": "검증 차단",
  "Ready for approval": "결재 가능",
  "Check": "점검",
  "State": "상태",
  "Financial data balanced": "재무 데이터 균형",
  "Vehicle logs": "차량 운행기록",
  "E-file precheck": "전자신고 사전점검",
  "OK": "정상",
  "CHECK": "점검",
  "Form review workbench": "서식 검토 작업장",
  "Generated": "생성됨",
  "Linkage": "연동",
  "Preview validations": "미리보기 검증",
  "Attachment catalog": "부속서식 목록",
  "Generate, select, and route into review for each attached form.": "각 부속서식을 생성, 선택하고 검토 화면으로 이동합니다.",
  "Generate selected": "선택 서식 생성",
  "Approved or filed work can be printed.": "승인 또는 신고 완료 작업은 출력할 수 있습니다.",
  "PDF output is gated until approval is complete.": "PDF 출력은 승인이 완료될 때까지 제한됩니다.",
  "FORM3 review": "별지 3호 검토",
  "Generate and review the main corporate tax return form.": "법인세 신고 주 서식을 생성하고 검토합니다.",
  "Generate FORM3": "별지 3호 생성",
  "Form": "서식",
  "Validations": "검증",
  "Amount": "금액",
  "Preview": "미리보기",
  "Generate": "생성",
  "No forms generated yet.": "아직 생성된 서식이 없습니다.",
  "Field": "필드",
  "Value": "값",
  "Source": "원천",
  "Jump": "이동",
  "No preview fields.": "미리보기 필드가 없습니다.",
  "Generate the selected form to review preview fields.": "미리보기 필드를 검토하려면 선택한 서식을 생성하세요.",
  "No form validation issues.": "서식 검증 이슈가 없습니다.",
  "No linkage differences.": "연동 차이가 없습니다.",
  "Manual review adjustment": "수동 검토 조정",
  "Save overrides": "수동 수정 저장",
  "No editable fields in the current preview.": "현재 미리보기에는 편집 가능한 필드가 없습니다.",
  "Change": "변경",
  "By": "처리자",
  "At": "시각",
  "No form edit history.": "서식 수정 이력이 없습니다.",
  "Form linkage check": "서식 연동 점검",
  "No form-to-form linkage differences are currently reported.": "현재 보고된 서식 간 연동 차이가 없습니다.",
  "Review linkage differences and jump to the affected form source.": "연동 차이를 검토하고 영향받은 서식 원천으로 이동합니다.",
  "Industry Statistics": "업종별 통계",
  "Industry": "업종",
  "Avg tax": "평균 세액",
  "Loss Expiry": "결손금 만료",
  "Origin": "발생연도",
  "Expires": "만료연도",
  "Remaining": "잔액",
  "Loss report": "결손금 리포트",
  "Report": "리포트",
  "Name": "이름",
  "Columns": "컬럼",
  "Open": "열기",
  "Access level": "접근권한",
  "Acted at": "처리일시",
  "Action": "작업",
  "Actor": "처리자",
  "Amendment sequence": "수정 순번",
  "Approval": "승인",
  "Approval state": "승인 상태",
  "Approver": "승인자",
  "Area": "영역",
  "Asset ID": "자산 ID",
  "Asset rows": "자산 행",
  "Audit ID": "감사 ID",
  "Broken rows": "오류 행",
  "Business km": "업무용 km",
  "Business no.": "사업자번호",
  "Category": "구분",
  "Changed": "변경시각",
  "Changed by": "변경자",
  "Checked rows": "점검 행",
  "Checksum": "체크섬",
  "Claim": "청구",
  "Comment": "의견",
  "Count": "건수",
  "Current": "현재",
  "Current module rows": "현재 모듈 행",
  "Current status": "현재 상태",
  "Date": "일자",
  "Delta": "차이",
  "Direction": "방향",
  "Display label": "표시 라벨",
  "Download enabled": "다운로드 가능",
  "E-file masters": "전자신고 마스터",
  "E-filing id": "전자신고 ID",
  "Effect": "효과",
  "Enabled": "사용",
  "Event": "이벤트",
  "Expected": "기대값",
  "Failed attempts": "실패 시도",
  "Feature flag": "기능 플래그",
  "File": "파일",
  "Filed lock": "신고 잠금",
  "Fix validation issues": "검증 이슈 수정",
  "Focus": "구분",
  "From": "이전",
  "Function": "기능",
  "Functions": "기능",
  "Gate": "게이트",
  "Generated file": "생성 파일",
  "Generated forms": "생성 서식",
  "Hash chain": "해시 체인",
  "Key": "키",
  "Lock mode": "잠금 모드",
  "Login": "로그인",
  "Menu": "메뉴",
  "Metric": "지표",
  "Module": "모듈",
  "Month": "월",
  "No effective access rows.": "유효 접근권한 행이 없습니다.",
  "Open inbox": "결재함 열기",
  "Original": "원신고",
  "Original business year": "원 사업연도",
  "Parent": "상위",
  "Pending days": "대기일",
  "Permission": "권한",
  "Permissions": "권한",
  "Policy": "정책",
  "Precheck": "사전점검",
  "Previous hash": "이전 해시",
  "Primary": "주 담당",
  "Printed at": "출력일시",
  "Printed by": "출력자",
  "Property": "속성",
  "Receipt id": "접수 ID",
  "Record count": "레코드 수",
  "Records": "레코드",
  "Refund": "환급",
  "Request approval again": "결재 다시 요청",
  "Role": "역할",
  "Route": "경로",
  "Row": "행",
  "Schema": "스키마",
  "Scopes": "업무범위",
  "Snapshot ID": "스냅샷 ID",
  "Sort": "정렬",
  "Standard": "표준",
  "Storage URL": "저장 URL",
  "Submitted at": "제출시각",
  "Success": "성공",
  "System": "시스템",
  "Table": "테이블",
  "Target scopes": "대상 업무범위",
  "Title": "제목",
  "To": "이후",
  "Total km": "총 km",
  "Transaction rows": "거래 행",
  "Type": "유형",
  "Unique IPs": "고유 IP",
  "Unique users": "고유 사용자",
  "Uploaded": "업로드",
  "User": "사용자",
  "Vehicle": "차량",
  "Watermark": "워터마크",
  "Workflow status": "워크플로 상태",
  "Actions that can be enabled per menu.": "메뉴별로 활성화할 수 있는 작업입니다.",
  "Approval inbox": "결재함",
  "Approval request": "결재 요청",
  "Attach evidence": "증빙 첨부",
  "Audit and change review": "감사 및 변경 검토",
  "Broken rows require database-level investigation before filing submission.": "오류 행은 신고 제출 전 데이터베이스 수준의 조사가 필요합니다.",
  "Canonical action codes referenced by menus, roles, and APIs.": "메뉴, 역할, API에서 참조하는 표준 작업 코드입니다.",
  "Choose a draft or active version and include locked snapshots only when auditing.": "초안 또는 활성 버전을 선택하고 감사 시에만 잠긴 스냅샷을 포함합니다.",
  "Codes used by customer, workflow, form, and adjustment screens.": "고객사, 워크플로, 서식, 세무조정 화면에서 사용하는 코드입니다.",
  "Context for interpreting historical changes.": "이력 변경을 해석하기 위한 컨텍스트입니다.",
  "Create": "생성",
  "Current global configuration visible to administrators.": "관리자에게 표시되는 현재 전역 설정입니다.",
  "Customer access controls": "고객사 접근 제어",
  "Customer master administration": "고객사 마스터 관리",
  "Customer-level target work scopes that limit user-level grants.": "사용자별 권한 부여를 제한하는 고객사 단위 대상 업무범위입니다.",
  "Decision comment": "결재 의견",
  "Dismiss": "해제",
  "Distribution after direct assignment, group/rule expansion, delegation, and override precedence.": "직접 배정, 그룹/규칙 확장, 위임, 예외 우선순위 적용 후의 분포입니다.",
  "Download PDF": "PDF 다운로드",
  "Failed attempts and unusual addresses should be reviewed before high-risk filing operations.": "고위험 신고 작업 전 실패 시도와 비정상 주소를 검토해야 합니다.",
  "Field, masking behavior, and role allowed to reveal values.": "필드, 마스킹 동작, 값 표시가 허용된 역할입니다.",
  "File name": "파일명",
  "Form version administration": "서식 버전 관리",
  "Include locked snapshots": "잠긴 스냅샷 포함",
  "Menu action catalog used by permission screens.": "권한 화면에서 사용하는 메뉴 작업 목록입니다.",
  "Menu and function governance": "메뉴 및 기능 관리",
  "Menu x function permissions for visible action buttons.": "표시되는 작업 버튼에 대한 메뉴 x 기능 권한입니다.",
  "No preview is available for the selected form.": "선택한 서식의 미리보기가 없습니다.",
  "Open business years potentially affected by the law version window.": "법령 버전 기간의 영향을 받을 수 있는 열린 사업연도입니다.",
  "Open catalog": "목록 열기",
  "Open issues": "열린 이슈",
  "Permission gate and feature flag by menu leaf.": "메뉴 leaf별 권한 게이트와 기능 플래그입니다.",
  "Print bundle": "출력 묶음",
  "Recalculate preview": "미리보기 재계산",
  "Replaces the allowed function set for one menu node.": "하나의 메뉴 노드에 허용된 기능 집합을 교체합니다.",
  "Request approval": "결재 요청",
  "Resolve validation": "검증 해결",
  "Return to draft": "초안으로 반려",
  "Reveal role": "표시 허용 역할",
  "Review completed from approval inbox.": "결재함에서 검토가 완료되었습니다.",
  "Role master rows used by menu and function permission checks.": "메뉴 및 기능 권한 점검에 사용하는 역할 마스터 행입니다.",
  "Role-level grants and denials that use the function catalog.": "기능 목록을 사용하는 역할 단위 허용/거부 권한입니다.",
  "Role/function changes captured for compliance review.": "준법 검토를 위해 기록된 역할/기능 변경입니다.",
  "Run validation": "검증 실행",
  "Read": "조회",
  "Update": "수정",
  "Delete": "삭제",
  "Import": "가져오기",
  "Export": "내보내기",
  "Calculate": "계산",
  "Approve": "승인",
  "E-file": "전자신고",
  "Print": "출력",
  "Mask off": "마스킹 해제",
  "Unmask": "마스킹 해제",
  "Delegate": "위임",
  "View records and screens": "레코드와 화면 조회",
  "Create records": "레코드 생성",
  "Update records": "레코드 수정",
  "Delete records": "레코드 삭제",
  "Import files or rows": "파일 또는 행 가져오기",
  "Export files or rows": "파일 또는 행 내보내기",
  "Run tax calculations": "세무 계산 실행",
  "Approve workflow items": "워크플로 항목 승인",
  "Create e-filing files": "전자신고 파일 생성",
  "Generate PDF/print output": "PDF/출력물 생성",
  "View unmasked sensitive fields": "마스킹 해제된 민감 필드 조회",
  "Delegate assigned access": "배정된 접근권한 위임",
  "carry forward taxable income": "과세표준 이월",
  "carry forward accounting income": "회계상 소득 이월",
  "carry forward donation amount": "기부금 금액 이월",
  "Policy endpoint": "정책 엔드포인트",
  "field-masking": "필드 마스킹",
  "industry_code starts 62": "업종코드가 62로 시작",
  "region = Seoul": "지역 = 서울",
  "demo": "데모",
  "enabled": "사용",
  "executed": "실행됨",
  "filed": "신고본",
  "current": "현재본",
  "Save data scope": "데이터 범위 저장",
  "Save masking policy": "마스킹 정책 저장",
  "Save menu functions": "메뉴 기능 저장",
  "Save parameter": "파라미터 저장",
  "Select a business year from this tenant to inspect its applied snapshot.": "적용된 스냅샷을 확인할 테넌트 사업연도를 선택합니다.",
  "Settings are reviewed alongside the tenant audit hash-chain status.": "설정은 테넌트 감사 해시 체인 상태와 함께 검토됩니다.",
  "Standard code": "표준 코드",
  "Standard name": "표준명",
  "Stores a versioned parameter row with category metadata.": "범주 메타데이터가 있는 버전별 파라미터 행을 저장합니다.",
  "Stores tenant-specific codes without changing the global master.": "전역 마스터를 변경하지 않고 테넌트별 코드를 저장합니다.",
  "Tenant and customer visibility rules.": "테넌트 및 고객사 표시 규칙입니다.",
  "Updates a representative tenant scope rule.": "대표 테넌트 범위 규칙을 수정합니다.",
  "Updates the representative business registration masking rule.": "대표 사업자등록번호 마스킹 규칙을 수정합니다.",
  "User login outcomes and source IP addresses.": "사용자 로그인 결과와 원천 IP 주소입니다.",
  "User, customer, access level, primary flag, and allowed work scopes.": "사용자, 고객사, 접근권한, 주 담당 여부, 허용 업무범위입니다.",
  "Validation issues": "검증 이슈",
  "Validation reviewed and ready for approval.": "검증이 검토되어 결재 요청할 수 있습니다.",
  "Which actions are currently attached to each menu leaf.": "각 메뉴 leaf에 현재 연결된 작업입니다.",
});

const KO_TEXT_REPLACEMENTS = Object.freeze([
  ["Demo Corporate Tax Workspace", "데모 법인세 신고 작업장"],
  ["Demo Tax Firm", "샘플 세무법인"],
  ["Alpha Manufacturing Co.", "알파 제조 주식회사"],
  ["Beta Platform Services", "베타 플랫폼 서비스"],
  ["Gamma Bio Research", "감마 바이오 연구소"],
  ["Dashboard Work Status", "대시보드 작업 상태"],
  ["Dashboard Recent Activity", "대시보드 최근 활동"],
  ["Dashboard Deadlines", "대시보드 마감 현황"],
  ["Dashboard Notifications", "대시보드 알림"],
  ["Dashboard Approval Actions", "대시보드 결재 작업"],
  ["Dashboard KPI Tax Burden", "대시보드 KPI 세부담"],
  ["Dashboard KPI Industry Loss", "대시보드 KPI 업종 결손"],
  ["Dashboard Customer", "대시보드 고객사"],
  ["Deadline Customer", "마감 관리 고객사"],
  ["Notification Customer", "알림 고객사"],
  ["Approval Action Customer", "결재 작업 고객사"],
  ["Activity Customer", "활동 이력 고객사"],
  ["KPI Customer", "KPI 고객사"],
  ["Software development", "소프트웨어 개발"],
  ["Cash", "현금"],
  ["Unspecified", "미지정"],
  ["Other", "기타"],
  ["Custom report", "사용자 정의 리포트"],
  ["Custom report", "사용자 리포트"],
  ["Loss expiry", "결손금 만료"],
  ["Tax burden", "세부담"],
  ["Reserve trend", "유보 추이"],
  ["Industry stats", "업종 통계"],
  ["Year comparison", "사업연도 비교"],
  ["Corporate Income Tax Act", "법인세법"],
  ["Corporate tax base and tax adjustment", "법인세 과세표준 및 세액조정계산서"],
  ["Income adjustment statement", "소득금액조정명세서"],
  ["Donation adjustment statement", "기부금 조정명세서"],
  ["Reserve rollforward statement", "유보 변동 명세서"],
  ["E-filing summary statement", "전자신고 요약 명세서"],
  ["Financial statement attachment", "재무제표 첨부서식"],
  ["Asset register attachment", "자산대장 첨부서식"],
  ["Transaction detail attachment", "거래명세 첨부서식"],
  ["Vehicle usage attachment", "업무용승용차 첨부서식"],
  ["Workflow approval attachment", "결재 승인 첨부서식"],
  ["Validation result attachment", "검증 결과 첨부서식"],
  ["Tax credit attachment", "세액공제 첨부서식"],
  ["Loss carryforward attachment", "이월결손금 첨부서식"],
  ["Foreign income attachment", "국외소득 첨부서식"],
  ["Consolidated tax attachment", "연결납세 첨부서식"],
  ["Approval requested", "결재 요청"],
  ["Approval completed", "결재 완료"],
  ["Approval returned", "결재 반려"],
  ["Filing completed", "신고 완료"],
  ["Amendment opened", "수정신고 시작"],
  ["A business year is waiting for approval", "사업연도가 결재 대기 중입니다."],
  ["All approval lines are approved", "모든 결재선이 승인되었습니다."],
  ["Approval was returned to draft", "결재가 반려되어 작성 단계로 돌아갔습니다."],
  ["The business year has been filed and locked", "사업연도 신고가 완료되어 잠겼습니다."],
  ["The filed business year was unlocked for amendment", "신고 완료된 사업연도가 수정신고용으로 열렸습니다."],
  ["Next step", "다음 단계"],
  ["0. Start", "0. 작업 시작"],
  ["1. Input", "1. 세무정보 입력"],
  ["2. Adjust", "2. 세무조정"],
  ["3. Forms", "3. 서식 작성"],
  ["4. Validate", "4. 검증"],
  ["5. Approve", "5. 결재"],
  ["6. Print", "6. 출력"],
  ["7. E-file", "7. 전자신고"],
  ["Filed work is complete", "신고 완료된 작업입니다."],
  ["Continue the filing workflow", "신고 작업을 계속 진행하세요."],
  ["Form engine generation", "서식 엔진 생성"],
  ["form engine generation", "서식 엔진 생성"],
  ["User override", "사용자 수동 수정"],
  ["user override", "사용자 수동 수정"],
  ["Tax adjustment summary", "세무조정 요약"],
  ["tax adjustment summary", "세무조정 요약"],
  ["created from form version", "서식 버전에서 생성됨"],
  ["migration can be executed", "마이그레이션을 실행할 수 있습니다."],
  ["target form version is not active", "대상 서식 버전이 활성 상태가 아닙니다."],
  ["migration executed", "마이그레이션이 실행되었습니다."],
  ["migration rolled back", "마이그레이션이 롤백되었습니다."],
  ["invalid tenant, login id, or password", "테넌트, 로그인 ID 또는 비밀번호가 올바르지 않습니다."],
  ["account locked", "계정이 잠겼습니다."],
  ["account is not active", "계정이 활성 상태가 아닙니다."],
  ["password expired", "비밀번호가 만료되었습니다."],
  ["tenant switch denied", "테넌트 전환 권한이 없습니다."],
  ["client IP is not allowed for this tenant", "이 테넌트에서 허용되지 않은 클라이언트 IP입니다."],
  ["2fa otp is required", "2FA OTP가 필요합니다."],
  ["2fa enrollment is required", "2FA 등록이 필요합니다."],
  ["invalid 2fa otp", "2FA OTP가 올바르지 않습니다."],
  ["missing authorization token", "인증 토큰이 없습니다."],
  ["authorization token must use Bearer scheme", "인증 토큰은 Bearer 방식을 사용해야 합니다."],
  ["authorization token must be a UUID", "인증 토큰은 UUID 형식이어야 합니다."],
  ["customer POST scope is required", "고객사 등록 권한이 필요합니다."],
  ["tenant access denied", "테넌트 접근 권한이 없습니다."],
  ["leaf record not found", "화면 레코드를 찾을 수 없습니다."],
  ["SUPER_ADMIN role is required", "SUPER_ADMIN 역할이 필요합니다."],
  ["dashboard KPI access denied", "대시보드 KPI 접근 권한이 없습니다."],
  ["customer not found for tenant", "테넌트의 고객사를 찾을 수 없습니다."],
  ["user not found", "사용자를 찾을 수 없습니다."],
  ["role not found", "역할을 찾을 수 없습니다."],
  ["invalid password: at least 8 characters required", "비밀번호는 8자 이상이어야 합니다."],
  ["2FA code", "2FA 코드"],
  ["Urgency", "긴급도"],
  ["Customer", "고객사"],
  ["Business year", "사업연도"],
  ["Due date", "마감일"],
  ["Status", "상태"],
  ["Work status", "업무현황"],
  ["Customers", "고객사"],
  ["deadline stable", "마감 안정"],
  ["Notifications / Approval inbox", "알림 / 결재 대기함"],
  ["Handle deadline alerts and approval queue in one place.", "마감 알림과 결재 대기 업무를 한 곳에서 처리합니다."],
  ["Alert center", "알림 센터"],
  ["Notifications", "알림"],
  ["Approval waiting", "결재 대기"],
  ["Dashboard notification tabs", "대시보드 알림 탭"],
  ["My approval inbox", "내 결재함"],
  ["No pending approvals.", "내 결재 대기 항목이 없습니다."],
  ["Requester", "요청자"],
  ["Waiting days", "대기일"],
  ["Approve", "승인"],
  ["Reject", "반려"],
  ["Detail", "상세"],
  ["No recent notifications.", "최근 알림이 없습니다."],
  ["Common", "공통"],
  ["Move", "이동"],
  ["Mark read", "읽음"],
  ["Recent activity", "최근활동"],
  ["Audit log feed summarizing recent changes.", "감사 로그를 업무 피드로 요약해 최근 변경된 화면으로 바로 이동합니다."],
  ["All audit logs", "감사 로그 전체"],
  ["Work change", "업무 변경"],
  ["No recent activity.", "최근활동이 없습니다."],
  ["Industry distribution", "업종별 법인 분포"],
  ["No industry data.", "업종별 법인 데이터가 없습니다."],
  ["Loss expiry forecast", "이월결손금 만료 예측"],
  ["No expiring loss carryforwards.", "만료 예정 이월결손금이 없습니다."],
  ["Key indicators", "핵심지표"],
  ["Tax burden analysis", "세부담 분석"],
  ["Average effective tax rate", "평균 실효세율"],
  ["Total tax burden", "총 부담세액"],
  ["Tax burden trend", "당기 세부담 추이"],
  ["Tax base", "과세표준"],
  ["Assigned customers", "담당 법인"],
  ["No tax burden trend data.", "세부담 추이 데이터가 없습니다."],
  ["No tax burden KPI permission.", "세부담 KPI 권한이 없습니다."],
  ["Tax burden analysis is available to administrators, tax experts, and reviewers.", "세부담 분석은 관리자, 세무전문가, 검토자 역할에서 조회할 수 있습니다."],
  ["Due soon", "신고마감 임박"],
  ["D-30, D-7, and D-Day filing work by priority and status.", "D-30, D-7, D-Day 신고 작업을 우선순위와 상태별로 확인합니다."],
  ["Dashboard", "대시보드"],
  ["Filing work status", "신고 업무 현황"],
  ["Review draft, validation, approval, approved, and filed statuses and jump to the next task.", "작성, 검증, 결재, 승인, 신고 완료 상태를 확인하고 다음 작업으로 바로 이동합니다."],
  ["Start filing work", "신고 작업 시작"],
  ["View all", "전체 보기"],
  ["returned", "반려"],
  ["needs redraft", "재작성 필요"],
  ["Cash", "현금"],
  ["Accounts payable", "미지급금"],
  ["Company sedan", "업무용 승용차"],
  ["CNC machine", "CNC 장비"],
  ["Good Charity", "좋은나눔재단"],
  ["Donation receipt", "기부금 영수증"],
  ["Client Dinner", "거래처 만찬"],
  ["Dinner meeting", "저녁 회의"],
]);

const EN_RENDERED_TEXT = Object.freeze(Object.fromEntries(
  Object.entries(KO_RENDERED_TEXT).map(([en, ko]) => [ko, en]),
));

export function localizeTextValue(value, locale) {
  if (locale !== "ko" && locale !== "en") return value;
  const trimmed = String(value || "").trim();
  if (!trimmed) return value;
  const translated = locale === "en" ? EN_RENDERED_TEXT[trimmed] : KO_RENDERED_TEXT[trimmed];
  if (translated) return String(value).replace(trimmed, translated);
  const dynamic = localizeDynamicTextValue(value, trimmed, locale);
  if (dynamic !== value) return dynamic;
  let localized = String(value);
  const replacements = [...KO_TEXT_REPLACEMENTS].sort((left, right) => {
    const leftSource = locale === "en" ? left[1] : left[0];
    const rightSource = locale === "en" ? right[1] : right[0];
    return rightSource.length - leftSource.length;
  });
  for (const [en, ko] of replacements) {
    const source = locale === "en" ? ko : en;
    const target = locale === "en" ? en : ko;
    localized = localized.split(source).join(target);
  }
  return localized !== String(value) ? localized : value;
}

function localizeDynamicTextValue(value, trimmed, locale) {
  if (locale === "ko") {
    const simulation = trimmed.match(/^Simulation completed for ([\d,]+) forms\.$/);
    if (simulation) return String(value).replace(trimmed, `시뮬레이션 완료: 영향 서식 ${simulation[1]}건`);
    const migration = trimmed.match(/^(.+): migration completed$/);
    if (migration) return String(value).replace(trimmed, `${migration[1]}: 마이그레이션이 완료되었습니다.`);
    const rows = trimmed.match(/^(.+): ([\d,]+) rows$/);
    if (rows) return String(value).replace(trimmed, `${rows[1]}: ${rows[2]}행`);
    const preview = trimmed.match(/^Preview recalculated at (.+)$/);
    if (preview) return String(value).replace(trimmed, `미리보기가 ${preview[1]}에 다시 계산되었습니다.`);
    const debitCredit = trimmed.match(/^debit total (.+) does not match credit total (.+)$/);
    if (debitCredit) return String(value).replace(trimmed, `차변 합계 ${debitCredit[1]}와 대변 합계 ${debitCredit[2]}가 일치하지 않습니다.`);
    const unsupportedAdjustment = trimmed.match(/^Unsupported adjustment module: (.+)$/);
    if (unsupportedAdjustment) return String(value).replace(trimmed, `지원하지 않는 세무조정 모듈: ${unsupportedAdjustment[1]}`);
    return value;
  }
  const simulation = trimmed.match(/^시뮬레이션 완료: 영향 서식 ([\d,]+)건$/);
  if (simulation) return String(value).replace(trimmed, `Simulation completed for ${simulation[1]} forms.`);
  const migration = trimmed.match(/^(.+): 마이그레이션이 완료되었습니다\.$/);
  if (migration) return String(value).replace(trimmed, `${migration[1]}: migration completed`);
  const rows = trimmed.match(/^(.+): ([\d,]+)행$/);
  if (rows) return String(value).replace(trimmed, `${rows[1]}: ${rows[2]} rows`);
  const preview = trimmed.match(/^미리보기가 (.+)에 다시 계산되었습니다\.$/);
  if (preview) return String(value).replace(trimmed, `Preview recalculated at ${preview[1]}`);
  const customerCount = trimmed.match(/^고객사 ([\d,]+)개$/);
  if (customerCount) return String(value).replace(trimmed, `${customerCount[1]} customers`);
  const urgent = trimmed.match(/^즉시 처리 필요 ([\d,]+)건$/);
  if (urgent) return String(value).replace(trimmed, `${urgent[1]} urgent`);
  const years = trimmed.match(/^최근 ([\d,]+)개년 당기 세부담 추이$/);
  if (years) return String(value).replace(trimmed, `Tax burden trend over the last ${years[1]} years`);
  const lossCaption = trimmed.match(/^향후 ([\d,]+)개년 만료 예정 잔액 (.+)$/);
  if (lossCaption) return String(value).replace(trimmed, `Expiring loss balance over the next ${lossCaption[1]} years ${lossCaption[2]}`);
  const returned = trimmed.match(/^반려 ([\d,]+)건 - 재작성 필요$/);
  if (returned) return String(value).replace(trimmed, `${returned[1]} returned - needs redraft`);
  const companyCount = trimmed.match(/^([\d,]+)개 법인$/);
  if (companyCount) return String(value).replace(trimmed, `${companyCount[1]} companies`);
  const year = trimmed.match(/^([12][0-9]{3})년$/);
  if (year) return String(value).replace(trimmed, year[1]);
  const lossRow = trimmed.match(/^([\d,]+)개 \/ ([\d,]+)건$/);
  if (lossRow) return String(value).replace(trimmed, `${lossRow[1]} customers / ${lossRow[2]} losses`);
  const latestTrend = trimmed.match(/^최신 (.+)년: 과세표준 (.+), 부담세액 (.+), 담당 법인 ([\d,]+)개$/);
  if (latestTrend) return String(value).replace(trimmed, `Latest ${latestTrend[1]}: tax base ${latestTrend[2]}, tax burden ${latestTrend[3]}, assigned customers ${latestTrend[4]}`);
  const debitCredit = trimmed.match(/^차변 합계 (.+)와 대변 합계 (.+)가 일치하지 않습니다\.$/);
  if (debitCredit) return String(value).replace(trimmed, `debit total ${debitCredit[1]} does not match credit total ${debitCredit[2]}`);
  const unsupportedAdjustment = trimmed.match(/^지원하지 않는 세무조정 모듈: (.+)$/);
  if (unsupportedAdjustment) return String(value).replace(trimmed, `Unsupported adjustment module: ${unsupportedAdjustment[1]}`);
  return value;
}

function localizeRenderedOutlet(outlet, locale) {
  if (!outlet || (locale !== "ko" && locale !== "en")) return;
  const walker = document.createTreeWalker(outlet, NodeFilter.SHOW_TEXT);
  const textNodes = [];
  while (walker.nextNode()) textNodes.push(walker.currentNode);
  textNodes.forEach((node) => {
    node.nodeValue = localizeTextValue(node.nodeValue, locale);
  });
  outlet.querySelectorAll("[placeholder], [aria-label], [title]").forEach((element) => {
    for (const attr of ["placeholder", "aria-label", "title"]) {
      if (element.hasAttribute(attr)) element.setAttribute(attr, localizeTextValue(element.getAttribute(attr), locale));
    }
  });
  outlet.querySelectorAll("option").forEach((option) => {
    option.textContent = localizeTextValue(option.textContent, locale);
    if (option.label) option.label = localizeTextValue(option.label, locale);
  });
}

const legacyRouteLabels = {
  dashboard: ["대시보드", "대시보드"],
  "ws-start": ["신고 작업", "0. 작업 시작"],
  "ws-info": ["신고 작업", "1. 세무정보 입력"],
  "ws-adj": ["신고 작업", "2. 세무조정"],
  "ws-form": ["신고 작업", "3. 서식 작성"],
  "ws-val": ["신고 작업", "4. 검증"],
  "ws-appr": ["신고 작업", "5. 결재"],
  "ws-print": ["신고 작업", "6. 출력"],
  "ws-file": ["신고 작업", "7. 전자신고"],
  "post-hist": ["사후 관리", "1. 신고 이력"],
  "post-amend": ["사후 관리", "2. 수정신고/경정청구"],
  "rp-alerts": ["분석/보고서", "1. 알림 센터"],
  "rp-compare": ["분석/보고서", "2. 사업연도 비교"],
  "rp-burden": ["분석/보고서", "3. 세부담 분석"],
  "rp-reserve": ["분석/보고서", "4. 유보 잔액 추이"],
  "ad-tenant": ["관리", "0. 테넌트 관리"],
  "ad-cust": ["관리", "A. 고객사 관리"],
  "ad-user-list": ["관리", "B. 사용자 관리"],
  "ad-role": ["관리", "C. 역할/권한 매트릭스"],
  "ad-menu-fn": ["관리", "D. 메뉴/기능 관리"],
  "ad-cacc": ["관리", "E. 담당 법인 권한"],
  "ad-law": ["관리", "F. 법령/세율 버전"],
  "ad-form": ["관리", "G. 서식 버전"],
  "ad-audit": ["관리", "H. 감사/로그"],
};

const legacyRoutes = Object.freeze(Object.fromEntries(Object.entries(legacyRouteLabels).map(([key, labels]) => {
  const [group, title] = labels;
  return [key, route(String(group), String(title), legacyLayout(key), key, key)];
})));

export const leafRoutes = Object.freeze({
  ...Object.fromEntries([
    ["dashboard:overview"],
    ["dashboard:duesoon"],
    ["dashboard:inbox"],
    ["dashboard:recent"],
    ["dashboard:kpi-tax"],
  ].map(([key]) => leafRoute(key, "plain", "dashboard"))),
  ...Object.fromEntries([
    ["ws/start:customer-pick"],
    ["ws/start:by-pick"],
    ["ws/start:snapshot"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-start"))),
  ...Object.fromEntries([
    ["ws/info:fs"],
    ["ws/info:mapping"],
    ["ws/info:assets"],
    ["ws/info:transactions"],
    ["ws/info:vehicle"],
    ["ws/info:consistency"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-info"))),
  ...Object.fromEntries([
    ["B1"],
    ["B2"],
    ["B3"],
    ["B4"],
    ["B5"],
    ["B6"],
    ["B7"],
    ["B8"],
    ["B9"],
    ["B10"],
    ["B11"],
    ["B12"],
    ["B13"],
    ["B14"],
    ["B15"],
    ["B16"],
    ["B17"],
  ].map(([code]) => leafRoute(`ws/adj:${code}`, "workspace", "ws-adj"))),
  ...Object.fromEntries([
    ["ws/form:form3"],
    ["ws/form:attachments"],
    ["ws/form:preview"],
    ["ws/form:linkage"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-form"))),
  ...Object.fromEntries([
    ["ws/val:run"],
    ["ws/val:issues"],
    ["ws/val:rules"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-val"))),
  ...Object.fromEntries([
    ["ws/appr:request"],
    ["ws/appr:inbox"],
    ["ws/appr:rejected"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-appr"))),
  ...Object.fromEntries([
    ["ws/print:preview"],
    ["ws/print:bulk"],
    ["ws/print:history"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-print"))),
  ...Object.fromEntries([
    ["ws/file:precheck"],
    ["ws/file:generate"],
    ["ws/file:submit"],
    ["ws/file:done"],
  ].map(([key]) => leafRoute(key, "workspace", "ws-file"))),
  ...Object.fromEntries([
    ["post/hist:list", "post-hist"],
    ["post/amend:unlock", "post-amend"],
    ["post/amend:version", "post-amend"],
    ["post/amend:diff", "post-amend"],
    ["post/amend:resubmit", "post-amend"],
    ["post/correction", "post-amend"],
  ].map(([key, delegate]) => leafRoute(key, "plain", delegate))),
  ...Object.fromEntries([
    ["report:year-compare", "rp-compare"],
    ["report:tax-burden", "rp-burden"],
    ["report:reserve-trend", "rp-reserve"],
    ["report:loss-expiry", "rp-reserve"],
    ["report:industry-stats", "rp-burden"],
    ["report:custom", "rp-reserve"],
  ].map(([key, delegate]) => leafRoute(key, "plain", delegate))),
  ...Object.fromEntries([
    ["admin/tenant:list", "ad-tenant"],
    ["admin/cust:list", "ad-cust"],
    ["admin/cust:by-master", "ad-cust"],
    ["admin/cust:agent", "ad-cust"],
    ["admin/sec:users", "ad-user-list"],
    ["admin/sec:roles", "ad-role"],
    ["admin/sec:matrix", "ad-role"],
    ["admin/sec:menus", "ad-menu-fn"],
    ["admin/sec:functions", "ad-menu-fn"],
    ["admin/sec:mask", "ad-role"],
    ["admin/sec:scope", "ad-role"],
    ["admin/cacc:assign", "ad-cacc"],
    ["admin/cacc:groups", "ad-cacc"],
    ["admin/cacc:rules", "ad-cacc"],
    ["admin/cacc:delegate", "ad-cacc"],
    ["admin/cacc:override", "ad-cacc"],
    ["admin/law:master", "ad-law"],
    ["admin/law:rates", "ad-law"],
    ["admin/law:limits", "ad-law"],
    ["admin/law:credits", "ad-law"],
    ["admin/law:depr-lives", "ad-law"],
    ["admin/law:sme", "ad-law"],
    ["admin/law:loss-rule", "ad-law"],
    ["admin/law:snapshots", "ad-law"],
    ["admin/law:impact", "ad-law"],
    ["admin/law:history", "ad-law"],
    ["admin/form:master", "ad-form"],
    ["admin/form:versions", "ad-form"],
    ["admin/form:fields", "ad-form"],
    ["admin/form:validations", "ad-form"],
    ["admin/form:linkage-rule", "ad-form"],
    ["admin/form:migration", "ad-form"],
    ["admin/form:efile-map", "ad-form"],
    ["admin/form:by-set", "ad-form"],
    ["admin/form:impact", "ad-form"],
    ["admin/code:manage", "ad-menu-fn"],
    ["admin/audit:events", "ad-audit"],
    ["admin/audit:login", "ad-audit"],
    ["admin/audit:perm", "ad-audit"],
    ["admin/audit:settings", "ad-audit"],
  ].map(([key, delegate]) => leafRoute(key, "admin", delegate))),
});

const routes = Object.freeze({ ...legacyRoutes, ...leafRoutes });

const screenByDelegate = {
  dashboard: renderDashboard,
  "ws-start": renderWorkStart,
  "ws-info": renderWorkInfo,
  "ws-adj": renderAdjustments,
  "ws-form": renderForms,
  "ws-val": renderValidation,
  "ws-appr": renderApproval,
  "ws-print": renderPrint,
  "ws-file": renderEfiling,
  "post-hist": renderPostHistory,
  "post-amend": renderPostAmend,
  "rp-alerts": renderAlerts,
  "rp-compare": renderYearCompare,
  "rp-burden": renderTaxBurden,
  "rp-reserve": renderReserveTrend,
  "ad-tenant": renderAdminTenants,
  "ad-cust": renderAdminCustomers,
  "ad-user-list": renderAdminUsers,
  "ad-role": renderAdminRoles,
  "ad-menu-fn": renderAdminMenus,
  "ad-cacc": renderAdminCustomerAccess,
  "ad-law": renderAdminLaw,
  "ad-form": renderAdminForms,
  "ad-audit": renderAdminAudit,
};

export const workflowLeafRendererContract = Object.freeze({
  "ws/start:customer-pick": "renderWorkStartCustomerPick",
  "ws/start:by-pick": "renderWorkStartBusinessYearPick",
  "ws/start:snapshot": "renderWorkStartSnapshot",
  "ws/info:fs": "renderWorkInfoFinancialStatements",
  "ws/info:mapping": "renderWorkInfoAccountMapping",
  "ws/info:assets": "renderWorkInfoAssets",
  "ws/info:transactions": "renderWorkInfoTransactions",
  "ws/info:vehicle": "renderWorkInfoVehicleUsage",
  "ws/info:consistency": "renderWorkInfoConsistency",
  "ws/adj:B1": "renderAdjustmentB1",
  "ws/adj:B2": "renderAdjustmentB2",
  "ws/adj:B3": "renderAdjustmentB3",
  "ws/adj:B4": "renderAdjustmentB4",
  "ws/adj:B5": "renderAdjustmentB5",
  "ws/adj:B6": "renderAdjustmentB6",
  "ws/adj:B7": "renderAdjustmentB7",
  "ws/adj:B8": "renderAdjustmentB8",
  "ws/adj:B9": "renderAdjustmentB9",
  "ws/adj:B10": "renderAdjustmentB10",
  "ws/adj:B11": "renderAdjustmentB11",
  "ws/adj:B12": "renderAdjustmentB12",
  "ws/adj:B13": "renderAdjustmentB13",
  "ws/adj:B14": "renderAdjustmentB14",
  "ws/adj:B15": "renderAdjustmentB15",
  "ws/adj:B16": "renderAdjustmentB16",
  "ws/adj:B17": "renderAdjustmentB17",
  "ws/form:form3": "renderFormsForm3",
  "ws/form:attachments": "renderFormsAttachments",
  "ws/form:preview": "renderFormsPreview",
  "ws/form:linkage": "renderFormsLinkage",
  "ws/val:run": "renderValidationRun",
  "ws/val:issues": "renderValidationIssues",
  "ws/val:rules": "renderValidationRules",
  "ws/appr:request": "renderApprovalRequest",
  "ws/appr:inbox": "renderApprovalInbox",
  "ws/appr:rejected": "renderApprovalRejected",
  "ws/print:preview": "renderPrintPreview",
  "ws/print:bulk": "renderPrintBulk",
  "ws/print:history": "renderPrintHistory",
  "ws/file:precheck": "renderEfilingPrecheck",
  "ws/file:generate": "renderEfilingGenerate",
  "ws/file:submit": "renderEfilingSubmit",
  "ws/file:done": "renderEfilingDone",
  "post/hist:list": "renderPostHistoryLeaf",
  "post/amend:unlock": "renderPostAmendUnlock",
  "post/amend:version": "renderPostAmendVersion",
  "post/amend:diff": "renderPostAmendDiff",
  "post/amend:resubmit": "renderPostAmendResubmit",
  "post/correction": "renderPostCorrection",
});

export const workflowStageContract = Object.freeze({
  workStart: {
    stage: "3.3",
    routes: ["ws/start:customer-pick", "ws/start:by-pick", "ws/start:snapshot"],
    renderer: "renderWorkStartLeaf",
    generic: false,
  },
  taxData: {
    stage: "3.5",
    routes: ["ws/info:fs", "ws/info:mapping", "ws/info:assets", "ws/info:transactions", "ws/info:vehicle", "ws/info:consistency"],
    renderer: "renderWorkInfoLeaf",
    generic: false,
  },
  adjustments: {
    stage: "3.6",
    routes: ["ws/adj:B1", "ws/adj:B2", "ws/adj:B3", "ws/adj:B4", "ws/adj:B5", "ws/adj:B6", "ws/adj:B7", "ws/adj:B8", "ws/adj:B9", "ws/adj:B10", "ws/adj:B11", "ws/adj:B12", "ws/adj:B13", "ws/adj:B14", "ws/adj:B15", "ws/adj:B16", "ws/adj:B17"],
    renderer: "renderAdjustmentLeaf",
    generic: false,
  },
  forms: {
    stage: "3.7",
    routes: ["ws/form:form3", "ws/form:attachments", "ws/form:preview", "ws/form:linkage"],
    renderer: "renderFormsLeaf",
    generic: false,
  },
  validation: {
    stage: "3.8",
    routes: ["ws/val:run", "ws/val:issues", "ws/val:rules"],
    renderer: "renderValidationLeaf",
    generic: false,
  },
  approval: {
    stage: "3.8",
    routes: ["ws/appr:request", "ws/appr:inbox", "ws/appr:rejected"],
    renderer: "renderApprovalLeaf",
    generic: false,
  },
  print: {
    stage: "3.9",
    routes: ["ws/print:preview", "ws/print:bulk", "ws/print:history"],
    renderer: "renderPrintLeaf",
    generic: false,
  },
  efiling: {
    stage: "3.10",
    routes: ["ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done"],
    renderer: "renderEfilingLeaf",
    generic: false,
  },
  postHistory: {
    stage: "3.11",
    routes: ["post/hist:list"],
    renderer: "renderPostHistoryLeaf",
    generic: false,
  },
  postAmend: {
    stage: "3.11",
    routes: ["post/amend:unlock", "post/amend:version", "post/amend:diff", "post/amend:resubmit", "post/correction"],
    renderer: "renderPostAmendLeaf",
    generic: false,
  },
});

function route(group, title, layout, delegate, key = "") {
  return {
    group,
    title,
    groupKey: groupKeyForDelegate(delegate),
    titleKey: routeKeyToLabelKey(key) || routeKeyToLabelKey(delegate) || title,
    layout,
    delegate,
    s1: false,
  };
}

function bindAdminRouteButtons(env) {
  localizeRenderedOutlet(env.outlet, env.locale);
  document.querySelectorAll("[data-admin-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.adminRoute));
  });
}

function leafRoute(key, layout, delegate) {
  const titleKey = routeKeyToLabelKey(key);
  const groupKey = groupKeyForDelegate(delegate);
  return [key, {
    group: groupKey,
    title: titleKey || key,
    groupKey,
    titleKey: titleKey || key,
    layout,
    delegate,
    leafKey: key,
    s1: true,
  }];
}

function groupKeyForDelegate(delegate) {
  if (delegate === "dashboard") return "nav.dashboard";
  if (String(delegate).startsWith("ws-")) return "nav.workspace";
  if (String(delegate).startsWith("post-")) return "nav.post";
  if (String(delegate).startsWith("rp-")) return "nav.reports";
  if (String(delegate).startsWith("ad-")) return "nav.admin";
  return routeKeyToLabelKey(delegate) || String(delegate || "");
}

function legacyLayout(key) {
  if (key.startsWith("ws-")) return "workspace";
  if (key.startsWith("ad-")) return "admin";
  return "plain";
}

export const adjustmentTaxonomy = Object.freeze([
  { code: "B1", ko: "소득금액조정명세서", en: "Income adjustment statement", module: "income", api: "adjustments/income" },
  { code: "B2", ko: "기부금", en: "Donations", module: "transactions", api: "adjustments/transactions/B2" },
  { code: "B3", ko: "접대비", en: "Entertainment expense", module: "transactions", api: "adjustments/transactions/B3" },
  { code: "B4", ko: "감가상각비", en: "Depreciation expense", module: "assets", api: "adjustments/assets/B4" },
  { code: "B5", ko: "퇴직급여충당금/퇴직연금", en: "Retirement allowance reserve/pension", module: "assets", api: "adjustments/assets/B5" },
  { code: "B6", ko: "대손충당금 및 대손금", en: "Bad debt reserve and bad debts", module: "assets", api: "adjustments/assets/B6" },
  { code: "B7", ko: "외화자산·부채 평가", en: "Foreign currency asset/liability valuation", module: "evaluation", api: "adjustments/evaluation/B7" },
  { code: "B8", ko: "재고자산·유가증권 평가", en: "Inventory/securities valuation", module: "evaluation", api: "adjustments/evaluation/B8" },
  { code: "B9", ko: "지급이자 손금불산입", en: "Non-deductible interest expense", module: "transactions", api: "adjustments/transactions/B9" },
  { code: "B10", ko: "업무용승용차 관련비용", en: "Business vehicle expenses", module: "assets", api: "adjustments/assets/B10" },
  { code: "B11", ko: "이월결손금", en: "Loss carryforward", module: "evaluation", api: "adjustments/evaluation/B11" },
  { code: "B12", ko: "세액공제·감면", en: "Tax credits/reductions", module: "tax", api: "adjustments/tax/B12" },
  { code: "B13", ko: "최저한세", en: "Minimum tax", module: "tax", api: "adjustments/tax/B13" },
  { code: "B14", ko: "가산세", en: "Additional tax", module: "tax", api: "adjustments/tax/B14" },
  { code: "B15", ko: "자본금과 적립금", en: "Capital and reserves", module: "evaluation", api: "adjustments/evaluation/B15" },
  { code: "B16", ko: "외국법인 세무조정", en: "Foreign corporation adjustment", module: "special", api: "adjustments/special/B16" },
  { code: "B17", ko: "연결납세", en: "Consolidated tax", module: "special", api: "adjustments/special/B17" },
]);

const adjustmentModules = adjustmentTaxonomy.map(({ code, ko, module }) => [code, ko, module]);

const adjustmentGridColumns = [
  { key: "source_module", labelKey: "field.module" },
  { key: "item_code", labelKey: "field.code" },
  { key: "item_name", labelKey: "field.item" },
  { key: "direction", labelKey: "field.direction" },
  { key: "amount", labelKey: "field.amount", format: "money" },
  { key: "disposition", labelKey: "field.disposition" },
];

const leafViewState = new Map();
const adjustmentRunState = new Map();
const validationRunState = new Map();

export const leafScreenSpecs = Object.freeze({
  "dashboard:overview": leafSpec("GET", "/api/tenants/{tenant}/dashboard", "dashboard", "READ"),
  "dashboard:duesoon": leafSpec("GET", "/api/tenants/{tenant}/dashboard/filing-deadlines?withinDays=30", "dashboard", "READ"),
  "dashboard:inbox": leafSpec("GET", "/api/tenants/{tenant}/dashboard/notifications?limit=50", "dashboard", "READ"),
  "dashboard:recent": leafSpec("GET", "/api/tenants/{tenant}/dashboard/recent-activities?limit=50", "audit", "READ"),
  "dashboard:kpi-tax": leafSpec("GET", "/api/tenants/{tenant}/dashboard/kpi/tax-burden?years=5", "reports", "READ"),
  "ws/start:customer-pick": leafSpec("GET", "/api/tenants/{tenant}/customers", "customer", "READ"),
  "ws/start:by-pick": leafSpec("GET", "/api/tenants/{tenant}/business-years", "customer", "READ"),
  "ws/start:snapshot": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/snapshot", "customer", "READ", { requires: ["work-context"] }),
  "ws/info:fs": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/financial-statements", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:mapping": leafSpec("GET", "/api/tenants/{tenant}/customers/{customerId}/account-mappings", "tax-data", "UPDATE", { requires: ["work-context"] }),
  "ws/info:assets": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/assets", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:transactions": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/transactions", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:vehicle": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/vehicle-usage-logs", "tax-data", "READ", { requires: ["work-context"] }),
  "ws/info:consistency": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/tax-data/validation", "tax-data", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B1": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/income", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B2": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B2", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B3": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B3", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B4": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B4", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B5": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B5", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B6": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B6", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B7": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B7", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B8": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B8", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B9": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/transactions/B9", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B10": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/assets/B10", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B11": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B11", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B12": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B12", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B13": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B13", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B14": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/tax/B14", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B15": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/evaluation/B15", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B16": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/special/B16", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/adj:B17": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/adjustments/special/B17", "adjustment", "CALCULATE", { requires: ["work-context"] }),
  "ws/form:form3": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "CREATE", { requires: ["work-context"] }),
  "ws/form:attachments": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/attachments", "forms", "CREATE", { requires: ["work-context"] }),
  "ws/form:preview": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "READ", { requires: ["work-context"] }),
  "ws/form:linkage": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/linkage-check", "forms", "READ", { requires: ["work-context"] }),
  "ws/val:run": leafSpec("POST", "/api/tenants/{tenant}/business-years/{byId}/validation/run", "validation", "CALCULATE", { requires: ["work-context"] }),
  "ws/val:issues": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/validation/issues", "validation", "READ", { requires: ["work-context"] }),
  "ws/val:rules": leafSpec("GET", "/api/tenants/{tenant}/validation/rules", "validation", "READ", { requires: ["work-context"] }),
  "ws/appr:request": leafSpec("POST", "/api/tenants/{tenant}/business-years/{byId}/workflow/request", "workflow", "APPROVE", { requires: ["work-context"] }),
  "ws/appr:inbox": leafSpec("GET", "/api/tenants/{tenant}/workflow/queue?assignee=me", "workflow", "READ", { requires: ["work-context"] }),
  "ws/appr:rejected": leafSpec("GET", "/api/tenants/{tenant}/workflow/events?status=REJECTED", "workflow", "READ", { requires: ["work-context"] }),
  "ws/print:preview": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/FORM3/preview", "forms", "READ", { requires: ["work-context"] }),
  "ws/print:bulk": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/forms/attachments", "forms", "PRINT", { requires: ["work-context"] }),
  "ws/print:history": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/print/history", "forms", "READ", { requires: ["work-context"] }),
  "ws/file:precheck": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/precheck", "efiling", "READ", { requires: ["work-context"] }),
  "ws/file:generate": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/format-spec", "efiling", "EFILE", { requires: ["work-context"] }),
  "ws/file:submit": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings", "efiling", "EFILE", { requires: ["work-context"] }),
  "ws/file:done": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings/latest", "efiling", "READ", { requires: ["work-context"] }),
  "post/hist:list": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/efilings", "efiling", "READ"),
  "post/amend:unlock": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "UPDATE", { requires: ["work-context"] }),
  "post/amend:version": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-version-mode", "post", "CREATE", { requires: ["work-context"] }),
  "post/amend:diff": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "READ", { requires: ["work-context"] }),
  "post/amend:resubmit": leafSpec("GET", "/api/tenants/{tenant}/business-years/{byId}/amendment-preview", "post", "EFILE", { requires: ["work-context"] }),
  "post/correction": leafSpec("GET", "/api/tenants/{tenant}/correction-claims", "post", "CREATE"),
  "report:year-compare": leafSpec("GET", "/api/tenants/{tenant}/reports/year-comparison", "reports", "READ"),
  "report:tax-burden": leafSpec("GET", "/api/tenants/{tenant}/reports/tax-burden", "reports", "READ"),
  "report:reserve-trend": leafSpec("GET", "/api/tenants/{tenant}/reports/reserve-trend", "reports", "READ"),
  "report:loss-expiry": leafSpec("GET", "/api/tenants/{tenant}/reports/loss-expiry", "reports", "READ"),
  "report:industry-stats": leafSpec("GET", "/api/tenants/{tenant}/reports/industry-statistics", "reports", "READ"),
  "report:custom": leafSpec("GET", "/api/tenants/{tenant}/reports/custom", "reports", "READ"),
  "admin/tenant:list": leafSpec("GET", "/api/tenants", "admin", "READ"),
  "admin/cust:list": leafSpec("GET", "/api/tenants/{tenant}/customers", "customer", "READ"),
  "admin/cust:by-master": leafSpec("GET", "/api/tenants/{tenant}/business-years?bare=true", "customer", "UPDATE"),
  "admin/cust:agent": leafSpec("GET", "/api/tenants/{tenant}/tax-agents", "customer", "UPDATE"),
  "admin/sec:users": leafSpec("GET", "/api/admin/tenants/{tenant}/users", "admin", "READ"),
  "admin/sec:roles": leafSpec("GET", "/api/admin/roles", "admin", "READ"),
  "admin/sec:matrix": leafSpec("GET", "/api/admin/role-permissions", "admin", "READ"),
  "admin/sec:menus": leafSpec("GET", "/api/admin/menus", "admin", "UPDATE"),
  "admin/sec:functions": leafSpec("GET", "/api/admin/functions", "admin", "UPDATE"),
  "admin/sec:mask": leafSpec("GET", "/api/admin/field-masking", "admin", "MASK_OFF"),
  "admin/sec:scope": leafSpec("GET", "/api/admin/data-scope", "admin", "UPDATE"),
  "admin/cacc:assign": leafSpec("GET", "/api/tenants/{tenant}/access-delegations", "permissions", "UPDATE"),
  "admin/cacc:groups": leafSpec("GET", "/api/admin/customer-groups", "permissions", "UPDATE"),
  "admin/cacc:rules": leafSpec("GET", "/api/admin/customer-rules", "permissions", "UPDATE"),
  "admin/cacc:delegate": leafSpec("GET", "/api/admin/access-delegations", "permissions", "DELEGATE"),
  "admin/cacc:override": leafSpec("GET", "/api/admin/customer-access/override", "permissions", "UPDATE"),
  "admin/law:master": leafSpec("GET", "/api/tax-laws", "law", "READ"),
  "admin/law:rates": leafSpec("GET", "/api/tax-rates", "law", "UPDATE"),
  "admin/law:limits": leafSpec("GET", "/api/tax-limits?category=LIMIT", "law", "UPDATE"),
  "admin/law:credits": leafSpec("GET", "/api/tax-limits?category=CREDIT", "law", "UPDATE"),
  "admin/law:depr-lives": leafSpec("GET", "/api/tax-limits?category=DEPRECIATION_LIFE", "law", "UPDATE"),
  "admin/law:sme": leafSpec("GET", "/api/tax-limits?category=SME_CRITERIA", "law", "UPDATE"),
  "admin/law:loss-rule": leafSpec("GET", "/api/tax-limits?category=LOSS_RULE", "law", "UPDATE"),
  "admin/law:snapshots": leafSpec("GET", "/api/law-versioning/summary", "law", "READ"),
  "admin/law:impact": leafSpec("GET", "/api/law-versioning/summary", "law", "CALCULATE"),
  "admin/law:history": leafSpec("GET", "/api/law-amendments", "law", "READ"),
  "admin/form:master": leafSpec("GET", "/api/form-versioning/forms", "forms", "READ"),
  "admin/form:versions": leafSpec("GET", "/api/form-versioning/versions", "forms", "READ"),
  "admin/form:fields": leafSpec("GET", "/api/form-versioning/versions/{formVersionId}/fields", "forms", "UPDATE"),
  "admin/form:validations": leafSpec("GET", "/api/form-versioning/versions/{formVersionId}/validations", "forms", "UPDATE"),
  "admin/form:linkage-rule": leafSpec("GET", "/api/form-versioning/relationships", "forms", "UPDATE"),
  "admin/form:migration": leafSpec("GET", "/api/form-versioning/versions", "forms", "CREATE"),
  "admin/form:efile-map": leafSpec("GET", "/api/form-versioning/efile-map", "forms", "UPDATE"),
  "admin/form:by-set": leafSpec("GET", "/api/form-versioning/by-set", "forms", "UPDATE"),
  "admin/form:impact": leafSpec("POST", "/api/form-versioning/impact", "forms", "CALCULATE"),
  "admin/code:manage": leafSpec("GET", "/api/tenants/{tenant}/codes?group=ALL", "admin", "UPDATE"),
  "admin/audit:events": leafSpec("GET", "/api/tenants/{tenant}/audit-logs", "audit", "READ"),
  "admin/audit:login": leafSpec("GET", "/api/login-history", "audit", "READ"),
  "admin/audit:perm": leafSpec("GET", "/api/permission-change-history", "audit", "READ"),
  "admin/audit:settings": leafSpec("GET", "/api/system-settings", "audit", "READ"),
});

export const screenByLeaf = Object.freeze({
  "dashboard:overview": (env) => renderDashboard(env),
  "dashboard:duesoon": (env) => renderDashboardDueSoon(env),
  "dashboard:inbox": (env) => renderDashboardInbox(env),
  "dashboard:recent": (env) => renderDashboardRecent(env),
  "dashboard:kpi-tax": (env) => renderDashboardKpiTax(env),
  "ws/start:customer-pick": (env) => renderWorkStartCustomerPick(env),
  "ws/start:by-pick": (env) => renderWorkStartBusinessYearPick(env),
  "ws/start:snapshot": (env) => renderWorkStartSnapshot(env),
  "ws/info:fs": (env) => renderWorkInfoFinancialStatements(env),
  "ws/info:mapping": (env) => renderWorkInfoAccountMapping(env),
  "ws/info:assets": (env) => renderWorkInfoAssets(env),
  "ws/info:transactions": (env) => renderWorkInfoTransactions(env),
  "ws/info:vehicle": (env) => renderWorkInfoVehicleUsage(env),
  "ws/info:consistency": (env) => renderWorkInfoConsistency(env),
  "ws/adj:B1": (env) => renderAdjustmentB1(env),
  "ws/adj:B2": (env) => renderAdjustmentB2(env),
  "ws/adj:B3": (env) => renderAdjustmentB3(env),
  "ws/adj:B4": (env) => renderAdjustmentB4(env),
  "ws/adj:B5": (env) => renderAdjustmentB5(env),
  "ws/adj:B6": (env) => renderAdjustmentB6(env),
  "ws/adj:B7": (env) => renderAdjustmentB7(env),
  "ws/adj:B8": (env) => renderAdjustmentB8(env),
  "ws/adj:B9": (env) => renderAdjustmentB9(env),
  "ws/adj:B10": (env) => renderAdjustmentB10(env),
  "ws/adj:B11": (env) => renderAdjustmentB11(env),
  "ws/adj:B12": (env) => renderAdjustmentB12(env),
  "ws/adj:B13": (env) => renderAdjustmentB13(env),
  "ws/adj:B14": (env) => renderAdjustmentB14(env),
  "ws/adj:B15": (env) => renderAdjustmentB15(env),
  "ws/adj:B16": (env) => renderAdjustmentB16(env),
  "ws/adj:B17": (env) => renderAdjustmentB17(env),
  "ws/form:form3": (env) => renderFormsForm3(env),
  "ws/form:attachments": (env) => renderFormsAttachments(env),
  "ws/form:preview": (env) => renderFormsPreview(env),
  "ws/form:linkage": (env) => renderFormsLinkage(env),
  "ws/val:run": (env) => renderValidationRun(env),
  "ws/val:issues": (env) => renderValidationIssues(env),
  "ws/val:rules": (env) => renderValidationRules(env),
  "ws/appr:request": (env) => renderApprovalRequest(env),
  "ws/appr:inbox": (env) => renderApprovalInbox(env),
  "ws/appr:rejected": (env) => renderApprovalRejected(env),
  "ws/print:preview": (env) => renderPrintPreview(env),
  "ws/print:bulk": (env) => renderPrintBulk(env),
  "ws/print:history": (env) => renderPrintHistory(env),
  "ws/file:precheck": (env) => renderEfilingPrecheck(env),
  "ws/file:generate": (env) => renderEfilingGenerate(env),
  "ws/file:submit": (env) => renderEfilingSubmit(env),
  "ws/file:done": (env) => renderEfilingDone(env),
  "post/hist:list": (env) => renderPostHistoryLeaf(env),
  "post/amend:unlock": (env) => renderPostAmendUnlock(env),
  "post/amend:version": (env) => renderPostAmendVersion(env),
  "post/amend:diff": (env) => renderPostAmendDiff(env),
  "post/amend:resubmit": (env) => renderPostAmendResubmit(env),
  "post/correction": (env) => renderPostCorrection(env),
  "report:year-compare": (env) => renderYearCompare(env),
  "report:tax-burden": (env) => renderTaxBurden(env),
  "report:reserve-trend": (env) => renderReserveTrend(env),
  "report:loss-expiry": (env) => renderLossExpiryReport(env),
  "report:industry-stats": (env) => renderIndustryStatsReport(env),
  "report:custom": (env) => renderCustomReports(env),
  "admin/tenant:list": (env) => renderAdminTenantLeaf(env),
  "admin/cust:list": (env) => renderAdminCustomerList(env),
  "admin/cust:by-master": (env) => renderAdminBusinessYearMaster(env),
  "admin/cust:agent": (env) => renderAdminTaxAgentContracts(env),
  "admin/sec:users": (env) => renderAdminUsers(env),
  "admin/sec:roles": (env) => renderAdminRoleCatalog(env),
  "admin/sec:matrix": (env) => renderAdminPermissionMatrix(env),
  "admin/sec:menus": (env) => renderAdminMenuManagement(env),
  "admin/sec:functions": (env) => renderAdminFunctionCatalog(env),
  "admin/sec:mask": (env) => renderAdminFieldMasking(env),
  "admin/sec:scope": (env) => renderAdminDataScope(env),
  "admin/cacc:assign": (env) => renderAdminCustomerAssignment(env),
  "admin/cacc:groups": (env) => renderAdminCustomerGroups(env),
  "admin/cacc:rules": (env) => renderAdminCustomerRules(env),
  "admin/cacc:delegate": (env) => renderAdminCustomerDelegation(env),
  "admin/cacc:override": (env) => renderAdminCustomerOverrides(env),
  "admin/law:master": (env) => renderAdminLawMaster(env),
  "admin/law:rates": (env) => renderAdminTaxRates(env),
  "admin/law:limits": (env) => renderAdminLawLimits(env),
  "admin/law:credits": (env) => renderAdminLawCredits(env),
  "admin/law:depr-lives": (env) => renderAdminLawDepreciationLives(env),
  "admin/law:sme": (env) => renderAdminLawSmeCriteria(env),
  "admin/law:loss-rule": (env) => renderAdminLawLossRules(env),
  "admin/law:snapshots": (env) => renderAdminLawSnapshots(env),
  "admin/law:impact": (env) => renderAdminLawImpact(env),
  "admin/law:history": (env) => renderAdminLawHistory(env),
  "admin/form:master": (env) => renderAdminFormMaster(env),
  "admin/form:versions": (env) => renderAdminFormVersions(env),
  "admin/form:fields": (env) => renderAdminFormFields(env),
  "admin/form:validations": (env) => renderAdminFormValidations(env),
  "admin/form:linkage-rule": (env) => renderAdminFormLinkageRules(env),
  "admin/form:migration": (env) => renderAdminFormMigration(env),
  "admin/form:efile-map": (env) => renderAdminFormEfileMap(env),
  "admin/form:by-set": (env) => renderAdminFormBySet(env),
  "admin/form:impact": (env) => renderAdminFormImpact(env),
  "admin/code:manage": (env) => renderAdminCodes(env),
  "admin/audit:events": (env) => renderAdminAuditEvents(env),
  "admin/audit:login": (env) => renderAdminLoginHistory(env),
  "admin/audit:perm": (env) => renderAdminPermissionChangeHistory(env),
  "admin/audit:settings": (env) => renderAdminSystemSettingsAudit(env),
});

async function renderWorkStartLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/start:by-pick") return renderWorkStartBusinessYearPick(env);
  if (activeLeaf === "ws/start:snapshot") return renderWorkStartSnapshot(env);
  return renderWorkStartCustomerPick(env);
}

async function renderWorkInfoLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/info:mapping") return renderWorkInfoAccountMapping(env);
  if (activeLeaf === "ws/info:assets") return renderWorkInfoAssets(env);
  if (activeLeaf === "ws/info:transactions") return renderWorkInfoTransactions(env);
  if (activeLeaf === "ws/info:vehicle") return renderWorkInfoVehicleUsage(env);
  if (activeLeaf === "ws/info:consistency") return renderWorkInfoConsistency(env);
  return renderWorkInfoFinancialStatements(env);
}

async function renderAdjustmentLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const moduleCode = String(activeLeaf || "").split(":")[1] || env.leafSuffix || "B1";
  if (moduleCode === "B2") return renderAdjustmentB2(env);
  if (moduleCode === "B3") return renderAdjustmentB3(env);
  if (moduleCode === "B4") return renderAdjustmentB4(env);
  if (moduleCode === "B5") return renderAdjustmentB5(env);
  if (moduleCode === "B6") return renderAdjustmentB6(env);
  if (moduleCode === "B7") return renderAdjustmentB7(env);
  if (moduleCode === "B8") return renderAdjustmentB8(env);
  if (moduleCode === "B9") return renderAdjustmentB9(env);
  if (moduleCode === "B10") return renderAdjustmentB10(env);
  if (moduleCode === "B11") return renderAdjustmentB11(env);
  if (moduleCode === "B12") return renderAdjustmentB12(env);
  if (moduleCode === "B13") return renderAdjustmentB13(env);
  if (moduleCode === "B14") return renderAdjustmentB14(env);
  if (moduleCode === "B15") return renderAdjustmentB15(env);
  if (moduleCode === "B16") return renderAdjustmentB16(env);
  if (moduleCode === "B17") return renderAdjustmentB17(env);
  return renderAdjustmentB1(env);
}

async function renderAdjustmentModuleLeaf(env, moduleCode) {
  await renderAdjustments({
    ...env,
    key: `ws/adj:${moduleCode}`,
    routeKey: `ws/adj:${moduleCode}`,
    leafKey: `ws/adj:${moduleCode}`,
    leafSuffix: moduleCode,
  });
}

async function renderAdjustmentB1(env) { await renderAdjustmentModuleLeaf(env, "B1"); }
async function renderAdjustmentB2(env) { await renderAdjustmentModuleLeaf(env, "B2"); }
async function renderAdjustmentB3(env) { await renderAdjustmentModuleLeaf(env, "B3"); }
async function renderAdjustmentB4(env) { await renderAdjustmentModuleLeaf(env, "B4"); }
async function renderAdjustmentB5(env) { await renderAdjustmentModuleLeaf(env, "B5"); }
async function renderAdjustmentB6(env) { await renderAdjustmentModuleLeaf(env, "B6"); }
async function renderAdjustmentB7(env) { await renderAdjustmentModuleLeaf(env, "B7"); }
async function renderAdjustmentB8(env) { await renderAdjustmentModuleLeaf(env, "B8"); }
async function renderAdjustmentB9(env) { await renderAdjustmentModuleLeaf(env, "B9"); }
async function renderAdjustmentB10(env) { await renderAdjustmentModuleLeaf(env, "B10"); }
async function renderAdjustmentB11(env) { await renderAdjustmentModuleLeaf(env, "B11"); }
async function renderAdjustmentB12(env) { await renderAdjustmentModuleLeaf(env, "B12"); }
async function renderAdjustmentB13(env) { await renderAdjustmentModuleLeaf(env, "B13"); }
async function renderAdjustmentB14(env) { await renderAdjustmentModuleLeaf(env, "B14"); }
async function renderAdjustmentB15(env) { await renderAdjustmentModuleLeaf(env, "B15"); }
async function renderAdjustmentB16(env) { await renderAdjustmentModuleLeaf(env, "B16"); }
async function renderAdjustmentB17(env) { await renderAdjustmentModuleLeaf(env, "B17"); }

async function renderFormsLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/form:attachments") return renderFormsAttachments(env);
  if (activeLeaf === "ws/form:preview") return renderFormsPreview(env);
  if (activeLeaf === "ws/form:linkage") return renderFormsLinkage(env);
  return renderFormsForm3(env);
}

async function renderValidationLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/val:issues") return renderValidationIssues(env);
  if (activeLeaf === "ws/val:rules") return renderValidationRules(env);
  return renderValidationRun(env);
}

async function renderApprovalLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/appr:inbox") return renderApprovalInbox(env);
  if (activeLeaf === "ws/appr:rejected") return renderApprovalRejected(env);
  return renderApprovalRequest(env);
}

async function renderPrintLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/print:bulk") return renderPrintBulk(env);
  if (activeLeaf === "ws/print:history") return renderPrintHistory(env);
  return renderPrintPreview(env);
}

async function renderEfilingLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "ws/file:generate") return renderEfilingGenerate(env);
  if (activeLeaf === "ws/file:submit") return renderEfilingSubmit(env);
  if (activeLeaf === "ws/file:done") return renderEfilingDone(env);
  return renderEfilingPrecheck(env);
}

async function renderPostHistoryLeaf(env) {
  await renderPostHistory(env);
}

async function renderPostAmendLeaf(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "post/amend:version") return renderPostAmendVersion(env);
  if (activeLeaf === "post/amend:diff") return renderPostAmendDiff(env);
  if (activeLeaf === "post/amend:resubmit") return renderPostAmendResubmit(env);
  if (activeLeaf === "post/correction") return renderPostCorrection(env);
  return renderPostAmendUnlock(env);
}

function leafSpec(method, path, module, fn, options = {}) {
  return {
    primary: { method, path },
    action: { method: "POST", path: "/api/tenants/{tenant}/leaf-actions" },
    perm: { module, function: fn },
    requires: options.requires || [],
    featureFlag: options.featureFlag || null,
    typology: options.typology || null,
    columns: options.columns || null,
    rowKey: options.rowKey || null,
    update: options.update || null,
    form: options.form || null,
    title: options.title || null,
    description: options.description || null,
    kpis: options.kpis || null,
  };
}

async function renderAdminTenantLeaf(env) {
  const key = "admin/tenant:list";
  const spec = enrichLeafSpec(key, leafScreenSpecs[key], env.locale);
  const meta = { ...(env.routeMeta || routeMeta(key, env.locale)), leafKey: key };
  const roles = env.auth?.user?.roles || [];
  if (!roles.includes("SUPER_ADMIN") && !roles.includes("TENANT_ADMIN")) {
    env.outlet.innerHTML = renderEmptyState(key, {
      kind: "perm",
      title: t(env.locale, "context.requiredTitle"),
      message: `SUPER_ADMIN / TENANT_ADMIN ${t(env.locale, "common.permission")}`,
    }, meta, spec, "", "", env.locale);
    return;
  }
  await renderAdminTenants(env);
}

async function renderLeafScreen(env, key) {
  const spec = enrichLeafSpec(key, leafScreenSpecs[key], env.locale);
  const meta = { ...(env.routeMeta || routeMeta(key, env.locale)), leafKey: key };
  const gate = leafGate(env, key, spec);
  if (gate) {
    env.outlet.innerHTML = renderEmptyState(key, gate, meta, spec, "", "", env.locale);
    bindEmptyStateActions(env, gate);
    return;
  }

  const primaryApi = resolveApiPath(spec.primary.path, env);
  const actionApi = resolveApiPath(spec.action.path, env);
  let primaryPayload;
  try {
    primaryPayload = await request(primaryApi, apiOptions(spec.primary, key, primaryApi, env));
  } catch (error) {
    env.outlet.innerHTML = renderEmptyState(key, {
      kind: "error",
      title: t(env.locale, "validation.loadFailed"),
      message: localizeTextValue(error.message, env.locale),
      action: "retry",
    }, meta, spec, primaryApi, actionApi, env.locale);
    bindEmptyStateActions(env, { action: "retry" });
    return;
  }

  const customPayload = await loadLeafRecords(env, key).catch(() => ({ rows: [] }));
  const primaryRows = normalizeLeafRows(primaryPayload, key, "api");
  const customRows = normalizeLeafRows(customPayload, key, "leaf_records");
  const rows = leafRowsForContext(key, env, [...customRows, ...primaryRows]);
  const state = {
    env,
    key,
    spec,
    meta,
    primaryApi,
    actionApi,
    rows,
    query: "",
    status: "ALL",
  };
  leafViewState.set(key, state);
  env.outlet.innerHTML = renderLeafTemplate(state);
  bindLeafTemplate(env, state);
}

async function loadLeafRecords(env, key) {
  return request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records?leaf_key=${encodeURIComponent(key)}`);
}

function leafRowsForContext(key, env, rows) {
  const customerId = env.context?.customerId;
  if (key !== "ws/start:by-pick" || !customerId) return rows;
  return rows.filter((row) => String(row.customer_id || "") === String(customerId));
}

const TYPOLOGY_RENDERERS = Object.freeze({
  grid: renderTypologyGrid,
  "grid-tree": renderTypologyGridTree,
  dashboard: renderTypologyDashboard,
  wizard: renderTypologyWizard,
  form: renderTypologyForm,
  chart: renderTypologyChart,
  detail: renderTypologyDetail,
});

const TYPOLOGY_GRID_TREE = new Set(["admin/sec:menus", "admin/form:fields", "admin/code:manage"]);
const TYPOLOGY_DASHBOARD = new Set(["dashboard:overview", "dashboard:duesoon", "dashboard:inbox", "dashboard:recent"]);
const TYPOLOGY_CHART = new Set(["dashboard:kpi-tax", "report:year-compare", "report:tax-burden", "report:reserve-trend", "report:loss-expiry", "report:industry-stats"]);
const TYPOLOGY_WIZARD = new Set(["ws/val:run", "ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done", "post/amend:resubmit", "admin/law:impact", "admin/form:migration", "admin/form:impact"]);
const TYPOLOGY_FORM = new Set(["ws/appr:request", "post/amend:unlock", "post/amend:version", "post/correction", "report:custom", "admin/cacc:delegate"]);
const TYPOLOGY_DETAIL = new Set(["ws/start:snapshot", "ws/info:fs", "ws/info:consistency", "ws/form:form3", "ws/form:preview", "ws/print:preview", "post/amend:diff", "admin/law:snapshots", "admin/form:by-set"]);
const LEAF_FORMATS = ["money", "bps", "date", "datetime", "biz", "corp", "tags", "status", "severity", "link", "boolean", "progress", "code", "email", "phone", "actions"];

function enrichLeafSpec(key, spec, locale = "ko") {
  const typology = spec.typology || leafTypology(key);
  return {
    ...spec,
    typology,
    rowKey: spec.rowKey || inferRowKey(key),
    update: spec.update || { method: "PATCH", path: "/api/tenants/{tenant}/leaf-records/{recordId}", fallback: "leaf-action" },
    description: spec.description || leafDescription(key, locale),
  };
}

function leafTypology(key) {
  if (TYPOLOGY_GRID_TREE.has(key)) return "grid-tree";
  if (TYPOLOGY_DASHBOARD.has(key)) return "dashboard";
  if (TYPOLOGY_CHART.has(key)) return "chart";
  if (TYPOLOGY_WIZARD.has(key)) return "wizard";
  if (TYPOLOGY_FORM.has(key)) return "form";
  if (TYPOLOGY_DETAIL.has(key)) return "detail";
  return "grid";
}

function inferRowKey(key) {
  if (key.includes("customer") || key === "admin/cust:list") return "customer_id";
  if (key.includes("by-pick") || key.includes("by-master") || key.includes("business-year")) return "by_id";
  if (key.includes("users")) return "login_id";
  if (key.includes("roles")) return "role_code";
  if (key.includes("menus")) return "menu_key";
  if (key.includes("law")) return "law_version_id";
  if (key.includes("form")) return "form_code";
  return "row_id";
}

function leafDescription(key, locale = "ko") {
  const typology = leafTypology(key);
  if (typology === "grid") return t(locale, "typology.grid.description");
  if (typology === "grid-tree") return t(locale, "typology.gridTree.description");
  if (typology === "dashboard") return t(locale, "typology.dashboard.description");
  if (typology === "wizard") return t(locale, "typology.wizard.description");
  if (typology === "form") return t(locale, "typology.form.description");
  if (typology === "chart") return t(locale, "typology.chart.description");
  return t(locale, "typology.detail.description");
}

function renderLeafTemplate(state) {
  const renderer = TYPOLOGY_RENDERERS[state.spec.typology] || renderTypologyGrid;
  return renderer(state);
}

function renderTypologyGrid(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="grid" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      ${renderLeafSummaryBlock(state, rows)}
      ${renderLeafTableBlock(state, rows, columns)}
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyGridTree(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology layout-tree-and-grid" data-typology="grid-tree" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <aside class="panel tree-panel">
        <div class="panel-head"><div><h2>${escapeHtml(t(locale, "common.category"))}</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
        ${renderLeafTree(state, rows)}
      </aside>
      <div class="grid-tree-main">
        ${renderLeafTableBlock(state, rows, columns)}
        ${renderLeafActionResult()}
      </div>
    </section>`;
}

function renderTypologyDashboard(state) {
  const rows = filterLeafRows(state);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="dashboard" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="dashboard-grid">
        ${dashboardMetrics(state, rows).map(([label, value, tone]) => `
          <article class="metric dashboard-metric ${escapeHtml(tone || "")}">
            <span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong>
          </article>`).join("")}
      </section>
      <section class="dashboard-secondary">
        ${dashboardCards(state, rows).map((card) => `
          <article class="panel">
            <div class="panel-head"><div><h2>${escapeHtml(card.title)}</h2><p>${escapeHtml(card.caption)}</p></div></div>
            ${card.body}
          </article>`).join("")}
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyWizard(state) {
  const locale = state.env.locale;
  const steps = [
    t(locale, "typology.wizard.step.prepare"),
    t(locale, "typology.wizard.step.validate"),
    t(locale, "typology.wizard.step.execute"),
    t(locale, "typology.wizard.step.result"),
  ];
  const active = wizardActiveStep(state.key);
  return `
    <section class="leaf-workbench leaf-typology" data-typology="wizard" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <ol class="wizard-stepper">
        ${steps.map((step, index) => `<li class="${index + 1 < active ? "done" : index + 1 === active ? "active" : ""}"><span>${index + 1}</span>${escapeHtml(step)}</li>`).join("")}
      </ol>
      <section class="panel wizard-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <button class="secondary-btn compact" type="button" data-step-edit data-row-id="${escapeHtml(firstRowId(state))}">${escapeHtml(t(locale, "typology.wizard.editStep"))}</button>
        </div>
        ${renderWizardBody(state, active)}
        <div class="wizard-nav">
          <button class="secondary-btn" type="button" data-wizard-prev ${active === 1 ? "disabled" : ""}>${escapeHtml(t(locale, "typology.wizard.previous"))}</button>
          <button class="primary-btn" type="button" data-wizard-next>${escapeHtml(active === steps.length ? t(locale, "typology.wizard.complete") : t(locale, "typology.wizard.next"))}</button>
        </div>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyForm(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = editableLeafColumns(state, row);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="form" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="grid two form-typology-body">
        <article class="panel">
          <div class="panel-head"><div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div></div>
          <form class="stack" data-leaf-form data-row-id="${escapeHtml(row.__rowId || "")}">
            ${columns.map((column) => renderEditField(column, row[column.key], locale)).join("")}
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.save"))}</button>
          </form>
        </article>
        <article class="panel form-preview">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.form.preview"))}</h2></div>
          ${renderObjectTable(row, leafColumns([row], state).slice(0, 6), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyChart(state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="chart" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      ${renderLeafSummaryBlock(state, rows)}
      <section class="panel chart-panel">
        <div class="panel-head">
          <div><h2>${escapeHtml(state.meta.title || state.key)}</h2><p>${escapeHtml(state.spec.description)}</p></div>
          <div class="panel-head-actions">
            <select data-chart-range aria-label="${escapeHtml(t(locale, "typology.chart.range"))}"><option>3y</option><option selected>5y</option><option>10y</option></select>
            <button class="secondary-btn compact" type="button" data-chart-config-edit data-row-id="${escapeHtml(firstRowId(state))}">${escapeHtml(t(locale, "typology.chart.editConfig"))}</button>
          </div>
        </div>
        <div class="chart-area" data-chart-target>
          ${renderChartBars(rows, locale)}
        </div>
        ${renderLeafTableShell(state, rows.slice(0, 8), columns)}
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderTypologyDetail(state) {
  const row = state.rows[0] || newLeafRecordData(state);
  const columns = leafColumns([row], state);
  const locale = state.env.locale;
  return `
    <section class="leaf-workbench leaf-typology" data-typology="detail" data-leaf-key="${escapeHtml(state.key)}" data-primary-api="${escapeHtml(state.primaryApi)}" data-action-api="${escapeHtml(state.actionApi)}">
      <section class="panel detail-header">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "typology.detail.badge"))}</span>
            <h2>${escapeHtml(detailTitle(state, row))}</h2>
            <p>${escapeHtml(state.spec.description)}</p>
          </div>
          <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(row.__rowId || "")}">${escapeHtml(t(locale, "common.edit"))}</button>
        </div>
      </section>
      <section class="grid two detail-body">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.detail.basic"))}</h2></div>
          ${renderObjectTable(row, columns.slice(0, 8), state)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "typology.detail.related"))}</h2></div>
          ${renderObjectTable(row, columns.slice(8, 16).length ? columns.slice(8, 16) : columns.slice(0, 4), state)}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
}

function renderLeafSummaryBlock(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const custom = rows.filter((row) => row.__source === "leaf_records").length;
  const locale = state.env.locale;
  return `
    <section class="panel leaf-summary" data-leaf-block="summary">
      <div class="panel-head">
        <div>
          <span class="badge info">${escapeHtml(state.spec.typology)}</span>
          <h2>${escapeHtml(state.meta.title || state.key)}</h2>
          <p>${escapeHtml(state.key)} · ${escapeHtml(state.spec.perm.module)}:${escapeHtml(state.spec.perm.function)}</p>
        </div>
      </div>
      ${metrics([
        [t(locale, "common.total"), money.format(rows.length)],
        [t(locale, "common.active"), money.format(active)],
        [t(locale, "common.custom"), money.format(custom)],
        [t(locale, "common.permission"), `${state.spec.perm.module}:${state.spec.perm.function}`],
      ])}
    </section>`;
}

function renderLeafTableBlock(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  return `
    <section class="panel leaf-table" data-leaf-block="table">
      <div class="panel-head">
        <div><h2>${escapeHtml(state.meta.title || t(locale, "common.list"))}</h2><p>${escapeHtml(t(locale, "leaf.count", { count: rows.length, description: state.spec.description }))}</p></div>
        <div class="panel-head-actions" data-leaf-block="toolbar">
          <div data-leaf-block="filters">
            ${renderLeafFilterControls(state)}
          </div>
          <button class="primary-btn compact" type="button" data-leaf-create="${escapeHtml(state.key)}">${escapeHtml(t(locale, "common.addPrefix"))}</button>
        </div>
      </div>
      ${renderLeafTableShell(state, rows, columns)}
    </section>`;
}

function renderLeafTableShell(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${columns.map((column) => `<th class="${escapeHtml(leafHeadClass(column))}">${escapeHtml(column.label)}</th>`).join("")}<th class="row-actions-th">${escapeHtml(t(locale, "common.actions"))}</th></tr></thead>
        <tbody data-leaf-table-body>${renderLeafTableRows(state, rows, columns)}</tbody>
      </table>
    </div>`;
}

function renderLeafFilterControls(state) {
  const locale = state.env.locale;
  return `
    <label class="inline-control">${escapeHtml(t(locale, "common.search"))} <input type="search" data-leaf-filter="q" value="${escapeHtml(state.query)}" placeholder="${escapeHtml(t(locale, "leaf.searchPlaceholder"))}" /></label>
    <label class="inline-control">${escapeHtml(t(locale, "field.status"))}
      <select data-leaf-filter="status">
        ${["ALL", "ACTIVE", "DRAFT", "IN_REVIEW", "APPROVED", "FILED", "SUSPENDED"].map((status) => `<option value="${status}" ${state.status === status ? "selected" : ""}>${escapeHtml(statusLabel(status, locale))}</option>`).join("")}
      </select>
    </label>
    <button class="secondary-btn compact" type="button" data-leaf-filter-reset>${escapeHtml(t(locale, "common.reset"))}</button>`;
}

function renderLeafTableRows(state, rows, columns = leafColumns(rows, state)) {
  const locale = state.env.locale;
  if (!rows.length) {
    return `<tr><td colspan="${columns.length + 1}"><div class="empty-state compact"><strong>${escapeHtml(t(locale, "typology.grid.emptyTitle"))}</strong><p class="empty">${escapeHtml(t(locale, "typology.grid.emptyDescription"))}</p></div></td></tr>`;
  }
  return rows.map((item) => `
    <tr data-leaf-row="${escapeHtml(item.__rowId)}">
      ${columns.map((column) => `<td class="${escapeHtml(leafCellClass(column))}" data-format="${escapeHtml(column.format)}">${formatLeafValue(item[column.key], column, item, state)}</td>`).join("")}
      <td class="row-actions" data-leaf-block="row-actions" data-format="actions">${renderLeafRowActions(state, item)}</td>
    </tr>`).join("");
}

function renderLeafRowActions(state, item) {
  const locale = state.env.locale;
  return `
    ${renderLeafPrimaryRowAction(state, item)}
    <button class="secondary-btn compact" type="button" data-row-edit data-leaf-row-action="edit" data-row-id="${escapeHtml(item.__rowId)}" title="${escapeHtml(t(locale, "common.edit"))}">${escapeHtml(t(locale, "common.edit"))}</button>
    <button class="danger-btn compact" type="button" data-row-delete data-leaf-row-action="delete" data-row-id="${escapeHtml(item.__rowId)}" title="${escapeHtml(t(locale, "common.delete"))}">${escapeHtml(t(locale, "common.delete"))}</button>`;
}

function renderLeafPrimaryRowAction(state, item) {
  if (state.key === "ws/start:customer-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-customer" data-row-id="${escapeHtml(item.__rowId)}">${escapeHtml(t(state.env.locale, "route.ws.start.customerPick"))}</button>`;
  }
  if (state.key === "ws/start:by-pick") {
    return `<button class="primary-btn compact" type="button" data-leaf-row-action="select-by" data-row-id="${escapeHtml(item.__rowId)}">${escapeHtml(t(state.env.locale, "route.ws.start.byPick"))}</button>`;
  }
  return "";
}

function renderLeafActionResult() {
  return `<div class="leaf-action-result" aria-live="polite"></div>`;
}

function bindLeafTemplate(env, state) {
  state.env = env;
  if (env.outlet.__leafClickHandler) env.outlet.removeEventListener("click", env.outlet.__leafClickHandler);
  if (env.outlet.__leafInputHandler) env.outlet.removeEventListener("input", env.outlet.__leafInputHandler);
  if (env.outlet.__leafSubmitHandler) env.outlet.removeEventListener("submit", env.outlet.__leafSubmitHandler);
  env.outlet.__leafClickHandler = (event) => handleLeafClick(event, env, state);
  env.outlet.__leafInputHandler = (event) => handleLeafInput(event, env, state);
  env.outlet.__leafSubmitHandler = (event) => handleLeafSubmit(event, env, state);
  env.outlet.addEventListener("click", env.outlet.__leafClickHandler);
  env.outlet.addEventListener("input", env.outlet.__leafInputHandler);
  env.outlet.addEventListener("submit", env.outlet.__leafSubmitHandler);
}

async function handleLeafClick(event, env, state) {
  const reset = event.target.closest("[data-leaf-filter-reset]");
  if (reset) {
    state.query = "";
    state.status = "ALL";
    rerenderLeaf(env, state);
    return;
  }

  const create = event.target.closest("[data-leaf-create]");
  if (create) {
    await createLeafRow(env, state, create);
    return;
  }

  const close = event.target.closest("[data-edit-close]");
  if (close) {
    closeLeafModal(env);
    return;
  }

  const actionButton = event.target.closest("[data-leaf-row-action], [data-step-edit], [data-card-edit], [data-chart-config-edit]");
  if (!actionButton) return;
  const row = findLeafRow(state, actionButton.dataset.rowId) || state.rows[0] || newLeafRecordData(state);
  const action = actionButton.dataset.leafRowAction || (actionButton.dataset.stepEdit !== undefined ? "edit" : "edit");
  actionButton.disabled = true;
  try {
    if (action === "select-customer") {
      selectLeafCustomer(env, state, row);
      return;
    }
    if (action === "select-by") {
      await selectLeafBusinessYear(env, state, row);
      return;
    }
    if (action === "edit") {
      openEditModal(env, state, row);
      return;
    }
    if (action === "delete") {
      await deleteLeafRow(env, state, row);
      state.rows = state.rows.filter((item) => item.__rowId !== row.__rowId);
      setLeafActionMessage(t(env.locale, "modal.deleteSuccess"), false, env.locale);
      rerenderLeaf(env, state);
    }
  } catch (error) {
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    actionButton.disabled = false;
  }
}

function handleLeafInput(event, env, state) {
  if (!event.target.matches("[data-leaf-filter]")) return;
  state.query = env.outlet.querySelector('[data-leaf-filter="q"]')?.value || "";
  state.status = env.outlet.querySelector('[data-leaf-filter="status"]')?.value || "ALL";
  refreshLeafRows(env, state);
}

async function handleLeafSubmit(event, env, state) {
  const editForm = event.target.closest("[data-leaf-edit-form]");
  const leafForm = event.target.closest("[data-leaf-form]");
  if (!editForm && !leafForm) return;
  event.preventDefault();
  const form = editForm || leafForm;
  const row = findLeafRow(state, form.dataset.rowId) || state.rows[0] || normalizeLeafRow(newLeafRecordData(state), state.key, "leaf_records", 0);
  const message = form.querySelector("[data-edit-error]");
  const submit = form.querySelector('button[type="submit"]');
  if (submit) submit.disabled = true;
  if (message) message.textContent = "";
  try {
    const values = readLeafFormValues(form, row);
    const updated = await updateLeafRow(env, state, row, values);
    upsertLeafRow(state, updated);
    closeLeafModal(env);
    setLeafActionMessage(t(env.locale, "modal.saveSuccess"), false, env.locale);
    rerenderLeaf(env, state);
  } catch (error) {
    if (message) message.textContent = localizeTextValue(error.message, env.locale);
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    if (submit) submit.disabled = false;
  }
}

async function createLeafRow(env, state, button) {
  button.disabled = true;
  try {
    const created = await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records`, {
      method: "POST",
      body: JSON.stringify({
        leaf_key: state.key,
        data: newLeafRecordData(state),
      }),
    });
    state.rows.unshift(normalizeLeafRow(created.row, state.key, "leaf_records", state.rows.length));
    setLeafActionMessage(t(env.locale, "leaf.addSuccess"), false, env.locale);
    rerenderLeaf(env, state);
  } catch (error) {
    setLeafActionMessage(error.message, true, env.locale);
  } finally {
    button.disabled = false;
  }
}

function openEditModal(env, state, row) {
  closeLeafModal(env);
  const columns = editableLeafColumns(state, row);
  const locale = env.locale;
  env.outlet.insertAdjacentHTML("beforeend", `
    <section class="leaf-modal-backdrop" data-leaf-modal>
      <form class="leaf-edit-modal" data-leaf-edit-form data-row-id="${escapeHtml(row.__rowId || "")}">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(locale, "modal.editTitle", { title: state.meta.title || state.key }))}</h2><p>${escapeHtml(row.__rowId || state.spec.rowKey || "-")}</p></div>
          <button class="secondary-btn compact" type="button" data-edit-close>${escapeHtml(t(locale, "common.cancel"))}</button>
        </div>
        <div class="form-grid">
          ${columns.map((column) => renderEditField(column, row[column.key], locale)).join("")}
        </div>
        <p class="edit-error" data-edit-error></p>
        <div class="button-row">
          <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.save"))}</button>
          <button class="secondary-btn" type="button" data-edit-close>${escapeHtml(t(locale, "common.cancel"))}</button>
        </div>
      </form>
    </section>`);
}

function closeLeafModal(env) {
  env.outlet.querySelector("[data-leaf-modal]")?.remove();
}

async function updateLeafRow(env, state, row, values = {}) {
  const updated = {
    ...row,
    ...values,
    status: values.status || row.status || nextLeafStatus(row.status),
    updated_at: today(),
  };
  if (row.__recordId) {
    const response = await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records/${encodeURIComponent(row.__recordId)}`, {
      method: "PATCH",
      body: JSON.stringify({ data: stripLeafInternalFields(updated) }),
    });
    return normalizeLeafRow(response.row, state.key, "leaf_records", 0);
  }
  await request(state.actionApi, {
    method: "POST",
    body: JSON.stringify({
      leaf_key: state.key,
      action: "update",
      row_id: row.__rowId,
      data: stripLeafInternalFields(updated),
    }),
  });
  return normalizeLeafRow(updated, state.key, row.__source || "api", 0);
}

async function deleteLeafRow(env, state, row) {
  if (row.__recordId) {
    await request(`/api/tenants/${encodeURIComponent(tenantCode(env))}/leaf-records/${encodeURIComponent(row.__recordId)}`, { method: "DELETE" });
    return;
  }
  await request(state.actionApi, {
    method: "POST",
    body: JSON.stringify({
      leaf_key: state.key,
      action: "delete",
      row_id: row.__rowId,
    }),
  });
}

function refreshLeafRows(env, state) {
  const rows = filterLeafRows(state);
  const columns = leafColumns(state.rows, state);
  const tbody = env.outlet.querySelector("[data-leaf-table-body]");
  if (tbody) tbody.innerHTML = renderLeafTableRows(state, rows, columns);
  const tableHead = env.outlet.querySelector('[data-leaf-block="table"] .panel-head p');
  if (tableHead) tableHead.textContent = t(env.locale, "leaf.count", { count: rows.length, description: state.spec.description });
}

function rerenderLeaf(env, state) {
  env.outlet.innerHTML = renderLeafTemplate(state);
  bindLeafTemplate(env, state);
}

function selectLeafCustomer(env, state, row) {
  const customerId = row.customer_id || row.id;
  if (!customerId) {
    throw new Error(t(env.locale, "context.missingCustomer"));
  }
  env.setContext({
    customerId,
    customerName: row.customer_name || row.name || row.customer_code || env.context.customerName,
  });
  setLeafActionMessage(t(env.locale, "context.customerSelected"), false, env.locale);
  env.navigate("ws/start:by-pick", { customerId });
}

async function selectLeafBusinessYear(env, state, row) {
  const byId = row.by_id || row.business_year_id || row.id;
  if (!byId || !row.customer_id) {
    throw new Error(t(env.locale, "context.missingBusinessYear"));
  }
  const by = { ...row, by_id: byId };
  const customer = await customerForBusinessYear(env, by);
  await refreshContextFromBy(env, by, customer);
  setLeafActionMessage(t(env.locale, "context.businessYearSelected"), false, env.locale);
  env.navigate("ws/info:fs", { byId: by.by_id, customerId: by.customer_id });
}

async function customerForBusinessYear(env, by) {
  if (String(env.context?.customerId || "") === String(by.customer_id || "")) {
    return {
      customer_id: by.customer_id,
      customer_name: env.context.customerName,
    };
  }
  const customers = await request(`${routeRoot(env)}/customers`).catch(() => []);
  return asArray(customers).find((item) => String(item.customer_id) === String(by.customer_id)) || {
    customer_id: by.customer_id,
    customer_name: by.customer_name || String(by.customer_id),
  };
}

function normalizeLeafRows(payload, key, source) {
  return extractLeafRows(payload).map((row, index) => normalizeLeafRow(row, key, source, index));
}

function normalizeLeafRow(row, key, source, index) {
  const object = row && typeof row === "object" && !Array.isArray(row) ? { ...row } : { value: row };
  const recordId = object.record_id || null;
  const rowId = recordId ? `record-${recordId}` : String(object.row_id || object.id || object[`${key.split(":")[0].split("/").pop()}_id`] || object.customer_id || object.by_id || object.login_id || object.menu_key || `api-${index + 1}`);
  return {
    ...object,
    __recordId: recordId,
    __rowId: rowId,
    __source: source,
  };
}

function extractLeafRows(payload) {
  if (Array.isArray(payload)) return payload;
  if (!payload || typeof payload !== "object") return payload === null || payload === undefined ? [] : [{ value: payload }];
  if (Array.isArray(payload.rows)) return payload.rows;
  const preferred = ["items", "customers", "business_years", "users", "roles", "permissions", "events", "issues", "rules", "attachments", "forms", "versions", "fields", "relationships", "rates", "limits", "logs", "histories", "reports", "notifications", "data"];
  for (const key of preferred) {
    if (Array.isArray(payload[key])) return payload[key];
  }
  const firstArray = Object.values(payload).find((value) => Array.isArray(value));
  if (firstArray) return firstArray;
  return [payload];
}

function leafColumns(rows, state = null) {
  const locale = state?.env?.locale || "ko";
  if (state?.spec?.columns?.length) {
    return state.spec.columns.map((column) => ({ ...column, label: column.labelKey ? t(locale, column.labelKey) : column.labels?.[locale] || column.label || fieldLabel(column.key, locale), format: column.format || inferColumnFormat(column.key) }));
  }
  const keys = [];
  rows.forEach((row) => {
    Object.keys(row || {}).forEach((key) => {
      if (!key.startsWith("__") && !["_source", "metadata", "snapshot_data", "payload"].includes(key) && !keys.includes(key)) keys.push(key);
    });
  });
  const selected = prioritizeLeafKeys(keys).slice(0, 7);
  return (selected.length ? selected : ["row_id", "title", "status"]).map((key) => ({
    key,
    label: leafColumnLabel(key, locale),
    format: inferColumnFormat(key),
  }));
}

function prioritizeLeafKeys(keys) {
  const preferred = ["row_id", "record_id", "tenant_code", "customer_code", "customer_name", "login_id", "role_code", "menu_key", "title", "name", "status", "severity", "year_label", "amount", "tax_due", "progress", "biz_reg_no", "corp_reg_no", "email", "phone", "created_at"];
  return [...preferred.filter((key) => keys.includes(key)), ...keys.filter((key) => !preferred.includes(key))];
}

function leafColumnLabel(key, locale = "ko") {
  return fieldLabel(key, locale);
}

function inferColumnFormat(key) {
  const normalized = key.toLowerCase();
  if (normalized === "actions") return "actions";
  if (normalized.includes("email")) return "email";
  if (normalized.includes("phone") || normalized.includes("mobile")) return "phone";
  if (normalized.includes("biz_reg")) return "biz";
  if (normalized.includes("corp_reg")) return "corp";
  if (normalized.includes("code") || normalized.endsWith("_id") || normalized === "id" || normalized.includes("key")) return "code";
  if (normalized.includes("bps") || normalized.includes("rate")) return "bps";
  if (normalized.includes("amount") || normalized.includes("tax") || normalized.includes("income") || normalized.includes("revenue") || normalized.includes("balance") || normalized.includes("refund")) return "money";
  if (normalized.includes("created_at") || normalized.includes("updated_at") || normalized.includes("acted_at") || normalized.includes("timestamp")) return "datetime";
  if (normalized.endsWith("_date") || normalized.includes("valid_from") || normalized.includes("valid_to") || normalized.includes("contract_")) return "date";
  if (normalized.includes("scopes") || normalized.includes("roles") || normalized.includes("tags")) return "tags";
  if (normalized === "status" || normalized === "state") return "status";
  if (normalized === "severity") return "severity";
  if (normalized.includes("url") || normalized.includes("link")) return "link";
  if (normalized.startsWith("is_") || normalized.includes("locked") || normalized.includes("active") || normalized.includes("valid") || normalized.includes("balanced")) return "boolean";
  if (normalized.includes("progress") || normalized.includes("percent")) return "progress";
  return "text";
}

function filterLeafRows(state) {
  const query = state.query.trim().toLowerCase();
  return state.rows.filter((row) => {
    const status = String(row.status || row.state || "").toUpperCase();
    const matchesStatus = state.status === "ALL" || status === state.status;
    const matchesQuery = !query || Object.values(row).some((value) => String(value ?? "").toLowerCase().includes(query));
    return matchesStatus && matchesQuery;
  });
}

function formatLeafValue(value, column = {}, row = {}, state = null) {
  if (value === null || value === undefined || value === "") return "-";
  const format = column.format || inferColumnFormat(column.key || "");
  if (format === "money") return `<span class="num">${escapeHtml(money.format(Number(value) || 0))}</span>`;
  if (format === "bps") return `${((Number(value) || 0) / 100).toFixed(2)}%`;
  if (format === "date") return escapeHtml(formatDate(value));
  if (format === "datetime") return escapeHtml(formatDateTime(value));
  if (format === "biz") return `<span class="code-cell">${escapeHtml(formatBizNo(value))}</span>`;
  if (format === "corp") return `<span class="code-cell">${escapeHtml(formatCorpNo(value))}</span>`;
  if (format === "tags") return renderTags(value);
  if (format === "status") return pill(value, state?.env?.locale || "ko");
  if (format === "severity") return `<span class="badge ${escapeHtml(severityClass(value))}">${escapeHtml(statusLabel(value, state?.env?.locale || "ko"))}</span>`;
  if (format === "link") return renderLeafLink(value, row, state);
  if (format === "boolean") return `<span class="boolean-mark ${value ? "yes" : "no"}">${value ? "Y" : "N"}</span>`;
  if (format === "progress") return renderProgress(value);
  if (format === "code") return `<span class="code-cell">${escapeHtml(value)}</span>`;
  if (format === "email") return escapeHtml(maskEmail(value));
  if (format === "phone") return escapeHtml(maskPhone(value));
  if (Array.isArray(value)) return renderTags(value);
  if (typeof value === "object") return escapeHtml(compactObjectLabel(value));
  return escapeHtml(value);
}

function compactObjectLabel(value) {
  const keys = Object.keys(value || {});
  if (!keys.length) return "{}";
  return keys.slice(0, 3).map((key) => `${key}:${value[key]}`).join(" · ");
}

function newLeafRecordData(state) {
  const locale = state.env?.locale || "ko";
  return {
    title: t(locale, "leaf.newItem", { title: state.meta.title || state.key }),
    status: "DRAFT",
    leaf_key: state.key,
    created_at: today(),
    owner: "UI",
  };
}

function stripLeafInternalFields(row) {
  return Object.fromEntries(Object.entries(row).filter(([key]) => !key.startsWith("__")));
}

function nextLeafStatus(status) {
  const value = String(status || "DRAFT").toUpperCase();
  if (value === "DRAFT") return "ACTIVE";
  if (value === "ACTIVE") return "IN_REVIEW";
  if (value === "IN_REVIEW") return "APPROVED";
  return "DRAFT";
}

function setLeafActionMessage(message, error = false, locale = "ko") {
  const result = document.querySelector(".leaf-action-result");
  if (result) result.innerHTML = `<strong>${escapeHtml(error ? t(locale, "leaf.actionFailed") : t(locale, "leaf.actionDone"))}</strong><p class="empty">${escapeHtml(localizeTextValue(message, locale))}</p>`;
}

function leafHeadClass(column) {
  return ["money", "bps", "progress"].includes(column.format) ? "align-right" : "";
}

function leafCellClass(column) {
  return ["money", "bps", "progress"].includes(column.format) ? "align-right" : "";
}

function editableLeafColumns(state, row = {}) {
  const blocked = new Set(["record_id", "row_id", "tenant_code", "leaf_key", "_source", state.spec.rowKey]);
  const columns = leafColumns([row], state).filter((column) => {
    const value = row[column.key];
    return !blocked.has(column.key)
      && column.format !== "actions"
      && value !== undefined
      && (value === null || typeof value !== "object" || Array.isArray(value));
  });
  if (columns.length) return columns.slice(0, 8);
  return [
    { key: "title", label: fieldLabel("title", state.env?.locale || "ko"), format: "text" },
    { key: "status", label: fieldLabel("status", state.env?.locale || "ko"), format: "status" },
  ];
}

function renderEditField(column, value, locale = "ko") {
  const inputType = editInputType(column.format);
  if (column.format === "boolean") {
    return `<label class="checkbox-field"><span>${escapeHtml(column.label)}</span><input name="${escapeHtml(column.key)}" type="checkbox" ${value ? "checked" : ""} /></label>`;
  }
  if (column.format === "tags") {
    return `<label>${escapeHtml(column.label)}<input name="${escapeHtml(column.key)}" value="${escapeHtml(asArray(value).join(", "))}" placeholder="${escapeHtml(t(locale, "validation.commaSeparated"))}" /></label>`;
  }
  if (String(value || "").length > 80) {
    return `<label>${escapeHtml(column.label)}<textarea name="${escapeHtml(column.key)}">${escapeHtml(value || "")}</textarea></label>`;
  }
  return `<label>${escapeHtml(column.label)}<input name="${escapeHtml(column.key)}" type="${inputType}" value="${escapeHtml(value ?? "")}" /></label>`;
}

function editInputType(format) {
  if (format === "date") return "date";
  if (format === "datetime") return "datetime-local";
  if (format === "money" || format === "bps" || format === "progress") return "number";
  if (format === "email") return "email";
  if (format === "phone") return "tel";
  return "text";
}

function readLeafFormValues(form, row) {
  const values = {};
  form.querySelectorAll("[name]").forEach((control) => {
    const key = control.name;
    const current = row[key];
    if (control.type === "checkbox") {
      values[key] = control.checked;
    } else if (Array.isArray(current)) {
      values[key] = control.value.split(",").map((item) => item.trim()).filter(Boolean);
    } else if (typeof current === "number") {
      values[key] = Number(control.value || 0);
    } else {
      values[key] = control.value;
    }
  });
  return values;
}

function findLeafRow(state, rowId) {
  if (!rowId) return null;
  return state.rows.find((row) => String(row.__rowId) === String(rowId)) || null;
}

function upsertLeafRow(state, row) {
  const index = state.rows.findIndex((item) => item.__rowId === row.__rowId);
  if (index >= 0) {
    state.rows[index] = row;
  } else {
    state.rows.unshift(row);
  }
}

function firstRowId(state) {
  return state.rows[0]?.__rowId || "";
}

function renderLeafTree(state, rows) {
  const groups = new Map();
  const locale = state.env.locale;
  rows.forEach((row) => {
    const raw = row.parent_key || row.menu_key || row.group_code || row.category || row.status || t(locale, "status.all");
    const label = String(raw).split(/[/:.]/)[0] || t(locale, "status.all");
    groups.set(label, (groups.get(label) || 0) + 1);
  });
  if (!groups.size) return `<p class="empty">${escapeHtml(t(locale, "leaf.emptyCategories"))}</p>`;
  return `<ul class="leaf-tree">${[...groups.entries()].map(([label, count]) => `<li><button type="button" class="secondary-btn compact" data-tree-node="${escapeHtml(label)}">${escapeHtml(label)} <span>${money.format(count)}</span></button></li>`).join("")}</ul>`;
}

function dashboardMetrics(state, rows) {
  const active = rows.filter((row) => String(row.status || row.state || "").toUpperCase() === "ACTIVE").length;
  const warnings = rows.filter((row) => ["WARN", "ERROR"].includes(String(row.severity || "").toUpperCase())).length;
  const locale = state.env.locale;
  return [
    [t(locale, "common.total"), money.format(rows.length), "info"],
    [t(locale, "status.active"), money.format(active), "ok"],
    [t(locale, "typology.dashboard.waiting"), money.format(rows.filter((row) => String(row.status || "").includes("PENDING")).length), "warn"],
    [t(locale, "typology.dashboard.warning"), money.format(warnings), "warn"],
    [t(locale, "common.custom"), money.format(rows.filter((row) => row.__source === "leaf_records").length), "info"],
  ];
}

function dashboardCards(state, rows) {
  const sample = rows.slice(0, 5);
  const locale = state.env.locale;
  const list = sample.length
    ? `<ul class="compact-list">${sample.map((row) => `<li><strong>${escapeHtml(detailTitle(state, row))}</strong><span>${escapeHtml(row.status ? statusLabel(row.status, locale) : row.severity || row.created_at || "-")}</span></li>`).join("")}</ul>`
    : `<p class="empty">${escapeHtml(t(locale, "leaf.emptyItems"))}</p>`;
  return [
    { title: t(locale, "typology.dashboard.overview"), caption: state.key, body: list },
    { title: t(locale, "typology.dashboard.recent"), caption: t(locale, "leaf.count", { count: sample.length, description: "" }).trim(), body: list },
    { title: t(locale, "typology.dashboard.guide"), caption: state.spec.typology, body: `<p class="empty">${escapeHtml(state.spec.description)}</p>` },
  ];
}

function wizardActiveStep(key) {
  if (key.endsWith(":generate")) return 2;
  if (key.endsWith(":submit") || key.endsWith(":resubmit")) return 3;
  if (key.endsWith(":done")) return 4;
  return 1;
}

function renderWizardBody(state, active) {
  const rows = state.rows.slice(0, 4);
  const locale = state.env.locale;
  return `
    <div class="wizard-body" data-wizard-step="${active}">
      ${metrics([
        [t(locale, "typology.wizard.next"), `${active}/4`],
        [t(locale, "common.item"), money.format(state.rows.length)],
        [t(locale, "field.status"), statusLabel(state.rows[0]?.status || "READY", locale)],
        [t(locale, "common.category"), state.spec.typology],
      ])}
      <div class="wizard-checklist">
        ${(rows.length ? rows : [newLeafRecordData(state)]).map((row, index) => `
          <article class="card">
            <span class="badge ${index + 1 <= active ? "ok" : "info"}">${index + 1}</span>
            <strong>${escapeHtml(detailTitle(state, row))}</strong>
            <p>${escapeHtml(row.status || row.state || state.spec.description)}</p>
          </article>`).join("")}
      </div>
    </div>`;
}

function renderChartBars(rows, locale = "ko") {
  const points = rows.slice(0, 8).map((row) => ({ label: chartLabel(row), value: chartValue(row) }));
  const max = Math.max(...points.map((point) => point.value), 1);
  if (!points.length) return `<p class="empty">${escapeHtml(t(locale, "leaf.noChartData"))}</p>`;
  return `<div class="chart-bars">${points.map((point) => `
    <div class="chart-bar-row">
      <span>${escapeHtml(point.label)}</span>
      <div class="chart-bar-track"><i style="width:${Math.max(4, Math.round(point.value / max * 100))}%"></i></div>
      <strong>${escapeHtml(money.format(point.value))}</strong>
    </div>`).join("")}</div>`;
}

function chartLabel(row) {
  return String(row.customer_name || row.report_name || row.year_label || row.item_name || row.title || row.row_id || row.__rowId || "-");
}

function chartValue(row) {
  const entry = Object.entries(row).find(([key, value]) => typeof value === "number" && !key.endsWith("_id"));
  return Math.max(0, Number(entry?.[1] || 0));
}

function detailTitle(state, row) {
  return row.customer_name || row.report_name || row.form_name || row.title || row.name || row.menu_key || row.login_id || row.role_code || row.__rowId || state.meta.title || state.key;
}

function renderObjectTable(object, columns, state) {
  const locale = state.env.locale;
  if (!columns.length) return `<p class="empty">${escapeHtml(t(locale, "leaf.emptyFields"))}</p>`;
  return table([t(locale, "common.item"), t(locale, "common.value")], columns.map((column) => row([
    escapeHtml(column.label),
    formatLeafValue(object[column.key], column, object, state),
  ])));
}

function formatDate(value) {
  const text = String(value || "");
  return text.includes("T") ? text.slice(0, 10) : text.slice(0, 10) || "-";
}

function formatDateTime(value) {
  const text = String(value || "");
  if (!text) return "-";
  return text.replace("T", " ").slice(0, 16);
}

function formatBizNo(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length !== 10) return String(value || "-");
  return `${digits.slice(0, 3)}-${digits.slice(3, 5)}-${digits.slice(5)}`;
}

function formatCorpNo(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length !== 13) return String(value || "-");
  return `${digits.slice(0, 6)}-${digits.slice(6)}`;
}

function renderTags(value) {
  const tags = Array.isArray(value) ? value : String(value || "").split(",").map((item) => item.trim()).filter(Boolean);
  if (!tags.length) return "-";
  return `<span class="tag-list">${tags.map((tag) => `<span class="tag-chip">${escapeHtml(typeof tag === "object" ? compactObjectLabel(tag) : tag)}</span>`).join("")}</span>`;
}

function severityClass(value) {
  const severity = String(value || "").toUpperCase();
  if (severity === "ERROR" || severity === "CRITICAL") return "danger";
  if (severity === "WARN" || severity === "WARNING") return "warn";
  return "info";
}

function renderLeafLink(value, row, state) {
  const href = String(value || "").startsWith("http") || String(value || "").startsWith("#") ? String(value) : keyToHash(String(value || state?.key || "dashboard:overview"));
  return `<a class="leaf-link" href="${escapeHtml(href)}">${escapeHtml(row.title || row.name || value)}</a>`;
}

function renderProgress(value) {
  const progress = Math.max(0, Math.min(100, Number(value) || 0));
  return `<div class="bar-track progress-cell"><span style="width:${progress}%"></span></div><span class="progress-label">${progress}%</span>`;
}

function maskEmail(value) {
  const text = String(value || "");
  const [name, domain] = text.split("@");
  if (!domain) return text;
  return `${name.slice(0, 2)}***@${domain}`;
}

function maskPhone(value) {
  const digits = String(value || "").replace(/\D/g, "");
  if (digits.length < 7) return String(value || "");
  return `${digits.slice(0, 3)}-****-${digits.slice(-4)}`;
}

function leafGate(env, key, spec) {
  const locale = env.locale || "ko";
  if (spec.requires.includes("work-context") && !hasWorkContext(env.context)) {
    return {
      kind: "ctx",
      title: t(locale, "context.requiredTitle"),
      message: t(locale, "context.requiredMessage"),
      action: "work-start",
    };
  }
  if (!canAccessLeaf(env, spec.perm)) {
    return {
      kind: "perm",
      title: t(locale, "context.requiredTitle"),
      message: `${spec.perm.module}:${spec.perm.function} ${t(locale, "common.permission")}`,
    };
  }
  const flag = spec.featureFlag || env.routeMeta?.feature_flag || null;
  if (flag && !isFeatureEnabled(env, flag)) {
    return {
      kind: "flag",
      title: t(locale, "menu.unavailable"),
      message: `${flag} ${t(locale, "menu.unavailable")}`,
    };
  }
  return null;
}

function canAccessLeaf(env, perm) {
  const permissions = env.auth?.permissions;
  if (!Array.isArray(permissions)) return true;
  return permissions.some((item) => {
    if (item === "*") return true;
    if (typeof item === "string") return item === `${perm.module}:${perm.function}` || item === `${perm.module}:*`;
    return item.module === perm.module && (item.function === perm.function || item.function === "*");
  });
}

function isFeatureEnabled(env, flag) {
  const flags = env.auth?.featureFlags || env.auth?.feature_flags || {};
  if (Array.isArray(flags)) return flags.includes(flag);
  return flags[flag] !== false;
}

function renderEmptyState(key, gate, meta, spec, primaryApi = "", actionApi = "", locale = "ko") {
  return `
    <section class="panel empty-state" data-leaf-key="${escapeHtml(key)}" data-empty-kind="${escapeHtml(gate.kind)}" data-primary-api="${escapeHtml(primaryApi)}" data-action-api="${escapeHtml(actionApi)}">
      <div class="panel-head">
        <div>
          <span class="badge warn">${escapeHtml(t(locale, "grid.emptyTitle"))}</span>
          <h2>${escapeHtml(meta.title || key)}</h2>
          <p>${escapeHtml(key)} / ${escapeHtml(spec.perm.module)}:${escapeHtml(spec.perm.function)}</p>
        </div>
      </div>
      <p class="empty">${escapeHtml(gate.message)}</p>
      ${gate.action === "work-start" ? `<button id="goStart" class="primary-btn" type="button">${escapeHtml(t(locale, "context.pickCustomerYear"))}</button>` : ""}
      ${gate.action === "retry" ? `<button id="retryLeaf" class="primary-btn" type="button">${escapeHtml(t(locale, "common.retry"))}</button>` : ""}
    </section>`;
}

function bindEmptyStateActions(env, gate) {
  if (gate.action === "work-start") {
    document.getElementById("goStart")?.addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  }
  if (gate.action === "retry") {
    document.getElementById("retryLeaf")?.addEventListener("click", () => env.navigate(env.routeKey || env.leafKey || env.key));
  }
}

function apiOptions(api, key, primaryApi, env) {
  if (api.method === "GET") return {};
  return {
    method: api.method,
    body: JSON.stringify({
      leaf_key: key,
      primary_api: primaryApi,
      tenant_code: tenantCode(env),
      business_year_id: env.context?.byId || 1,
      by_id: env.context?.byId || 1,
      customer_id: env.context?.customerId || 1,
      form_code: "FORM3",
      to_version_id: 1,
      law_version_id: 1,
      include_locked: false,
      actor: env.auth?.user?.login_id || "ui",
    }),
  };
}

function resolveApiPath(template, env) {
  const replacements = {
    tenant: tenantCode(env),
    byId: env.context?.byId || 1,
    customerId: env.context?.customerId || 1,
    formVersionId: env.context?.formVersionId || 1,
    efilingId: env.context?.efilingId || 1,
  };
  return template.replace(/\{(\w+)\}/g, (_, key) => encodeURIComponent(replacements[key] ?? ""));
}

function cssEscape(value) {
  if (window.CSS?.escape) return CSS.escape(value);
  return String(value).replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

export function routeMeta(key, locale = "ko") {
  const meta = routes[key] || routes.dashboard;
  return localizeRouteMeta({ group: meta.group, groupKey: meta.groupKey, title: meta.title, titleKey: meta.titleKey, layout: meta.layout, delegate: meta.delegate, s1: meta.s1 }, locale);
}

export async function refreshHealth(badge, text, locale = "ko") {
  try {
    await request("/health");
    badge.className = "health-badge ok";
    text.textContent = t(locale, "health.ok");
  } catch {
    badge.className = "health-badge error";
    text.textContent = t(locale, "health.error");
  }
}

export async function renderScreen(env) {
  const meta = routes[env.key] || routes.dashboard;
  const displayMeta = localizeRouteMeta({ ...meta, ...(env.routeMeta || {}) }, env.locale);
  const showFlowChrome = shouldShowFlowChrome(env.key, meta);
  if (showFlowChrome) {
    renderLawBanner(env.lawBanner, env.context, env.locale);
  } else {
    hideLawBanner(env.lawBanner);
  }
  const screen = screenByLeaf[env.key] || screenByDelegate[meta.delegate] || renderDashboard;
  if (screen !== renderDashboard) {
    stopDashboardRealtime();
  }
  const leafEnv = {
    ...env,
    key: meta.delegate,
    routeKey: env.key,
    routeMeta: displayMeta,
    leafKey: meta.s1 ? env.key : null,
    leafSuffix: meta.s1 ? leafSuffix(env.key) : null,
    leafTitle: displayMeta.title,
  };
  await screen(leafEnv);
  if (meta.s1) {
    prependLeafFocus(env.outlet, env.key, displayMeta, env.locale);
  }
  if (showFlowChrome) {
    await appendNextStepCard(env.outlet, leafEnv);
  }
  localizeRenderedOutlet(env.outlet, env.locale);
  localizeRenderedOutlet(document.body, env.locale);
}

function shouldShowFlowChrome(key, meta) {
  return meta.layout === "workspace"
    || key.startsWith("post/amend:")
    || key.startsWith("admin/law:")
    || key.startsWith("admin/form:");
}

function hideLawBanner(container) {
  container.classList.add("hidden");
  container.innerHTML = "";
}

function prependLeafFocus(outlet, key, meta, locale = "ko") {
  const section = document.createElement("section");
  section.className = "leaf-focus leaf-watermark";
  section.dataset.leafKey = key;
  section.dataset.leafDelegate = meta.delegate;
  const siblings = siblingLeafRoutes(meta.delegate, key);
  section.innerHTML = `
    <div>
      <div class="leaf-watermark-head">
        <span class="badge info">${escapeHtml(t(locale, "leaf.badge"))}</span>
        <strong>${escapeHtml(meta.title)}</strong>
        <span class="leaf-key">${escapeHtml(key)}</span>
      </div>
      <p>${leafFocusText(locale, key, meta.delegate)}</p>
      <div class="leaf-subnav" aria-label="${escapeHtml(t(locale, "leaf.siblingNavigation"))}">
        ${siblings.map(([siblingKey, siblingMeta]) => `
          <a class="${siblingKey === key ? "active" : ""}" href="${escapeHtml(keyToHash(siblingKey))}" data-leaf-nav="${escapeHtml(siblingKey)}">
            ${escapeHtml(localizeRouteMeta(siblingMeta, locale).title)}
          </a>`).join("")}
      </div>
    </div>
  `;
  outlet.prepend(section);
}

function siblingLeafRoutes(delegate, activeKey) {
  const activePrefix = activeKey.includes(":") ? activeKey.split(":")[0] : activeKey;
  return Object.entries(leafRoutes).filter(([key, meta]) => {
    const prefix = key.includes(":") ? key.split(":")[0] : key;
    return meta.delegate === delegate && prefix === activePrefix;
  });
}

function leafSuffix(key) {
  if (key.includes(":")) return key.split(":").slice(1).join(":");
  return key.split("/").pop();
}

function keyToHash(key) {
  if (key.includes(":")) {
    const [scope, suffix] = key.split(":");
    if (scope === "dashboard") return `#/dashboard/${suffix}`;
    if (scope.startsWith("ws/")) return `#/workspace/${scope}/${suffix}`;
    if (scope.startsWith("post/")) return `#/post/${scope.slice("post/".length)}/${suffix}`;
    if (scope === "report") return `#/report/${suffix}`;
    if (scope.startsWith("admin/")) return `#/admin/${scope.slice("admin/".length)}/${suffix}`;
  }
  if (key === "post/correction") return "#/post/correction";
  return `#/${key}`;
}

function tenantCode(env) {
  return env.auth?.user?.tenant_code || "demo";
}

function routeRoot(env) {
  return `/api/tenants/${tenantCode(env)}`;
}

function workRoot(env) {
  return `${routeRoot(env)}/business-years/${env.context.byId}`;
}

function requireWorkContext(env) {
  if (hasWorkContext(env.context)) return true;
  env.outlet.innerHTML = `
    <section class="panel empty-state work-context-empty" data-leaf-key="${escapeHtml(env.routeKey || env.leafKey || env.key)}">
      <div class="panel-head"><h2>${escapeHtml(t(env.locale, "context.requiredTitle"))}</h2></div>
      <p class="empty">${escapeHtml(t(env.locale, "context.requiredMessage"))}</p>
      <button id="goStart" class="primary-btn" type="button">${escapeHtml(t(env.locale, "context.pickCustomerYear"))}</button>
    </section>`;
  document.getElementById("goStart").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  return false;
}

function renderLawBanner(container, context, locale = "ko") {
  if (!hasWorkContext(context)) {
    container.classList.remove("hidden");
    container.classList.add("empty");
    container.innerHTML = `
      <div>
        <span>${escapeHtml(t(locale, "common.workflow"))}</span>
        <strong>${escapeHtml(t(locale, "context.select"))}</strong>
      </div>
      <button class="secondary-btn compact" type="button" data-flow-start>${escapeHtml(t(locale, "common.startWork"))}</button>`;
    container.querySelector("[data-flow-start]")?.addEventListener("click", () => {
      window.location.hash = "#/workspace/ws/start/customer-pick";
    });
    return;
  }
  const snapshot = context.snapshot || {};
  const data = snapshot.snapshot_data || {};
  container.classList.remove("hidden");
  container.classList.remove("empty");
  container.innerHTML = `
    <div><span>${escapeHtml(t(locale, "field.customerName"))}</span><strong>${escapeHtml(context.customerName || "-")}</strong></div>
    <div><span>${escapeHtml(t(locale, "field.yearLabel"))}</span><strong>${escapeHtml(context.fy || "-")}</strong></div>
    <div><span>${escapeHtml(t(locale, "nav.admin.law"))}</span><strong>${escapeHtml(lawLabel(data.law_version?.version_code || snapshot.law_version_id || "-"))}</strong></div>
    <div><span>${escapeHtml(t(locale, "nav.admin.forms"))}</span><strong>${escapeHtml(lawLabel(data.form?.version_no || data.form_version || snapshot.form_version_id || "-"))}</strong></div>
  `;
}

async function appendNextStepCard(outlet, env) {
  const key = env.routeKey || env.leafKey || env.key;
  if (!hasWorkContext(env.context)) {
    outlet.insertAdjacentHTML("beforeend", `
      <section class="flow-next-card" data-flow-card="${escapeHtml(key)}">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(env.locale, "common.nextStep"))}</h2><p class="empty">${escapeHtml(t(env.locale, "context.select"))}</p></div>
          <button class="primary-btn" type="button" data-next-leaf="ws/start:customer-pick">${escapeHtml(t(env.locale, "common.startWork"))}</button>
        </div>
      </section>`);
    bindNextStepNavigation(outlet, env);
    return;
  }
  let progress;
  try {
    progress = await request(`${workRoot(env)}/progress`);
  } catch {
    progress = { status: env.context.status || "DRAFT", next_leaf: "ws/info:fs", progress: env.context.progress || 0, recommendations: [] };
  }
  const next = progress.recommendations?.[0] || { leaf_key: progress.next_leaf || "ws/info:fs", label: t(env.locale, "common.nextStep"), enabled: true };
  outlet.insertAdjacentHTML("beforeend", `
    <section class="flow-next-card" data-flow-card="${escapeHtml(key)}" data-progress-api="${escapeHtml(`${workRoot(env)}/progress`)}">
      <div class="panel-head">
        <div>
          <span class="badge ok">${escapeHtml(t(env.locale, "common.workflow"))}</span>
          <h2>${escapeHtml(t(env.locale, "common.nextStep"))}</h2>
          <p>${escapeHtml(statusLabel(progress.status || env.context.status || "DRAFT", env.locale))} / ${escapeHtml(t(env.locale, "field.progress"))} ${escapeHtml(progress.progress ?? env.context.progress ?? 0)}%</p>
        </div>
        <button class="primary-btn" type="button" data-next-leaf="${escapeHtml(next.leaf_key)}" ${next.enabled === false ? "disabled" : ""}>${escapeHtml(next.label || t(env.locale, "common.nextStep"))}</button>
      </div>
    </section>`);
  bindNextStepNavigation(outlet, env);
}

function bindNextStepNavigation(outlet, env) {
  outlet.querySelectorAll("[data-next-leaf]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.nextLeaf));
  });
}

function lawLabel(value) {
  if (!value || value === "-") return "-";
  return String(value).replaceAll("_", " ");
}

function metrics(items) {
  return `<div class="grid four">${items.map(([label, value]) => `<article class="metric"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></article>`).join("")}</div>`;
}

function table(headers, rows, empty = t(currentDocumentLocale(), "grid.empty")) {
  return `
    <div class="table-wrap">
      <table>
        <thead><tr>${headers.map((head) => `<th>${escapeHtml(head)}</th>`).join("")}</tr></thead>
        <tbody>${rows.length ? rows.join("") : `<tr><td colspan="${headers.length}">${escapeHtml(empty)}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function currentDocumentLocale() {
  return typeof document === "undefined" ? "ko" : document.documentElement.lang;
}

function row(cells) {
  return `<tr>${cells.map((cell) => `<td>${cell}</td>`).join("")}</tr>`;
}

function pill(status, locale = "ko") {
  return `<span class="status-pill ${statusClass(status)}">${escapeHtml(statusLabel(status, locale))}</span>`;
}

function renderSnapshotSummary(snapshot, locale = currentDocumentLocale()) {
  const data = snapshot?.snapshot_data || {};
  const law = data.law || data.law_version || {};
  const form = data.form || {};
  return table([t(locale, "common.item"), t(locale, "common.value")], [
    row(["Snapshot ID", escapeHtml(snapshot?.snapshot_id || "-")]),
    row([escapeHtml(t(locale, "nav.admin.law")), escapeHtml(law.version_code || snapshot?.law_version_id || "-")]),
    row([escapeHtml(t(locale, "nav.admin.forms")), escapeHtml(form.version_no || form.form_version || snapshot?.form_version_ids || "-")]),
    row([escapeHtml(t(locale, "status.locked")), snapshot?.locked ? "Y" : "N"]),
  ]);
}

function renderTaxDataValidationSummary(validation) {
  return table(["항목", "값"], [
    row(["차변 합계", money.format(validation.debit_total || 0)]),
    row(["대변 합계", money.format(validation.credit_total || 0)]),
    row(["차대 일치", validation.balanced ? "Y" : "N"]),
    row(["미매핑 계정", money.format(validation.unresolved_mapping_count || 0)]),
    row(["배치 오류", money.format(validation.batch_error_count || 0)]),
  ]);
}

function renderValidationOverview(taxData, efile) {
  return `
    ${metrics([
      ["차대 일치", taxData.balanced ? "Y" : "N"],
      ["미매핑", money.format(taxData.unresolved_mapping_count || 0)],
      ["배치 오류", money.format(taxData.batch_error_count || 0)],
      ["전자신고", efile?.valid ? "가능" : "확인 필요"],
    ])}
    ${table(["검증 항목", "현재 값"], [
      row(["재무제표 라인", money.format(taxData.fs_line_count || 0)]),
      row(["자산", money.format(taxData.asset_count || 0)]),
      row(["업무용 차량", money.format(taxData.business_vehicle_count || 0)]),
      row(["거래", money.format(taxData.transaction_count || 0)]),
    ])}`;
}

async function refreshContextFromBy(env, by, customer) {
  const snapshot = await request(`${routeRoot(env)}/business-years/${by.by_id}/snapshot`);
  env.setContext({
    customerId: by.customer_id,
    customerName: customer?.customer_name || env.context.customerName,
    byId: by.by_id,
    fy: String(by.year_label),
    period: `${by.start_date} ~ ${by.end_date}`,
    status: by.status,
    progress: progressForStatus(by.status),
    snapshot,
    lockMode: by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN"),
  });
}

function formatDday(daysRemaining) {
  const days = Number(daysRemaining ?? 0);
  if (days === 0) return "D-Day";
  if (days < 0) return `D+${Math.abs(days)}`;
  return `D-${days}`;
}

function deadlineUrgencyClass(urgencyLevel) {
  const normalized = String(urgencyLevel || "NOTICE").toLowerCase();
  return `deadline-${normalized}`;
}

function formatNotificationTime(value) {
  if (!value) return "-";
  return String(value).replace("T", " ").slice(0, 16);
}

function notificationSeverityClass(severity) {
  if (severity === "ERROR") return "danger";
  if (severity === "WARN") return "warn";
  if (severity === "OK") return "ok";
  return "info";
}

function renderDashboardDeadlineTable(deadlines, locale = "ko") {
  const rows = asArray(deadlines).map((item) => {
    const statusText = item.status === "DRAFT"
      ? `${item.statusLabel || statusLabel(item.status, locale)} (${item.progressPct || 0}%)`
      : item.statusLabel || statusLabel(item.status, locale);
    return `
      <tr class="deadline-row ${escapeHtml(deadlineUrgencyClass(item.urgencyLevel))}" data-deadline-by="${escapeHtml(item.businessYearId)}" tabindex="0">
        <td><span class="badge ${item.urgencyLevel === "CRITICAL" ? "danger" : item.urgencyLevel === "WARNING" ? "warn" : "info"}">${escapeHtml(formatDday(item.daysRemaining))}</span></td>
        <td>${escapeHtml(item.customerName)}</td>
        <td>${escapeHtml(item.fiscalYear)}</td>
        <td>${escapeHtml(item.filingDueDate)}</td>
        <td>${pill(item.status, locale)} <span class="muted">${escapeHtml(statusText)}</span></td>
      </tr>`;
  });
  return `
    <div class="table-wrap dashboard-deadlines" data-dashboard-section="filing-deadlines">
      <table>
        <thead><tr><th>긴급도</th><th>고객사</th><th>사업연도</th><th>마감일</th><th>상태</th></tr></thead>
        <tbody>${rows.length ? rows.join("") : `<tr><td colspan="5">${escapeHtml(t(locale, "grid.empty"))}</td></tr>`}</tbody>
      </table>
    </div>`;
}

function dashboardStatusRoute(status) {
  return {
    DRAFT: "ws/start:customer-pick",
    IN_REVIEW_VALIDATION: "ws/val:run",
    IN_REVIEW_APPROVAL: "ws/appr:inbox",
    APPROVED: "ws/print:preview",
    FILED: "post/hist:list",
  }[status] || "dashboard:overview";
}

function renderDashboardWorkStatusCards(summary, locale = "ko") {
  const statuses = asArray(summary.workStatus);
  return `
    <section class="dashboard-status-grid" data-dashboard-section="work-status" aria-label="업무현황">
      ${statuses.map((item) => `
        <button class="dashboard-status-card" type="button" data-work-status="${escapeHtml(item.status)}" style="--status-color: ${escapeHtml(item.color || "#3B82F6")}">
          <span class="status-accent" aria-hidden="true"></span>
          <span class="status-title">${escapeHtml(item.label || statusLabel(item.status, locale))}</span>
          <strong>${money.format(item.yearCount || 0)}</strong>
          <span class="status-meta">고객사 ${money.format(item.customerCount || 0)}개</span>
          ${Number(item.urgentCount || 0) > 0 ? `<span class="status-urgent">즉시 처리 필요 ${money.format(item.urgentCount)}건</span>` : `<span class="status-quiet">마감 안정</span>`}
        </button>`).join("")}
    </section>`;
}

function renderDashboardNotificationPanel(notificationSummary, queue, locale = "ko", showApprovals = true) {
  const notifications = asArray(notificationSummary?.notifications).slice(0, 10);
  const unreadCount = Number(notificationSummary?.unreadCount || 0);
  const approvalRows = asArray(queue);
  return `
    <article class="panel dashboard-notification-panel" data-dashboard-section="notifications">
      <div class="panel-head">
        <div>
          <h2>알림 / 결재 대기함</h2>
          <p class="empty">마감 알림과 결재 대기 업무를 한 곳에서 처리합니다.</p>
        </div>
        <button id="dashAlerts" class="secondary-btn compact" type="button">알림 센터</button>
      </div>
      <div class="dashboard-tabs" role="tablist" aria-label="대시보드 알림 탭">
        <button class="tab active" type="button" role="tab" aria-selected="true" data-dashboard-tab="notifications">
          알림 <span class="dashboard-unread-badge" data-notification-unread-badge>${money.format(unreadCount)}</span>
        </button>
        ${showApprovals ? `<button class="tab" type="button" role="tab" aria-selected="false" data-dashboard-tab="approvals">
          결재 대기 <span class="dashboard-unread-badge quiet">${money.format(approvalRows.length)}</span>
        </button>` : ""}
      </div>
      <div class="dashboard-tab-panel" data-dashboard-tab-panel="notifications">
        ${renderDashboardNotificationList(notifications, locale)}
      </div>
      ${showApprovals ? `<div class="dashboard-tab-panel hidden" data-dashboard-tab-panel="approvals">
        <div class="panel-head compact-head"><h3>내 결재함</h3><button id="dashApproval" class="secondary-btn compact" type="button">열기</button></div>
        ${renderDashboardApprovalQueue(approvalRows, locale)}
      </div>` : ""}
    </article>`;
}

function renderDashboardApprovalQueue(queue, locale = "ko") {
  if (!queue.length) {
    return `<p class="empty dashboard-empty">내 결재 대기 항목이 없습니다.</p>`;
  }
  return `
    <ul class="dashboard-approval-list">
      ${queue.map((item) => `
        <li class="dashboard-approval-item" data-approval-by="${escapeHtml(item.by_id)}" tabindex="0">
          <div class="approval-target">
            <span class="badge warn">결재 대기</span>
            <strong>${escapeHtml(item.customer_name)} · ${escapeHtml(item.year_label)}</strong>
            <span class="muted">사업연도 ${escapeHtml(item.start_date || "-")} ~ ${escapeHtml(item.end_date || "-")} · ${escapeHtml(statusLabel(item.status, locale))}</span>
          </div>
          <div class="approval-meta">
            <span>요청자 <strong>${escapeHtml(item.requester_login_id || "-")}</strong></span>
            <span>대기일 <strong>${money.format(item.pending_days || 0)}일</strong></span>
          </div>
          <div class="approval-inline-actions">
            <button class="primary-btn compact" type="button" data-approve-approval="${escapeHtml(item.by_id)}">승인</button>
            <button class="danger-btn compact" type="button" data-reject-approval="${escapeHtml(item.by_id)}">반려</button>
            <button class="secondary-btn compact" type="button" data-open-approval="${escapeHtml(item.by_id)}">상세</button>
          </div>
        </li>`).join("")}
    </ul>`;
}

function renderDashboardNotificationList(notifications, locale = "ko") {
  if (!notifications.length) {
    return `<p class="empty dashboard-empty">최근 알림이 없습니다.</p>`;
  }
  return `
    <ul class="dashboard-notification-list">
      ${notifications.map((item) => {
        const unread = item.status === "UNREAD";
        const bucket = item.dueBucket ? `<span class="badge info">${escapeHtml(item.dueBucket)}</span>` : "";
        return `
          <li class="dashboard-notification-item ${unread ? "unread" : "read"}" data-notification-id="${escapeHtml(item.notificationId)}">
            <span class="notification-dot" aria-hidden="true"></span>
            <div class="notification-copy">
              <div class="notification-title-line">
                <span class="badge ${notificationSeverityClass(item.severity)}">${escapeHtml(item.severity)}</span>
                ${bucket}
                <strong>${escapeHtml(item.title)}</strong>
              </div>
              <p>${escapeHtml(item.message)}</p>
              <span class="muted">${escapeHtml(item.customerName || "공통")} · ${escapeHtml(formatNotificationTime(item.createdAt))} · ${escapeHtml(item.notificationType || "GENERAL")}</span>
            </div>
            <div class="notification-actions">
              <button class="secondary-btn compact" type="button" data-open-notification="${escapeHtml(item.notificationId)}">이동</button>
              <button class="secondary-btn compact" type="button" data-read-notification="${escapeHtml(item.notificationId)}" ${unread ? "" : "disabled"}>읽음</button>
            </div>
          </li>`;
      }).join("")}
    </ul>`;
}

function renderDashboardRecentActivities(activitySummary, locale = "ko") {
  const activities = asArray(activitySummary?.activities).slice(0, 15);
  return `
    <article class="panel dashboard-activity-panel" data-dashboard-section="recent-activities">
      <div class="panel-head">
        <div>
          <h2>최근활동</h2>
          <p class="empty">감사 로그를 업무 피드로 요약해 최근 변경된 화면으로 바로 이동합니다.</p>
        </div>
        <button id="dashAudit" class="secondary-btn compact" type="button">감사 로그 전체</button>
      </div>
      ${activities.length ? `
        <ul class="dashboard-activity-list">
          ${activities.map((item) => {
            const target = [item.customerName || "공통", item.fiscalYear || ""].filter(Boolean).join(" / ");
            return `
              <li class="dashboard-activity-item" data-activity-audit="${escapeHtml(item.auditId)}" tabindex="0">
                <time>${escapeHtml(formatNotificationTime(item.occurredAt))}</time>
                <div class="activity-copy">
                  <div class="activity-title-line">
                    <span class="badge info">${escapeHtml(item.typeLabel || item.activityType || "업무 변경")}</span>
                    <strong>${escapeHtml(item.description || item.activityType || item.action)}</strong>
                  </div>
                  <p>${escapeHtml(target || item.tableName || "-")}</p>
                  <span class="muted">${escapeHtml(item.actorName || item.actorLoginId || "system")} · ${escapeHtml(item.routeKey || "admin/audit:events")}</span>
                </div>
                <button class="secondary-btn compact" type="button" data-open-activity="${escapeHtml(item.auditId)}">이동</button>
              </li>`;
          }).join("")}
        </ul>` : `<p class="empty dashboard-empty">최근활동이 없습니다.</p>`}
    </article>`;
}

function kpiDonutGradient(industries) {
  const colors = ["#0ea5e9", "#22c55e", "#f59e0b", "#ef4444", "#64748b"];
  let start = 0;
  const segments = asArray(industries).slice(0, 5).map((item, index) => {
    const pct = Math.max(0, Number(item.percentageBps || 0) / 100);
    const end = Math.min(100, start + pct);
    const segment = `${colors[index % colors.length]} ${start}% ${end}%`;
    start = end;
    return segment;
  });
  if (start < 100) segments.push(`#e2e8f0 ${start}% 100%`);
  return `conic-gradient(${segments.join(", ")})`;
}

function renderKpiIndustryDistribution(industrySummary) {
  const industries = asArray(industrySummary?.industries);
  return `
    <section class="kpi-subpanel" data-dashboard-section="kpi-industry-distribution">
      <div class="kpi-subpanel-head">
        <h3>업종별 법인 분포</h3>
        <span>${money.format(industrySummary?.totalCustomers || 0)}개 법인</span>
      </div>
      ${industries.length ? `
        <div class="kpi-donut-layout">
          <div class="kpi-donut" style="background:${escapeHtml(kpiDonutGradient(industries))}" aria-label="업종별 법인 분포"></div>
          <ul class="kpi-distribution-list">
            ${industries.slice(0, 5).map((item) => `
              <li class="kpi-distribution-row" data-kpi-industry="${escapeHtml(item.industryCode)}">
                <span>${escapeHtml(item.industryName || item.industryCode)}</span>
                <strong>${Number(item.percentagePct || 0).toFixed(1)}%</strong>
                <em>${money.format(item.customerCount || 0)}개</em>
              </li>`).join("")}
          </ul>
        </div>` : `<p class="empty dashboard-empty">업종별 법인 데이터가 없습니다.</p>`}
    </section>`;
}

function renderKpiLossExpiry(lossSummary) {
  const buckets = asArray(lossSummary?.buckets);
  const maxAmount = Math.max(1, ...buckets.map((item) => Number(item.totalAmount || 0)));
  return `
    <section class="kpi-subpanel" data-dashboard-section="kpi-loss-expiry">
      <div class="kpi-subpanel-head">
        <h3>이월결손금 만료 예측</h3>
        <span>${money.format(lossSummary?.totalCustomerCount || 0)}개 법인</span>
      </div>
      ${buckets.length ? `
        <div class="kpi-loss-table">
          ${buckets.map((item) => `
            <div class="kpi-loss-row" data-kpi-loss-year="${escapeHtml(item.expiresYear)}">
              <span>${escapeHtml(item.expiresYear)}년</span>
              <div class="bar-track"><span style="width:${Math.max(4, Math.round(Number(item.totalAmount || 0) / maxAmount * 100))}%"></span></div>
              <strong>${money.format(item.totalAmount || 0)}</strong>
              <em>${money.format(item.customerCount || 0)}개 / ${money.format(item.lossCount || 0)}건</em>
            </div>`).join("")}
        </div>
        <p class="kpi-caption">향후 ${escapeHtml(lossSummary?.years || 3)}개년 만료 예정 잔액 ${money.format(lossSummary?.totalAmount || 0)}</p>`
        : `<p class="empty dashboard-empty">만료 예정 이월결손금이 없습니다.</p>`}
    </section>`;
}

function renderDashboardTaxBurdenKpi(kpiSummary, industrySummary, lossSummary, locale = "ko") {
  const trend = asArray(kpiSummary?.trend).slice(-5);
  const maxRate = Math.max(1, ...trend.map((item) => Number(item.effectiveTaxRateBps || 0)));
  const latest = trend[trend.length - 1];
  const averagePct = Number(kpiSummary?.averageEffectiveTaxRatePct || 0);
  return `
    <article class="panel dashboard-kpi-panel" data-dashboard-section="kpi-tax-burden">
      <div class="panel-head">
        <div>
          <h2>핵심지표</h2>
          <p class="empty">최근 ${escapeHtml(kpiSummary?.years || 5)}개년 당기 세부담 추이</p>
        </div>
        <button id="dashKpiTax" class="secondary-btn compact" type="button">세부담 분석</button>
      </div>
      <div class="kpi-summary-strip">
        <span>평균 실효세율 <strong>${averagePct.toFixed(2)}%</strong></span>
        <span>총 부담세액 <strong>${money.format(kpiSummary?.totalTaxDue || 0)}</strong></span>
      </div>
      ${trend.length ? `
        <div class="dashboard-kpi-chart" aria-label="당기 세부담 추이">
          ${trend.map((item) => {
            const rate = Number(item.effectiveTaxRateBps || 0);
            return `
              <div class="kpi-trend-row" data-kpi-year="${escapeHtml(item.fiscalYear)}">
                <span>${escapeHtml(item.fiscalYear)}</span>
                <div class="bar-track"><span style="width:${Math.max(4, Math.round(rate / maxRate * 100))}%"></span></div>
                <strong>${(rate / 100).toFixed(2)}%</strong>
              </div>`;
          }).join("")}
        </div>
        <p class="kpi-caption">최신 ${escapeHtml(latest?.fiscalYear || "-")}년: 과세표준 ${money.format(latest?.taxableIncome || 0)}, 부담세액 ${money.format(latest?.totalTaxDue || 0)}, 담당 법인 ${money.format(latest?.customerCount || 0)}개</p>`
        : `<p class="empty dashboard-empty">세부담 추이 데이터가 없습니다.</p>`}
      <section class="dashboard-kpi-secondary">
        ${renderKpiIndustryDistribution(industrySummary)}
        ${renderKpiLossExpiry(lossSummary)}
      </section>
    </article>`;
}

function dashboardRoles(auth) {
  return asArray(auth?.user?.roles).map((role) => String(role).toUpperCase());
}

function canViewDashboardApprovals(auth) {
  return dashboardRoles(auth).includes("TAX_REVIEWER");
}

function canViewDashboardKpi(auth) {
  return dashboardRoles(auth).some((role) => ["SUPER_ADMIN", "TENANT_ADMIN", "SYSTEM_ADMIN", "TAX_EXPERT", "TAX_REVIEWER"].includes(role));
}

function invalidateDashboardCache(reason = "manual") {
  dashboardCacheVersion += 1;
  return { version: dashboardCacheVersion, reason };
}

function stopDashboardRealtime() {
  if (!dashboardRealtime) return;
  clearInterval(dashboardRealtime.pollTimer);
  dashboardRealtime = null;
}

function startDashboardRealtime(env, root) {
  stopDashboardRealtime();
  const realtime = { root, pollTimer: null, refreshing: false };
  realtime.pollTimer = setInterval(async () => {
    if (!dashboardRealtime || dashboardRealtime.root !== root || realtime.refreshing) return;
    realtime.refreshing = true;
    try {
      invalidateDashboardCache("poll");
      await renderDashboard(env);
    } finally {
      if (dashboardRealtime === realtime) {
        realtime.refreshing = false;
      }
    }
  }, DASHBOARD_REFRESH_INTERVAL_MS);
  dashboardRealtime = realtime;
}

function bindDashboardTabs(outlet) {
  outlet.querySelectorAll("[data-dashboard-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      outlet.querySelectorAll("[data-dashboard-tab]").forEach((tab) => {
        const active = tab === button;
        tab.classList.toggle("active", active);
        tab.setAttribute("aria-selected", active ? "true" : "false");
      });
      outlet.querySelectorAll("[data-dashboard-tab-panel]").forEach((panel) => {
        panel.classList.toggle("hidden", panel.dataset.dashboardTabPanel !== button.dataset.dashboardTab);
      });
    });
  });
}

function bindDashboardDeadlineRows(env, deadlineRows) {
  env.outlet.querySelectorAll("[data-deadline-by]").forEach((rowElement) => {
    const openDeadline = async () => {
      const item = deadlineRows.find((candidate) => String(candidate.businessYearId) === rowElement.dataset.deadlineBy);
      if (!item) return;
      await refreshContextFromBy(env, {
        by_id: item.businessYearId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.filingDueDate,
        status: item.status,
      }, { customer_name: item.customerName });
      env.navigate(item.routeKey || "ws/start:snapshot");
    };
    rowElement.addEventListener("click", openDeadline);
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openDeadline();
      }
    });
  });
}

function bindDashboardInboxActions(env, root, notificationSummary, queue, rerender) {
  const dashboardNotifications = asArray(notificationSummary?.notifications);
  const approvalQueue = asArray(queue);
  bindDashboardTabs(env.outlet);
  env.outlet.querySelectorAll("[data-read-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/notifications/${button.dataset.readNotification}`, {
        method: "PATCH",
        body: JSON.stringify({ status: "READ" }),
      });
      invalidateDashboardCache("notification-read");
      await rerender();
    });
  });
  env.outlet.querySelectorAll("[data-open-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      const item = dashboardNotifications.find((candidate) => String(candidate.notificationId) === String(button.dataset.openNotification));
      if (!item) return;
      if (item.byId) {
        await refreshContextFromBy(env, {
          by_id: item.byId,
          customer_id: item.customerId,
          year_label: item.fiscalYear,
          start_date: item.startDate,
          end_date: item.filingDueDate,
          status: item.businessYearStatus || "DRAFT",
        }, { customer_name: item.customerName });
      }
      env.navigate(item.routeKey || "dashboard:inbox");
    });
  });
  const openApproval = async (byId) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    await refreshContextFromBy(env, {
      by_id: item.by_id,
      customer_id: item.customer_id,
      year_label: item.year_label,
      start_date: item.start_date,
      end_date: item.end_date,
      status: item.status,
    }, { customer_name: item.customer_name });
    env.navigate(item.route_key || "ws/appr:inbox");
  };
  const runApprovalAction = async (byId, status) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    const approved = status === "APPROVED";
    await request(`${root}/business-years/${encodeURIComponent(item.by_id)}/status`, {
      method: "POST",
      body: JSON.stringify({
        status,
        actor: env.auth?.user?.login_id || "dashboard",
        approver: env.auth?.user?.login_id || item.approver_login_id || "dashboard",
        comment: approved ? "dashboard inline approval" : "dashboard inline rejection",
      }),
    });
    invalidateDashboardCache("approval-action");
    await rerender();
  };
  env.outlet.querySelectorAll("[data-open-approval]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openApproval(button.dataset.openApproval);
    });
  });
  env.outlet.querySelectorAll("[data-approve-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.approveApproval, "APPROVED");
    });
  });
  env.outlet.querySelectorAll("[data-reject-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.rejectApproval, "DRAFT");
    });
  });
  env.outlet.querySelectorAll("[data-approval-by]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openApproval(rowElement.dataset.approvalBy));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openApproval(rowElement.dataset.approvalBy);
      }
    });
  });
  env.outlet.querySelector("#dashApproval")?.addEventListener("click", () => env.navigate("ws/appr:inbox"));
  env.outlet.querySelector("#dashAlerts")?.addEventListener("click", () => env.navigate("dashboard:inbox"));
}

function bindDashboardRecentActions(env, recentActivities) {
  const openActivity = async (auditId) => {
    const item = recentActivities.find((candidate) => String(candidate.auditId) === String(auditId));
    if (!item) return;
    if (item.byId) {
      await refreshContextFromBy(env, {
        by_id: item.byId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.endDate,
        status: item.businessYearStatus || "DRAFT",
      }, { customer_name: item.customerName });
    }
    env.navigate(item.routeKey || "admin/audit:events");
  };
  env.outlet.querySelectorAll("[data-activity-audit]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openActivity(rowElement.dataset.activityAudit));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openActivity(rowElement.dataset.activityAudit);
      }
    });
  });
  env.outlet.querySelectorAll("[data-open-activity]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openActivity(button.dataset.openActivity);
    });
  });
  env.outlet.querySelector("#dashAudit")?.addEventListener("click", () => env.navigate("admin/audit:events"));
}

async function renderDashboardDueSoon(env) {
  const root = routeRoot(env);
  const deadlines = await request(`${root}/dashboard/filing-deadlines?withinDays=30`);
  const deadlineRows = asArray(deadlines.deadlines || deadlines).slice(0, 50);
  env.outlet.innerHTML = `
    <section class="dashboard-home dashboard-detail" data-dashboard="duesoon">
      <section class="panel dashboard-deadline-panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Dashboard</span>
            <h2>신고마감 임박</h2>
            <p class="empty">D-30, D-7, D-Day 신고 작업을 우선순위와 상태별로 확인합니다.</p>
          </div>
          <button class="secondary-btn compact" type="button" data-dashboard-overview>대시보드</button>
        </div>
        ${renderDashboardDeadlineTable(deadlineRows, env.locale)}
      </section>
    </section>`;
  bindDashboardDeadlineRows(env, deadlineRows);
  env.outlet.querySelector("[data-dashboard-overview]")?.addEventListener("click", () => env.navigate("dashboard:overview"));
}

async function renderDashboardInbox(env) {
  const root = routeRoot(env);
  const showApprovals = canViewDashboardApprovals(env.auth);
  const [notificationSummary, queue] = await Promise.all([
    request(`${root}/dashboard/notifications?limit=50`),
    showApprovals ? request(`${root}/workflow/queue?assignee=me`) : Promise.resolve([]),
  ]);
  env.outlet.innerHTML = `
    <section class="dashboard-home dashboard-detail" data-dashboard="inbox">
      ${renderDashboardNotificationPanel(notificationSummary, queue, env.locale, showApprovals)}
    </section>`;
  bindDashboardInboxActions(env, root, notificationSummary, queue, () => renderDashboardInbox(env));
}

async function renderDashboardRecent(env) {
  const root = routeRoot(env);
  const recentSummary = await request(`${root}/dashboard/recent-activities?limit=50`);
  const recentActivities = asArray(recentSummary.activities);
  env.outlet.innerHTML = `
    <section class="dashboard-home dashboard-detail" data-dashboard="recent">
      ${renderDashboardRecentActivities(recentSummary, env.locale)}
    </section>`;
  bindDashboardRecentActions(env, recentActivities);
}

async function renderDashboardKpiTax(env) {
  if (!canViewDashboardKpi(env.auth)) {
    env.outlet.innerHTML = `
      <section class="panel empty-state" data-dashboard="kpi-tax">
        <strong>세부담 KPI 권한이 없습니다.</strong>
        <p class="empty">세부담 분석은 관리자, 세무전문가, 검토자 역할에서 조회할 수 있습니다.</p>
      </section>`;
    return;
  }
  const root = routeRoot(env);
  const [kpiTaxBurden, kpiIndustryDistribution, kpiLossExpiry] = await Promise.all([
    request(`${root}/dashboard/kpi/tax-burden?years=5`),
    request(`${root}/dashboard/kpi/industry-distribution`),
    request(`${root}/dashboard/kpi/loss-expiry?years=3`),
  ]);
  env.outlet.innerHTML = `
    <section class="dashboard-home dashboard-detail" data-dashboard="kpi-tax">
      ${renderDashboardTaxBurdenKpi(kpiTaxBurden, kpiIndustryDistribution, kpiLossExpiry, env.locale)}
    </section>`;
  env.outlet.querySelector("#dashKpiTax")?.addEventListener("click", () => env.navigate("report:tax-burden"));
}

async function renderDashboard(env) {
  const root = routeRoot(env);
  const showApprovals = canViewDashboardApprovals(env.auth);
  const showKpi = canViewDashboardKpi(env.auth);
  const [
    summary,
    deadlines,
    notificationSummary,
    queue,
    recentSummary,
    kpiTaxBurden,
    kpiIndustryDistribution,
    kpiLossExpiry,
  ] = await Promise.all([
    request(`${root}/dashboard`),
    request(`${root}/dashboard/filing-deadlines?withinDays=30`),
    request(`${root}/dashboard/notifications?limit=10`),
    showApprovals ? request(`${root}/workflow/queue?assignee=me`) : Promise.resolve([]),
    request(`${root}/dashboard/recent-activities?limit=15`),
    showKpi ? request(`${root}/dashboard/kpi/tax-burden?years=5`) : Promise.resolve({ trend: [] }),
    showKpi ? request(`${root}/dashboard/kpi/industry-distribution`) : Promise.resolve({ industries: [] }),
    showKpi ? request(`${root}/dashboard/kpi/loss-expiry?years=3`) : Promise.resolve({ buckets: [] }),
  ]);
  const deadlineRows = asArray(deadlines.deadlines || summary.filingDeadlines?.deadlines).slice(0, 10);
  const dashboardNotifications = asArray(notificationSummary.notifications);
  const approvalQueue = asArray(queue);
  const recentActivities = asArray(recentSummary.activities);
  env.outlet.innerHTML = `
    <section class="dashboard-home" data-dashboard="overview" data-dashboard-cache-version="${dashboardCacheVersion}">
      <section class="panel dashboard-hero" data-dashboard-section="start">
        <div>
          <span class="badge info">Dashboard</span>
          <h2>신고 업무 현황</h2>
          <p class="empty">작성, 검증, 결재, 승인, 신고 완료 상태를 확인하고 다음 작업으로 바로 이동합니다.</p>
        </div>
        <button id="dashStartWork" class="primary-btn" type="button">신고 작업 시작</button>
      </section>
      ${Number(summary.rejectedCount || 0) > 0 ? `<section class="dashboard-rejected-banner" data-dashboard-section="rejected">반려 ${money.format(summary.rejectedCount)}건 - 재작성 필요</section>` : ""}
      ${renderDashboardWorkStatusCards(summary, env.locale)}
      <section class="dashboard-main-grid">
        <article class="panel dashboard-deadline-panel">
          <div class="panel-head"><h2>신고마감 임박</h2><button id="dashDueSoonAll" class="secondary-btn compact" type="button">전체 보기</button></div>
          ${renderDashboardDeadlineTable(deadlineRows, env.locale)}
        </article>
        ${renderDashboardNotificationPanel(notificationSummary, queue, env.locale, showApprovals)}
      </section>
      <section class="dashboard-lower-grid">
        ${renderDashboardRecentActivities(recentSummary, env.locale)}
        ${showKpi ? renderDashboardTaxBurdenKpi(kpiTaxBurden, kpiIndustryDistribution, kpiLossExpiry, env.locale) : ""}
      </section>
    </section>`;
  document.getElementById("dashStartWork").addEventListener("click", () => env.navigate("ws/start:customer-pick"));
  document.querySelectorAll("[data-work-status]").forEach((card) => {
    card.addEventListener("click", () => env.navigate(dashboardStatusRoute(card.dataset.workStatus)));
  });
  document.getElementById("dashDueSoonAll").addEventListener("click", () => env.navigate("dashboard:duesoon"));
  document.querySelectorAll("[data-dashboard-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll("[data-dashboard-tab]").forEach((tab) => {
        const active = tab === button;
        tab.classList.toggle("active", active);
        tab.setAttribute("aria-selected", active ? "true" : "false");
      });
      document.querySelectorAll("[data-dashboard-tab-panel]").forEach((panel) => {
        panel.classList.toggle("hidden", panel.dataset.dashboardTabPanel !== button.dataset.dashboardTab);
      });
    });
  });
  document.querySelectorAll("[data-read-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/notifications/${button.dataset.readNotification}`, {
        method: "PATCH",
        body: JSON.stringify({ status: "READ" }),
      });
      invalidateDashboardCache("notification-read");
      await renderDashboard(env);
    });
  });
  document.querySelectorAll("[data-open-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      const item = dashboardNotifications.find((candidate) => String(candidate.notificationId) === String(button.dataset.openNotification));
      if (!item) return;
      if (item.byId) {
        await refreshContextFromBy(env, {
          by_id: item.byId,
          customer_id: item.customerId,
          year_label: item.fiscalYear,
          start_date: item.startDate,
          end_date: item.filingDueDate,
          status: item.businessYearStatus || "DRAFT",
        }, { customer_name: item.customerName });
      }
      env.navigate(item.routeKey || "dashboard:inbox");
    });
  });
  const openApproval = async (byId) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    await refreshContextFromBy(env, {
      by_id: item.by_id,
      customer_id: item.customer_id,
      year_label: item.year_label,
      start_date: item.start_date,
      end_date: item.end_date,
      status: item.status,
    }, { customer_name: item.customer_name });
    env.navigate(item.route_key || "ws/appr:inbox");
  };
  const runApprovalAction = async (byId, status) => {
    const item = approvalQueue.find((candidate) => String(candidate.by_id) === String(byId));
    if (!item) return;
    const approved = status === "APPROVED";
    await request(`${root}/business-years/${encodeURIComponent(item.by_id)}/status`, {
      method: "POST",
      body: JSON.stringify({
        status,
        actor: env.auth?.user?.login_id || "dashboard",
        approver: env.auth?.user?.login_id || item.approver_login_id || "dashboard",
        comment: approved ? "dashboard inline approval" : "dashboard inline rejection",
      }),
    });
    invalidateDashboardCache("approval-action");
    await renderDashboard(env);
  };
  document.querySelectorAll("[data-open-approval]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openApproval(button.dataset.openApproval);
    });
  });
  document.querySelectorAll("[data-approve-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.approveApproval, "APPROVED");
    });
  });
  document.querySelectorAll("[data-reject-approval]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      await runApprovalAction(button.dataset.rejectApproval, "DRAFT");
    });
  });
  document.querySelectorAll("[data-approval-by]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openApproval(rowElement.dataset.approvalBy));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openApproval(rowElement.dataset.approvalBy);
      }
    });
  });
  document.querySelectorAll("[data-deadline-by]").forEach((rowElement) => {
    const openDeadline = async () => {
      const item = deadlineRows.find((candidate) => String(candidate.businessYearId) === rowElement.dataset.deadlineBy);
      if (!item) return;
      await refreshContextFromBy(env, {
        by_id: item.businessYearId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.filingDueDate,
        status: item.status,
      }, { customer_name: item.customerName });
      env.navigate(item.routeKey || "ws/start:snapshot");
    };
    rowElement.addEventListener("click", openDeadline);
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openDeadline();
      }
    });
  });
  const openActivity = async (auditId) => {
    const item = recentActivities.find((candidate) => String(candidate.auditId) === String(auditId));
    if (!item) return;
    if (item.byId) {
      await refreshContextFromBy(env, {
        by_id: item.byId,
        customer_id: item.customerId,
        year_label: item.fiscalYear,
        start_date: item.startDate,
        end_date: item.endDate,
        status: item.businessYearStatus || "DRAFT",
      }, { customer_name: item.customerName });
    }
    env.navigate(item.routeKey || "admin/audit:events");
  };
  document.querySelectorAll("[data-activity-audit]").forEach((rowElement) => {
    rowElement.addEventListener("click", () => openActivity(rowElement.dataset.activityAudit));
    rowElement.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openActivity(rowElement.dataset.activityAudit);
      }
    });
  });
  document.querySelectorAll("[data-open-activity]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      openActivity(button.dataset.openActivity);
    });
  });
  document.getElementById("dashKpiTax")?.addEventListener("click", () => env.navigate("report:tax-burden"));
  document.getElementById("dashApproval")?.addEventListener("click", () => env.navigate("ws/appr:inbox"));
  document.getElementById("dashAlerts")?.addEventListener("click", () => env.navigate("dashboard:inbox"));
  document.getElementById("dashAudit")?.addEventListener("click", () => env.navigate("admin/audit:events"));
  startDashboardRealtime(env, root);
}

async function renderWorkStart(env) {
  return renderWorkStartCustomerPick(env);
}

async function loadWorkStartData(env) {
  const root = routeRoot(env);
  const [customers, years] = await Promise.all([
    request(`${root}/customers`),
    request(`${root}/business-years`),
  ]);
  const yearsByCustomer = new Map();
  years.forEach((year) => {
    const list = yearsByCustomer.get(year.customer_id) || [];
    list.push(year);
    yearsByCustomer.set(year.customer_id, list);
  });
  return { root, customers, years, yearsByCustomer };
}

function renderWorkStartHeader(env, activeLeaf, title, description) {
  return `
    <section class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">${escapeHtml(t(env.locale, "workStart.title"))}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderStageRouteButtons(activeLeaf, ["ws/start:customer-pick", "ws/start:by-pick", "ws/start:snapshot"], env.locale)}</div>
      </div>
    </section>`;
}

async function renderWorkStartCustomerPick(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { root, customers, years } = await loadWorkStartData(env);
  const locale = env.locale;
  const recentYears = [...years]
    .sort((a, b) => String(b.updated_at || "").localeCompare(String(a.updated_at || "")))
    .slice(0, 5);
  env.outlet.innerHTML = `
    <section class="leaf-workbench work-start-workbench" data-stage="work-start" data-work-start-stage="customer-pick" data-leaf-key="ws/start:customer-pick">
      ${renderWorkStartHeader(env, activeLeaf, t(locale, "route.ws.start.customerPick"), t(locale, "context.pickCustomerYear"))}
      ${metrics([
        [t(locale, "field.customerName"), money.format(customers.length)],
        [t(locale, "field.yearLabel"), money.format(years.length)],
        [t(locale, "workStart.recent"), money.format(recentYears.length)],
        [t(locale, "workStart.snapshotPreview"), env.context?.snapshot ? t(locale, "status.ready") : t(locale, "status.pending")],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(t(locale, "route.ws.start.customerPick"))}</h2><p>${escapeHtml(t(locale, "workStart.selectWork"))}</p></div>
            <label class="inline-control">${escapeHtml(t(locale, "workStart.customerSearch"))} <input id="workStartSearch" type="search" placeholder="${escapeHtml(t(locale, "field.customerName"))}" /></label>
          </div>
          ${table([t(locale, "field.customerCode"), t(locale, "field.customerName"), t(locale, "field.bizRegNo"), "SME", ""], customers.map((customer) => row([
            escapeHtml(customer.customer_code),
            escapeHtml(customer.customer_name),
            escapeHtml(customer.biz_reg_no),
            customer.is_sme ? "Y" : "N",
            `<button class="primary-btn compact" type="button" data-select-customer="${escapeHtml(customer.customer_id)}" data-leaf-row-action="select-customer">${escapeHtml(t(locale, "route.ws.start.customerPick"))}</button>`,
          ])), "No customers found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.newCustomer"))}</h2><p>${escapeHtml(t(locale, "workStart.customerSearch"))}</p></div>
          <form id="customerForm" class="stack">
            <label>${escapeHtml(t(locale, "field.customerCode"))} <input id="newCustomerCode" value="cust${Date.now().toString(36).slice(-4)}" /></label>
            <label>${escapeHtml(t(locale, "field.customerName"))} <input id="newCustomerName" value="${escapeHtml(t(locale, "workStart.newCustomer"))}" /></label>
            <label>${escapeHtml(t(locale, "field.bizRegNo"))} <input id="newCustomerBiz" value="1234567890" /></label>
            <label>SME <input id="newCustomerSme" type="checkbox" checked /></label>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.create"))}</button>
          </form>
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.recent"))}</h2><p>${escapeHtml(t(locale, "workStart.selectWork"))}</p></div>
        ${table([t(locale, "field.customerName"), t(locale, "field.yearLabel"), t(locale, "field.status"), t(locale, "field.progress"), ""], recentYears.map((by) => {
          const customer = customers.find((item) => item.customer_id === by.customer_id);
          return row([
            escapeHtml(customer?.customer_name || by.customer_id),
            escapeHtml(by.year_label),
            pill(by.status, locale),
            `<div class="bar-track"><span style="width:${progressForStatus(by.status)}%"></span></div>`,
            `<button class="secondary-btn compact" type="button" data-select-by="${escapeHtml(by.by_id)}">${escapeHtml(t(locale, "common.continue"))}</button>`,
          ]);
        }))}
      </article>
    </section>`;

  bindWorkStartRouteButtons(env);
  env.outlet.querySelector("#workStartSearch")?.addEventListener("input", (event) => {
    const query = event.target.value.trim().toLowerCase();
    env.outlet.querySelectorAll("[data-select-customer]").forEach((button) => {
      const tr = button.closest("tr");
      if (!tr) return;
      tr.style.display = !query || tr.textContent.toLowerCase().includes(query) ? "" : "none";
    });
  });
  env.outlet.querySelectorAll("[data-select-customer]").forEach((button) => {
    button.addEventListener("click", () => {
      const customer = customers.find((item) => String(item.customer_id) === String(button.dataset.selectCustomer));
      env.setContext({ customerId: customer?.customer_id, customerName: customer?.customer_name });
      env.navigate("ws/start:by-pick", { customerId: button.dataset.selectCustomer });
    });
  });
  env.outlet.querySelectorAll("[data-select-by]").forEach((button) => {
    button.addEventListener("click", async () => {
      const by = years.find((item) => String(item.by_id) === button.dataset.selectBy);
      const customer = customers.find((item) => item.customer_id === by.customer_id);
      await refreshContextFromBy(env, by, customer);
      env.navigate("ws/start:snapshot", { byId: by.by_id, customerId: by.customer_id });
    });
  });
  env.outlet.querySelector("#customerForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/customers`, {
      method: "POST",
      body: JSON.stringify({
        customer_code: env.outlet.querySelector("#newCustomerCode").value.trim(),
        customer_name: env.outlet.querySelector("#newCustomerName").value.trim(),
        biz_reg_no: env.outlet.querySelector("#newCustomerBiz").value.trim(),
        is_sme: env.outlet.querySelector("#newCustomerSme").checked,
        work_scopes: ["INFO", "ADJUST", "FORM", "VALIDATE", "APPROVE", "PRINT", "EFILE", "POST"],
      }),
    });
    await renderWorkStartCustomerPick(env);
  });
}

async function renderWorkStartBusinessYearPick(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { root, customers, years, yearsByCustomer } = await loadWorkStartData(env);
  const locale = env.locale;
  const currentYear = new Date().getFullYear();
  const selectedCustomerId = Number(env.context?.customerId || new URLSearchParams(location.hash.split("?")[1] || "").get("customerId") || customers[0]?.customer_id || 0);
  const customerOptions = customers
    .map((customer) => `<option value="${escapeHtml(customer.customer_id)}" ${customer.customer_id === selectedCustomerId ? "selected" : ""}>${escapeHtml(customer.customer_name)} (${escapeHtml(customer.customer_code)})</option>`)
    .join("");
  const visibleYears = selectedCustomerId ? years.filter((year) => Number(year.customer_id) === selectedCustomerId) : years;
  env.outlet.innerHTML = `
    <section class="leaf-workbench work-start-workbench" data-stage="work-start" data-work-start-stage="business-year-pick" data-leaf-key="ws/start:by-pick">
      ${renderWorkStartHeader(env, activeLeaf, t(locale, "route.ws.start.byPick"), t(locale, "workStart.selectWork"))}
      ${metrics([
        [t(locale, "field.customerName"), money.format(customers.length)],
        [t(locale, "field.yearLabel"), money.format(visibleYears.length)],
        [t(locale, "workStart.carryForward"), money.format(years.length)],
        [t(locale, "workStart.snapshotPreview"), env.context?.snapshot ? t(locale, "status.ready") : t(locale, "status.pending")],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(t(locale, "route.ws.start.byPick"))}</h2><p>${escapeHtml(t(locale, "workStart.selectWork"))}</p></div>
            <label class="inline-control">${escapeHtml(t(locale, "field.customerName"))} <select id="byFilterCustomer">${customerOptions}</select></label>
          </div>
          ${table([t(locale, "field.customerName"), t(locale, "field.yearLabel"), t(locale, "field.status"), t(locale, "field.progress"), ""], visibleYears.map((by) => {
            const customer = customers.find((item) => item.customer_id === by.customer_id);
            return row([
              escapeHtml(customer?.customer_name || by.customer_id),
              escapeHtml(by.year_label),
              pill(by.status, locale),
              `<div class="bar-track"><span style="width:${progressForStatus(by.status)}%"></span></div>`,
              `<button class="primary-btn compact" type="button" data-select-by="${escapeHtml(by.by_id)}" data-leaf-row-action="select-by">${escapeHtml(t(locale, "common.continue"))}</button>`,
            ]);
          }), "No business years found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.newBusinessYear"))}</h2><p>${escapeHtml(t(locale, "workStart.carryForwardHelp"))}</p></div>
          <form id="businessYearForm" class="stack">
            <label>${escapeHtml(t(locale, "field.customerName"))} <select id="byCustomer">${customerOptions}</select></label>
            <label>${escapeHtml(t(locale, "field.yearLabel"))} <input id="byYear" type="number" value="${currentYear}" /></label>
            <div class="form-grid">
              <label>${escapeHtml(t(locale, "workStart.startDate"))} <input id="byStart" type="date" value="${currentYear}-01-01" /></label>
              <label>${escapeHtml(t(locale, "workStart.endDate"))} <input id="byEnd" type="date" value="${currentYear}-12-31" /></label>
            </div>
            <label class="inline-control"><input id="byCarryForward" type="checkbox" checked /> ${escapeHtml(t(locale, "workStart.carryForward"))}</label>
            <label>${escapeHtml(t(locale, "workStart.carryForwardSource"))} <select id="byCarryForwardSource"></select></label>
            <p class="empty">${escapeHtml(t(locale, "workStart.carryForwardHelp"))}</p>
            <button class="primary-btn" type="submit" ${customers.length ? "" : "disabled"}>${escapeHtml(t(locale, "common.create"))}</button>
          </form>
        </article>
      </section>
    </section>`;

  bindWorkStartRouteButtons(env);
  env.outlet.querySelector("#byFilterCustomer")?.addEventListener("change", (event) => {
    const customer = customers.find((item) => String(item.customer_id) === String(event.target.value));
    env.setContext({ customerId: customer?.customer_id, customerName: customer?.customer_name });
    renderWorkStartBusinessYearPick(env);
  });
  env.outlet.querySelectorAll("[data-select-by]").forEach((button) => {
    button.addEventListener("click", async () => {
      const by = years.find((item) => String(item.by_id) === button.dataset.selectBy);
      const customer = customers.find((item) => item.customer_id === by.customer_id);
      await refreshContextFromBy(env, by, customer);
      env.navigate("ws/start:snapshot", { byId: by.by_id, customerId: by.customer_id });
    });
  });

  const byCustomerSelect = env.outlet.querySelector("#byCustomer");
  const byYearInput = env.outlet.querySelector("#byYear");
  const byCarryForward = env.outlet.querySelector("#byCarryForward");
  const byCarryForwardSource = env.outlet.querySelector("#byCarryForwardSource");
  const byStartInput = env.outlet.querySelector("#byStart");
  const byEndInput = env.outlet.querySelector("#byEnd");

  function syncCarryForwardOptions() {
    const customerId = Number(byCustomerSelect.value);
    const candidates = [...(yearsByCustomer.get(customerId) || [])]
      .sort((a, b) => b.year_label - a.year_label || b.by_id - a.by_id);
    byCarryForwardSource.innerHTML = candidates.length
      ? candidates.map((item) => `<option value="${escapeHtml(item.by_id)}">${escapeHtml(item.year_label)} (${escapeHtml(item.start_date)} ~ ${escapeHtml(item.end_date)})</option>`).join("")
      : `<option value="">${escapeHtml(t(locale, "context.select"))}</option>`;
    byCarryForwardSource.disabled = !byCarryForward.checked || !candidates.length;
  }

  function syncBusinessYearDates() {
    const nextYear = Number(byYearInput.value || currentYear);
    byStartInput.value = `${nextYear}-01-01`;
    byEndInput.value = `${nextYear}-12-31`;
  }

  byCustomerSelect?.addEventListener("change", syncCarryForwardOptions);
  byYearInput?.addEventListener("change", syncBusinessYearDates);
  byCarryForward?.addEventListener("change", () => {
    syncCarryForwardOptions();
    byCarryForwardSource.disabled = !byCarryForward.checked || !byCarryForwardSource.options.length;
  });
  syncCarryForwardOptions();

  env.outlet.querySelector("#businessYearForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${root}/business-years`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: Number(env.outlet.querySelector("#byCustomer").value),
        year_label: Number(env.outlet.querySelector("#byYear").value),
        start_date: env.outlet.querySelector("#byStart").value,
        end_date: env.outlet.querySelector("#byEnd").value,
        carry_forward_from_by_id: env.outlet.querySelector("#byCarryForward").checked && env.outlet.querySelector("#byCarryForwardSource").value
          ? Number(env.outlet.querySelector("#byCarryForwardSource").value)
          : null,
      }),
    });
    const customer = customers.find((item) => item.customer_id === by.customer_id);
    await refreshContextFromBy(env, by, customer);
    env.navigate("ws/start:snapshot", { byId: by.by_id, customerId: by.customer_id });
  });
}

async function renderWorkStartSnapshot(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const root = routeRoot(env);
  const locale = env.locale;
  const snapshot = env.context?.snapshot || await request(`${root}/business-years/${env.context.byId}/snapshot`).catch(() => null);
  env.outlet.innerHTML = `
    <section class="leaf-workbench work-start-workbench" data-stage="work-start" data-work-start-stage="snapshot" data-leaf-key="ws/start:snapshot">
      ${renderWorkStartHeader(env, activeLeaf, t(locale, "route.ws.start.snapshot"), t(locale, "workStart.snapshotPreview"))}
      ${metrics([
        [t(locale, "field.customerName"), env.context?.customerName || "-"],
        [t(locale, "field.yearLabel"), env.context?.fy || "-"],
        [t(locale, "field.status"), statusLabel(env.context?.status || "DRAFT", locale)],
        [t(locale, "workStart.snapshotPreview"), snapshot ? t(locale, "status.ready") : t(locale, "status.pending")],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "workStart.snapshotPreview"))}</h2><p>${escapeHtml(env.context?.period || "-")}</p></div>
          <div id="snapshotPreview" class="stack">${snapshot ? renderSnapshotSummary(snapshot, locale) : `<p class="empty">${escapeHtml(t(locale, "context.select"))}</p>`}</div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "common.nextStep"))}</h2><p>${escapeHtml(t(locale, "context.select"))}</p></div>
          <div class="button-row">
            <button class="secondary-btn" type="button" data-next-leaf="ws/start:by-pick">${escapeHtml(t(locale, "route.ws.start.byPick"))}</button>
            <button class="primary-btn" type="button" data-next-leaf="ws/info:fs">${escapeHtml(t(locale, "route.ws.info.fs"))}</button>
          </div>
        </article>
      </section>
    </section>`;
  bindWorkStartRouteButtons(env);
  env.outlet.querySelectorAll("[data-next-leaf]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.nextLeaf)));
}

function bindWorkStartRouteButtons(env) {
  env.outlet.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
}

const TAX_DATA_ROUTES = ["ws/info:fs", "ws/info:mapping", "ws/info:assets", "ws/info:transactions", "ws/info:vehicle", "ws/info:consistency"];

async function loadTaxDataWorkbenchData(env) {
  const root = workRoot(env);
  const [validation, fs, assets, transactions, vehicleLogs, batches, mappings, issues] = await Promise.all([
    request(`${root}/tax-data/validation`),
    request(`${root}/tax-data/financial-statements`),
    request(`${root}/tax-data/assets`),
    request(`${root}/tax-data/transactions`),
    request(`${root}/vehicle-usage-logs`),
    request(`${root}/tax-data/import-batches`),
    request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`).catch(() => []),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  return { root, validation, fs, assets, transactions, vehicleLogs, batches, mappings, issues };
}

function taxDataHeader(env, activeLeaf, title, description, validation) {
  const locale = env.locale;
  return `
    <section class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">${escapeHtml(t(locale, "taxData.title"))}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)} / ${escapeHtml(env.context.customerName || "-")} / ${escapeHtml(env.context.fy || "-")}</p>
        </div>
        <div class="button-row">${renderStageRouteButtons(activeLeaf, TAX_DATA_ROUTES, locale)}</div>
      </div>
      ${metrics([
        [t(locale, "taxData.fsTab"), money.format(validation.fs_line_count || 0)],
        [t(locale, "taxData.assetTab"), money.format(validation.asset_count || 0)],
        [t(locale, "taxData.transactionTab"), money.format(validation.transaction_count || 0)],
        [t(locale, "taxData.consistency"), validation.balanced ? t(locale, "status.ok") : t(locale, "status.warn")],
      ])}
    </section>`;
}

function taxDataImportPanel(env, root, batches, importType, label) {
  const locale = env.locale;
  return `
    <article class="panel">
      <div class="panel-head">
        <div><h2>${escapeHtml(t(locale, "taxData.upload"))}</h2><p>${escapeHtml(label)}</p></div>
        <button class="secondary-btn compact" type="button" data-tax-template="${escapeHtml(importType)}">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: label }))}</button>
      </div>
      <form id="importForm" class="stack" data-import-type="${escapeHtml(importType)}">
        <label>CSV/Excel <input id="importFile" type="file" /></label>
        <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.upload"))}</button>
      </form>
      <div id="importResult" class="empty" aria-live="polite"></div>
      <h3>${escapeHtml(t(locale, "taxData.importHistory"))}</h3>
      ${table([t(locale, "common.category"), "File", t(locale, "common.total"), t(locale, "status.error"), ""], batches.filter((batch) => !importType || batch.data_type === importType).map((batch) => row([
        escapeHtml(batch.data_type),
        escapeHtml(batch.source_file_name || "-"),
        money.format(batch.row_count),
        money.format(batch.error_count),
        `<button class="secondary-btn compact" type="button" data-import-errors="${escapeHtml(batch.batch_id)}">${escapeHtml(t(locale, "common.errorDetail"))}</button>`,
      ])), t(locale, "grid.empty"))}
      <div id="importErrors" class="stack"></div>
    </article>`;
}

function bindTaxDataCommonActions(env, root, rerender) {
  const locale = env.locale;
  env.outlet.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  env.outlet.querySelectorAll("[data-tax-template]").forEach((button) => {
    button.addEventListener("click", () => {
      downloadBinary(`${routeRoot(env)}/tax-data/templates/${button.dataset.taxTemplate}`, `tax-data-${button.dataset.taxTemplate}-template.csv`);
    });
  });
  env.outlet.querySelectorAll("[data-import-errors]").forEach((button) => {
    button.addEventListener("click", async () => {
      const errors = await request(`${root}/tax-data/import-batches/${encodeURIComponent(button.dataset.importErrors)}/errors`);
      env.outlet.querySelector("#importErrors").innerHTML = `
        <h3>${escapeHtml(t(locale, "taxData.issueDrilldown"))}</h3>
        ${table(["Row", t(locale, "field.status"), t(locale, "field.name"), t(locale, "field.value")], errors.map((error) => row([
          escapeHtml(error.row_no),
          escapeHtml(statusLabel(error.severity, locale)),
          escapeHtml(error.field_name || "-"),
          escapeHtml(localizeTextValue(error.message, locale)),
        ])), t(locale, "grid.empty"))}`;
    });
  });
  env.outlet.querySelector("#importForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const file = env.outlet.querySelector("#importFile").files[0];
    if (!file) return;
    const form = new FormData();
    form.append("file", file);
    await request(`${root}/tax-data/${event.currentTarget.dataset.importType}/import`, {
      method: "POST",
      body: form,
    });
    env.outlet.querySelector("#importResult").textContent = t(locale, "taxData.importResult");
    await rerender();
  });
}

function taxDataRouteForSource(source) {
  if (source === "assets") return "ws/info:assets";
  if (source === "vehicle") return "ws/info:vehicle";
  if (source === "transactions") return "ws/info:transactions";
  return "ws/info:fs";
}

async function renderWorkInfoFinancialStatements(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="financial-statements" data-leaf-key="ws/info:fs">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.fs"), t(locale, "taxData.fsTab"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.fsTab"))}</h2><p>Imported balance sheet, income statement, and mapped statement lines.</p></div>
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "field.value"), "Standard"], data.fs.slice(0, 40).map((item) => row([
            escapeHtml(item.account_code),
            escapeHtml(item.account_name),
            money.format(item.amount),
            escapeHtml(item.standard_account_code || "-"),
          ])), t(locale, "grid.empty"))}
        </article>
        ${taxDataImportPanel(env, data.root, data.batches, "financial-statements", t(locale, "taxData.fsTab"))}
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoFinancialStatements(env));
}

async function renderWorkInfoAccountMapping(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="account-mapping" data-leaf-key="ws/info:mapping">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.mapping"), t(locale, "taxData.mapping"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.mapping"))}</h2><p>Map customer accounts to standard accounts used by adjustment and forms.</p></div>
          <form id="mappingForm" class="stack">
            <div class="form-grid">
              <label>${escapeHtml(t(locale, "field.code"))} <input id="mapSourceCode" value="${escapeHtml(data.fs[0]?.account_code || "")}" /></label>
              <label>${escapeHtml(t(locale, "field.name"))} <input id="mapSourceName" value="${escapeHtml(data.fs[0]?.account_name || "")}" /></label>
              <label>Standard code <input id="mapStandardCode" value="${escapeHtml(data.fs[0]?.standard_account_code || "")}" /></label>
              <label>Standard name <input id="mapStandardName" value="${escapeHtml(data.fs[0]?.standard_account_name || "")}" /></label>
            </div>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "taxData.mappingRule"))}</button>
          </form>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Mapping rules</h2><p>Existing customer-to-standard account mappings.</p></div>
          ${table([t(locale, "field.code"), t(locale, "field.name"), "Standard"], data.mappings.map((item) => row([
            escapeHtml(item.source_account_code),
            escapeHtml(item.source_account_name),
            escapeHtml(item.standard_account_name || item.standard_account_code || "-"),
          ])), t(locale, "grid.empty"))}
        </article>
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoAccountMapping(env));
  env.outlet.querySelector("#mappingForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`, {
      method: "POST",
      body: JSON.stringify({
        statement_type: "FS",
        source_account_code: env.outlet.querySelector("#mapSourceCode").value.trim(),
        source_account_name: env.outlet.querySelector("#mapSourceName").value.trim(),
        standard_account_code: env.outlet.querySelector("#mapStandardCode").value.trim(),
        standard_account_name: env.outlet.querySelector("#mapStandardName").value.trim(),
      }),
    });
    await renderWorkInfoAccountMapping(env);
  });
}

async function renderWorkInfoAssets(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="assets" data-leaf-key="ws/info:assets">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.assets"), t(locale, "taxData.assetTab"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.assetTab"))}</h2><p>Asset register, depreciation source rows, and business vehicle markers.</p></div>
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount"), "Vehicle"], data.assets.slice(0, 40).map((item) => row([
            escapeHtml(item.asset_code),
            escapeHtml(item.asset_name),
            escapeHtml(item.asset_category),
            money.format(item.acquisition_cost),
            item.is_business_vehicle ? "Y" : "N",
          ])), t(locale, "grid.empty"))}
        </article>
        ${taxDataImportPanel(env, data.root, data.batches, "assets", t(locale, "taxData.assetTab"))}
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoAssets(env));
}

async function renderWorkInfoTransactions(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="transactions" data-leaf-key="ws/info:transactions">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.transactions"), t(locale, "taxData.transactionTab"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.transactionTab"))}</h2><p>Donation, entertainment, interest, and other adjustment source transactions.</p></div>
          ${table(["Date", t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount")], data.transactions.slice(0, 40).map((item) => row([
            escapeHtml(item.tx_date),
            escapeHtml(item.partner_name),
            escapeHtml(item.category),
            money.format(item.amount),
          ])), t(locale, "grid.empty"))}
        </article>
        ${taxDataImportPanel(env, data.root, data.batches, "transactions", t(locale, "taxData.transactionTab"))}
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoTransactions(env));
}

async function renderWorkInfoVehicleUsage(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  const vehicleAssetOptions = data.assets
    .filter((asset) => asset.is_business_vehicle)
    .map((asset) => `<option value="${escapeHtml(asset.asset_id)}">${escapeHtml(asset.asset_name)} (${escapeHtml(asset.asset_code)})</option>`)
    .join("");
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="vehicle-usage" data-leaf-key="ws/info:vehicle">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.vehicle"), t(locale, "taxData.vehicleTab"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.vehicleTab"))}</h2><p>Monthly business-use mileage used by business vehicle adjustment.</p></div>
          ${table(["Asset ID", "Month", "Total km", "Business km", "%"], data.vehicleLogs.map((item) => row([
            escapeHtml(item.asset_id),
            escapeHtml(item.usage_month),
            escapeHtml(item.total_distance_km),
            escapeHtml(item.business_distance_km),
            `${(item.business_use_bps / 100).toFixed(1)}%`,
          ])), t(locale, "grid.empty"))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.vehicleEditor"))}</h2><p>${escapeHtml(t(locale, "taxData.addVehicleLog"))}</p></div>
          <form id="vehicleLogForm" class="stack">
            <label>${escapeHtml(t(locale, "taxData.vehicleTab"))} <select id="vehicleAsset">${vehicleAssetOptions}</select></label>
            <div class="form-grid">
              <label>Month <input id="vehicleMonth" type="date" value="${today().slice(0, 7)}-01" /></label>
              <label>Total km <input id="vehicleTotalKm" type="number" value="1000" /></label>
              <label>Business km <input id="vehicleBusinessKm" type="number" value="700" /></label>
            </div>
            <button class="primary-btn" type="submit" ${vehicleAssetOptions ? "" : "disabled"}>${escapeHtml(t(locale, "taxData.addVehicleLog"))}</button>
          </form>
        </article>
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoVehicleUsage(env));
  env.outlet.querySelector("#vehicleLogForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${data.root}/vehicle-usage-logs`, {
      method: "POST",
      body: JSON.stringify({
        asset_id: Number(env.outlet.querySelector("#vehicleAsset").value),
        usage_month: env.outlet.querySelector("#vehicleMonth").value,
        total_distance_km: Number(env.outlet.querySelector("#vehicleTotalKm").value || 0),
        business_distance_km: Number(env.outlet.querySelector("#vehicleBusinessKm").value || 0),
      }),
    });
    await renderWorkInfoVehicleUsage(env);
  });
}

async function renderWorkInfoConsistency(env) {
  if (!requireWorkContext(env)) return;
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadTaxDataWorkbenchData(env);
  const locale = env.locale;
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-tax-data-stage="consistency" data-leaf-key="ws/info:consistency">
      ${taxDataHeader(env, activeLeaf, t(locale, "route.ws.info.consistency"), t(locale, "taxData.consistency"), data.validation)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(t(locale, "taxData.consistency"))}</h2><p>${escapeHtml(t(locale, "taxData.sourceJump"))}</p></div>
            <button id="taxDataValidate" class="secondary-btn compact" type="button">${escapeHtml(t(locale, "common.run"))}</button>
          </div>
          ${renderTaxDataValidationSummary(data.validation)}
          <div class="button-row"><button class="primary-btn" type="button" id="taxDataComplete">${escapeHtml(t(locale, "taxData.completeInput"))}</button></div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.issueDrilldown"))}</h2><p>${escapeHtml(t(locale, "taxData.sourceJump"))}</p></div>
          ${table([t(locale, "field.severity"), t(locale, "field.title"), t(locale, "common.jump")], data.issues.map((issue) => row([
            escapeHtml(statusLabel(issue.severity || "WARN", locale)),
            escapeHtml(issue.message || issue.rule_code || "-"),
            `<button class="secondary-btn compact" type="button" data-source-jump="${escapeHtml(sourceTabForIssue(issue))}">${escapeHtml(t(locale, "common.jump"))}</button>`,
          ])), t(locale, "grid.empty"))}
        </article>
      </section>
    </section>`;
  bindTaxDataCommonActions(env, data.root, () => renderWorkInfoConsistency(env));
  env.outlet.querySelector("#taxDataValidate")?.addEventListener("click", async () => {
    await request(`${data.root}/tax-data/validation`, { method: "POST", body: "{}" });
    await renderWorkInfoConsistency(env);
  });
  env.outlet.querySelectorAll("[data-source-jump]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(taxDataRouteForSource(button.dataset.sourceJump)));
  });
  env.outlet.querySelector("#taxDataComplete")?.addEventListener("click", () => env.navigate("ws/adj:B1"));
}

async function renderWorkInfo(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const locale = env.locale;
  const [validation, fs, assets, transactions, vehicleLogs, batches, mappings, issues] = await Promise.all([
    request(`${root}/tax-data/validation`),
    request(`${root}/tax-data/financial-statements`),
    request(`${root}/tax-data/assets`),
    request(`${root}/tax-data/transactions`),
    request(`${root}/vehicle-usage-logs`),
    request(`${root}/tax-data/import-batches`),
    request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`).catch(() => []),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const vehicleAssetOptions = assets
    .filter((asset) => asset.is_business_vehicle)
    .map((asset) => `<option value="${escapeHtml(asset.asset_id)}">${escapeHtml(asset.asset_name)} (${escapeHtml(asset.asset_code)})</option>`)
    .join("");
  env.outlet.innerHTML = `
    <section class="leaf-workbench tax-data-workbench" data-stage="tax-data" data-workbench="tax-data">
      <section class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "taxData.title"))}</span>
            <h2>${escapeHtml(t(locale, "route.ws.info.fs"))}</h2>
            <p>${escapeHtml(env.context.customerName || "-")} / ${escapeHtml(env.context.fy || "-")}</p>
          </div>
          <div class="button-row">
            <button class="secondary-btn compact" type="button" data-tax-template="financial-statements">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.fsTab") }))}</button>
            <button class="secondary-btn compact" type="button" data-tax-template="assets">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.assetTab") }))}</button>
            <button class="secondary-btn compact" type="button" data-tax-template="transactions">${escapeHtml(t(locale, "taxData.downloadTemplate", { type: t(locale, "taxData.transactionTab") }))}</button>
          </div>
        </div>
      ${metrics([
          [t(locale, "taxData.fsTab"), money.format(validation.fs_line_count || 0)],
          [t(locale, "taxData.assetTab"), money.format(validation.asset_count || 0)],
          [t(locale, "taxData.transactionTab"), money.format(validation.transaction_count || 0)],
          [t(locale, "taxData.consistency"), validation.balanced ? t(locale, "status.ok") : t(locale, "status.warn")],
      ])}
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.upload"))}</h2></div>
          <form id="importForm" class="stack">
            <label>${escapeHtml(t(locale, "common.category"))}
              <select id="importType">
                <option value="financial-statements">${escapeHtml(t(locale, "taxData.fsTab"))}</option>
                <option value="assets">${escapeHtml(t(locale, "taxData.assetTab"))}</option>
                <option value="transactions">${escapeHtml(t(locale, "taxData.transactionTab"))}</option>
              </select>
            </label>
            <label>CSV/Excel <input id="importFile" type="file" /></label>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "common.upload"))}</button>
          </form>
          <div id="importResult" class="empty" aria-live="polite"></div>
          <h3>${escapeHtml(t(locale, "taxData.importHistory"))}</h3>
          ${table([t(locale, "common.category"), "File", t(locale, "common.total"), t(locale, "status.error"), ""], batches.map((batch) => row([
            escapeHtml(batch.data_type),
            escapeHtml(batch.source_file_name || "-"),
            money.format(batch.row_count),
            money.format(batch.error_count),
            `<button class="secondary-btn compact" type="button" data-import-errors="${escapeHtml(batch.batch_id)}">${escapeHtml(t(locale, "common.errorDetail"))}</button>`,
          ])))}
          <div id="importErrors" class="stack"></div>
        </article>
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(t(locale, "taxData.consistency"))}</h2><p>${escapeHtml(t(locale, "taxData.sourceJump"))}</p></div>
            <button id="taxDataValidate" class="secondary-btn compact" type="button">${escapeHtml(t(locale, "common.run"))}</button>
          </div>
          ${renderTaxDataValidationSummary(validation)}
          ${table([t(locale, "field.severity"), t(locale, "field.title"), t(locale, "common.jump")], issues.slice(0, 8).map((issue) => row([
            escapeHtml(statusLabel(issue.severity || "WARN", locale)),
            escapeHtml(issue.message || issue.rule_code || "-"),
            `<button class="secondary-btn compact" type="button" data-source-jump="${escapeHtml(sourceTabForIssue(issue))}">${escapeHtml(t(locale, "common.jump"))}</button>`,
          ])))}
        </article>
      </section>
      <section class="panel">
        <div class="tabs" role="tablist">
          <button class="active" type="button" data-tax-tab-button="fs">${escapeHtml(t(locale, "taxData.fsTab"))}</button>
          <button type="button" data-tax-tab-button="assets">${escapeHtml(t(locale, "taxData.assetTab"))}</button>
          <button type="button" data-tax-tab-button="transactions">${escapeHtml(t(locale, "taxData.transactionTab"))}</button>
          <button type="button" data-tax-tab-button="vehicle">${escapeHtml(t(locale, "taxData.vehicleTab"))}</button>
        </div>
        <div data-tax-tab="fs">
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "field.value")], fs.slice(0, 20).map((item) => row([escapeHtml(item.account_code), escapeHtml(item.account_name), money.format(item.amount)])))}
        </div>
        <div class="hidden" data-tax-tab="assets">
          ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount")], assets.slice(0, 20).map((item) => row([escapeHtml(item.asset_code), escapeHtml(item.asset_name), escapeHtml(item.asset_category), money.format(item.acquisition_cost)])))}
        </div>
        <div class="hidden" data-tax-tab="transactions">
          ${table(["Date", t(locale, "field.name"), t(locale, "common.category"), t(locale, "field.amount")], transactions.slice(0, 20).map((item) => row([escapeHtml(item.tx_date), escapeHtml(item.partner_name), escapeHtml(item.category), money.format(item.amount)])))}
        </div>
        <div class="hidden" data-tax-tab="vehicle">
          ${table(["Asset ID", "Month", "Total km", "Business km", "%"], vehicleLogs.map((item) => row([escapeHtml(item.asset_id), escapeHtml(item.usage_month), escapeHtml(item.total_distance_km), escapeHtml(item.business_distance_km), `${(item.business_use_bps / 100).toFixed(1)}%`])))}
        </div>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.mapping"))}</h2></div>
          <form id="mappingForm" class="stack">
            <div class="form-grid">
              <label>${escapeHtml(t(locale, "field.code"))} <input id="mapSourceCode" value="${escapeHtml(fs[0]?.account_code || "")}" /></label>
              <label>${escapeHtml(t(locale, "field.name"))} <input id="mapSourceName" value="${escapeHtml(fs[0]?.account_name || "")}" /></label>
              <label>Standard code <input id="mapStandardCode" value="${escapeHtml(fs[0]?.standard_account_code || "")}" /></label>
              <label>Standard name <input id="mapStandardName" value="${escapeHtml(fs[0]?.standard_account_name || "")}" /></label>
            </div>
            <button class="primary-btn" type="submit">${escapeHtml(t(locale, "taxData.mappingRule"))}</button>
          </form>
          ${table([t(locale, "field.code"), t(locale, "field.name"), "Standard"], mappings.slice(0, 8).map((item) => row([escapeHtml(item.source_account_code), escapeHtml(item.source_account_name), escapeHtml(item.standard_account_name)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(t(locale, "taxData.vehicleEditor"))}</h2></div>
          <form id="vehicleLogForm" class="stack">
            <label>${escapeHtml(t(locale, "taxData.vehicleTab"))} <select id="vehicleAsset">${vehicleAssetOptions}</select></label>
            <div class="form-grid">
              <label>Month <input id="vehicleMonth" type="date" value="${today().slice(0, 7)}-01" /></label>
              <label>Total km <input id="vehicleTotalKm" type="number" value="1000" /></label>
              <label>Business km <input id="vehicleBusinessKm" type="number" value="700" /></label>
            </div>
            <button class="primary-btn" type="submit" ${vehicleAssetOptions ? "" : "disabled"}>${escapeHtml(t(locale, "taxData.addVehicleLog"))}</button>
          </form>
        </article>
      </section>
      <div class="button-row">
        <button class="primary-btn" type="button" id="taxDataComplete">${escapeHtml(t(locale, "taxData.completeInput"))}</button>
      </div>
    </section>`;

  document.querySelectorAll("[data-tax-template]").forEach((button) => {
    button.addEventListener("click", () => {
      downloadBinary(`${routeRoot(env)}/tax-data/templates/${button.dataset.taxTemplate}`, `tax-data-${button.dataset.taxTemplate}-template.csv`);
    });
  });
  document.querySelectorAll("[data-tax-tab-button]").forEach((button) => {
    button.addEventListener("click", () => activateTaxDataTab(button.dataset.taxTabButton));
  });
  document.querySelectorAll("[data-source-jump]").forEach((button) => {
    button.addEventListener("click", () => activateTaxDataTab(button.dataset.sourceJump));
  });
  document.querySelectorAll("[data-import-errors]").forEach((button) => {
    button.addEventListener("click", async () => {
      const errors = await request(`${root}/tax-data/import-batches/${encodeURIComponent(button.dataset.importErrors)}/errors`);
      document.getElementById("importErrors").innerHTML = `
        <h3>${escapeHtml(t(locale, "taxData.issueDrilldown"))}</h3>
        ${table(["Row", t(locale, "field.status"), t(locale, "field.name"), t(locale, "field.value")], errors.map((error) => row([
          escapeHtml(error.row_no),
          escapeHtml(statusLabel(error.severity, locale)),
          escapeHtml(error.field_name || "-"),
          escapeHtml(localizeTextValue(error.message, locale)),
        ])))}`;
    });
  });
  document.getElementById("taxDataValidate").addEventListener("click", async () => {
    const result = await request(`${root}/tax-data/validation`, { method: "POST", body: "{}" });
    await renderWorkInfo(env);
    console.info(result);
  });
  document.getElementById("importForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const file = document.getElementById("importFile").files[0];
    if (!file) return;
    const form = new FormData();
    form.append("file", file);
    await request(`${root}/tax-data/${document.getElementById("importType").value}/import`, {
      method: "POST",
      body: form,
    });
    document.getElementById("importResult").textContent = t(locale, "taxData.importResult");
    await renderWorkInfo(env);
  });
  document.getElementById("mappingForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${routeRoot(env)}/customers/${encodeURIComponent(env.context.customerId)}/account-mappings`, {
      method: "POST",
      body: JSON.stringify({
        statement_type: "FS",
        source_account_code: document.getElementById("mapSourceCode").value.trim(),
        source_account_name: document.getElementById("mapSourceName").value.trim(),
        standard_account_code: document.getElementById("mapStandardCode").value.trim(),
        standard_account_name: document.getElementById("mapStandardName").value.trim(),
      }),
    });
    await renderWorkInfo(env);
  });
  document.getElementById("vehicleLogForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/vehicle-usage-logs`, {
      method: "POST",
      body: JSON.stringify({
        asset_id: Number(document.getElementById("vehicleAsset").value),
        usage_month: document.getElementById("vehicleMonth").value,
        total_distance_km: Number(document.getElementById("vehicleTotalKm").value || 0),
        business_distance_km: Number(document.getElementById("vehicleBusinessKm").value || 0),
      }),
    });
    await renderWorkInfo(env);
  });
  document.getElementById("taxDataComplete").addEventListener("click", () => env.navigate("ws/adj:B1"));
}

function activateTaxDataTab(tab) {
  document.querySelectorAll("[data-tax-tab]").forEach((panel) => {
    panel.classList.toggle("hidden", panel.dataset.taxTab !== tab);
  });
  document.querySelectorAll("[data-tax-tab-button]").forEach((button) => {
    button.classList.toggle("active", button.dataset.taxTabButton === tab);
  });
}

function sourceTabForIssue(issue) {
  const source = String(issue?.source || issue?.source_module || issue?.field_path || issue?.rule_code || "").toLowerCase();
  if (source.includes("asset")) return "assets";
  if (source.includes("vehicle")) return "vehicle";
  if (source.includes("transaction") || source.includes("tx")) return "transactions";
  return "fs";
}

async function renderAdjustmentsLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [adjustments, reserves, b1Items, b4Items, b15Items, history] = await Promise.all([
    request(`${root}/adjustments`),
    request(`${root}/reserves`),
    request(`${root}/adjustments/income`).catch(() => []),
    request(`${root}/adjustments/assets/B4`).catch(() => []),
    request(`${root}/adjustments/evaluation/B15`).catch(() => []),
    request(`${root}/adjustments/history`).catch(() => []),
  ]);
  const itemGrids = {
    B1: { rows: b1Items },
    B4: { rows: b4Items },
    B15: { rows: b15Items },
  };
  const evidenceItem = [...b1Items, ...b4Items, ...b15Items][0];
  const evidenceAttachments = evidenceItem
    ? await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`).catch(() => [])
    : [];
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([
        ["조정 건수", adjustments.length],
        ["유보 건수", reserves.length],
        ["가산", money.format(adjustments.filter((item) => item.direction === "ADD").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
        ["차감", money.format(adjustments.filter((item) => item.direction === "DEDUCT").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
      ])}
      <article class="panel">
        <div class="panel-head"><h2>17개 세무조정 모듈</h2></div>
        <div class="grid four">
          ${adjustmentModules.map(([code, label, family]) => `
            <article class="card">
              <h3>${escapeHtml(code)} ${escapeHtml(label)}</h3>
              <p class="eyebrow">${escapeHtml(family)}</p>
              <button class="primary-btn compact" type="button" data-run-adjustment="${code}">실행</button>
            </article>`).join("")}
        </div>
      </article>
      <section class="grid three">
        ${renderDataGrid({ id: "B1", title: `B1 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B1"), rows: b1Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
        ${renderDataGrid({ id: "B4", title: `B4 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B4"), rows: b4Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
        ${renderDataGrid({ id: "B15", title: `B15 ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, "route.ws.adj.B15"), rows: b15Items, columns: adjustmentGridColumns, importable: true, runLabelKey: "grid.addSample", locale: env.locale })}
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>조정 결과</h2></div>
          ${table(["코드", "방향", "금액", "상태"], adjustments.map((item) => row([
            escapeHtml(item.adj_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.status),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>유보 잔액</h2></div>
          ${table(["코드", "방향", "금액", "모듈"], reserves.map((item) => row([
            escapeHtml(item.reserve_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.source_module),
          ])))}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Adjustment History</h2></div>
        ${table(["Module", "Action", "Item", "Changed"], history.slice(0, 20).map((item) => row([
          escapeHtml(item.source_module),
          escapeHtml(item.action),
          escapeHtml(item.new_data?.item_code || item.old_data?.item_code || "-"),
          escapeHtml(item.changed_at),
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Evidence Attachments</h2></div>
        ${table(["File", "Type", "Storage URL", "Uploaded"], evidenceAttachments.map((item) => row([
          escapeHtml(item.file_name),
          escapeHtml(item.content_type),
          escapeHtml(item.storage_url || "-"),
          escapeHtml(item.created_at),
        ])))}
        <form id="adjustmentEvidenceForm" class="stack">
          <label>File name <input id="evidenceFileName" value="evidence.pdf" /></label>
          <label>Storage URL <input id="evidenceStorageUrl" value="evidence/${Date.now()}.pdf" /></label>
          <button class="primary-btn" type="submit" ${evidenceItem ? "" : "disabled"}>Attach evidence</button>
        </form>
      </article>
    </section>`;

  bindDataGridActions({
    grids: itemGrids,
    onRun: async (moduleCode) => {
      await runAdjustment(root, moduleCode);
      await renderAdjustments(env);
    },
    onImport: async (moduleCode, payload) => {
      await request(`${root}/${adjustmentModulePath(moduleCode)}`, {
        method: "POST",
        body: JSON.stringify(normalizeAdjustmentImportPayload(moduleCode, payload)),
      });
      await renderAdjustments(env);
    },
  });
  document.querySelectorAll("[data-run-adjustment]").forEach((button) => {
    button.addEventListener("click", async () => {
      await runAdjustment(root, button.dataset.runAdjustment);
      await renderAdjustments(env);
    });
  });
  document.getElementById("adjustmentEvidenceForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!evidenceItem) return;
    await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`, {
      method: "POST",
      body: JSON.stringify({
        file_name: document.getElementById("evidenceFileName").value,
        content_type: "application/pdf",
        storage_url: document.getElementById("evidenceStorageUrl").value,
        memo: uiText(env.locale, "세무조정 그리드에서 업로드", "Uploaded from adjustment grid"),
        uploaded_by: env.auth.user.login_id,
        adjustment_item_id: evidenceItem.adjustment_item_id,
      }),
    });
    await renderAdjustments(env);
  });
}

async function runAdjustmentLegacy(root, moduleCode) {
  if (moduleCode === "B1") {
    return request(`${root}/adjustments/income`, {
      method: "POST",
      body: JSON.stringify({
        accounting_income: 500000000,
        items: [
          { section: "ADD", item_code: "B1_SAMPLE_ADD", item_name: "Sample addback", amount: 12000000 },
          { section: "DEDUCT", item_code: "B1_SAMPLE_DEDUCT", item_name: "Sample deduction", amount: 3000000 },
        ],
      }),
    });
  }
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode);
  const path = adjustmentModulePath(moduleCode);
  return request(`${root}/${path}`, { method: "POST", body: JSON.stringify(sampleAdjustmentBody(code, family)) });
}

function adjustmentModulePath(moduleCode) {
  if (moduleCode === "B1") return "adjustments/income";
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode) || [];
  const path = {
    assets: `adjustments/assets/${code}`,
    transactions: `adjustments/transactions/${code}`,
    evaluation: `adjustments/evaluation/${code}`,
    tax: `adjustments/tax/${code}`,
    special: `adjustments/special/${code}`,
  }[family];
  if (!path) throw new Error(`Unsupported adjustment module: ${moduleCode}`);
  return path;
}

function normalizeAdjustmentImportPayload(moduleCode, payload) {
  if (moduleCode === "B1" && Array.isArray(payload)) {
    return {
      accounting_income: null,
      items: payload.map((item) => ({
        section: item.section || item.direction || "ADD",
        item_code: item.item_code || "B1_IMPORT",
        item_name: item.item_name || "가져온 항목",
        amount: Number(item.amount || 0),
      })),
    };
  }
  return payload;
}

function sampleAdjustmentBody(code, family) {
  if (family === "assets") {
    return code === "B10"
      ? { business_use_bps: 7200 }
      : { book_reserve: 90000000, estimated_liability: 65000000, external_fund: 10000000, receivable_balance: 500000000, rate_bps: 100 };
  }
  if (family === "transactions") {
    return {
      accounting_income: 500000000,
      taxable_income_before_donation: 480000000,
      gross_revenue: 3000000000,
      revenue_breakdowns: [{ revenue_category: "국내매출", amount: 3000000000 }],
      weighted_average_loan_balance: 120000000,
      weighted_average_interest_rate_bps: 460,
    };
  }
  if (family === "evaluation") {
    if (code === "B11") return { taxable_income_before_loss: 200000000, loss_carryforwards: [{ origin_year: 2025, original_amount: 100000000, remaining_amount: 100000000, expires_year: 2035 }] };
    if (code === "B15") return { capital_changes: [{ change_date: today(), change_type: "PAID_IN_CAPITAL", amount: 50000000, description: "유상증자" }] };
    return { positions: [{ item_code: "FX01", item_name: "USD 매출채권", book_amount: 120000000, tax_amount: 100000000 }] };
  }
  if (family === "tax") {
    if (code === "B12") return { tax_base: 500000000, calculated_tax: 70000000, credits: [{ credit_type: "RND", base_amount: 100000000, rate_bps: 2500 }] };
    if (code === "B13") return { tax_base: 500000000, regular_tax_after_credits: 30000000, minimum_tax_rate_bps: 1000 };
    return { penalties: [{ penalty_type: "UNDER_REPORTED", tax_base: 100000000, rate_bps: 1000, days_late: 1, reduction_bps: 5000 }] };
  }
  if (code === "B16") {
    return { foreign_incomes: [{ income_type: "INTEREST", gross_amount: 100000000, attributable_expense: 20000000, pe_allocation_bps: 10000, withholding_tax: 5000000 }] };
  }
  return { consolidated_entities: [{ entity_code: "PARENT", entity_name: "모회사", ownership_bps: 10000, taxable_income: 100000000 }], eliminations: [] };
}

function adjustmentFamilyLabel(family, locale) {
  const labels = {
    income: "소득",
    transactions: "거래",
    assets: "자산",
    evaluation: "평가",
    tax: "세액",
    special: "특수",
  };
  return uiText(locale, labels[family] || family, family);
}

function renderAdjustmentModuleNavigator(selectedCode, locale) {
  return adjustmentModules.map(([code, label, family]) => `
    <article class="card ${code === selectedCode ? "active" : ""}" data-adjustment-card="${escapeHtml(code)}">
      <h3>${escapeHtml(code)} ${escapeHtml(label)}</h3>
      <p class="eyebrow">${escapeHtml(adjustmentFamilyLabel(family, locale))}</p>
      <div class="button-row">
        <button class="secondary-btn compact" type="button" data-adjustment-route="ws/adj:${escapeHtml(code)}">${escapeHtml(t(locale, "common.open"))}</button>
        <button class="primary-btn compact" type="button" data-run-adjustment="${escapeHtml(code)}">${escapeHtml(t(locale, "common.run"))}</button>
      </div>
    </article>`).join("");
}

function renderAdjustmentModuleHighlights(spec, context, locale) {
  const vehicleCount = context.vehicleLogs.length;
  const businessAssetCount = context.assets.filter((item) => item.is_business_vehicle).length;
  const highlightRows = locale === "en" ? {
    B1: [["Accounting base", "FORM3 / income bridge"], ["Item workflow", "Addback, deduction, reserve"], ["Current items", money.format(context.currentRows.length)]],
    B2: [["Donation rows", money.format(context.transactions.filter((item) => item.category === "DONATION").length)], ["Carryforward", "Special/general donation tracking"], ["Limit", "Taxable income based"]],
    B3: [["Entertainment rows", money.format(context.transactions.filter((item) => item.category === "ENTERTAINMENT").length)], ["Limit", "Revenue based cap"], ["No-card check", "Receipt / card control"]],
    B4: [["Asset rows", money.format(context.assets.length)], ["Auto calc", "Useful life and tax law"], ["Reserve", "Depreciation gap tracking"]],
    B5: [["Reserve basis", "Book reserve vs estimated liability"], ["External fund", "Pension funding offset"], ["Result", "Reserve disposition"]],
    B6: [["Receivable base", "Bad debt cap by rate"], ["Rate input", "bps-based limit"], ["Output", "Reserve / write-off handling"]],
    B7: [["Position input", "FX monetary positions"], ["Comparison", "Book vs tax valuation"], ["Output", "Gain/loss adjustment"]],
    B8: [["Position input", "Inventory / securities valuation"], ["Comparison", "Book vs tax valuation"], ["Output", "Valuation reserve impact"]],
    B9: [["Interest rows", money.format(context.transactions.filter((item) => item.category === "INTEREST").length)], ["Loan average", "Weighted-average debt and rate"], ["Output", "Disallowed interest categories"]],
    B10: [["Vehicle assets", money.format(businessAssetCount)], ["Usage logs", money.format(vehicleCount)], ["Limit", "Business-use based addback"]],
    B11: [["Carryforward years", "Origin-year remaining balance"], ["Limit", "Deduction cap vs taxable income"], ["Output", "Usage and expiry trace"]],
    B12: [["Credit set", "Credit / reduction catalog"], ["Limit", "Calculated tax cap"], ["Output", "Credit impact on final tax"]],
    B13: [["Tax base", "Minimum tax comparison"], ["Input", "Regular tax after credits"], ["Output", "Additional minimum tax"]],
    B14: [["Penalty set", "Penalty type / delay / reduction"], ["Formula", "Base x rate x timing"], ["Output", "Penalty reflected in payable tax"]],
    B15: [["Capital changes", "Paid-in capital / earnings / reserve"], ["Linkage", "Capital and reserve schedule"], ["Output", "Reserve total and items"]],
    B16: [["Foreign income", "Income / expense / withholding"], ["Allocation", "PE allocation"], ["Output", "Domestic taxable base and tax"]],
    B17: [["Entity set", "Consolidated entity taxable income"], ["Elimination", "Intercompany elimination"], ["Output", "Consolidated tax base"]],
  } : {
    B1: [["회계 기준", "별지3호 / 소득 연결"], ["항목 흐름", "가산, 차감, 유보"], ["현재 항목", money.format(context.currentRows.length)]],
    B2: [["기부금 거래", money.format(context.transactions.filter((item) => item.category === "DONATION").length)], ["이월 관리", "특례/일반 기부금 추적"], ["한도", "과세소득 기준"]],
    B3: [["접대비 거래", money.format(context.transactions.filter((item) => item.category === "ENTERTAINMENT").length)], ["한도", "수입금액 기준 한도"], ["증빙 점검", "영수증 / 카드 통제"]],
    B4: [["자산 건수", money.format(context.assets.length)], ["자동 계산", "내용연수와 세법 기준"], ["유보", "상각 차이 추적"]],
    B5: [["충당금 기준", "장부충당금과 추계액 비교"], ["외부 적립", "퇴직연금 불입액 차감"], ["결과", "충당금 세무조정"]],
    B6: [["채권 기준", "대손한도율 기반"], ["율 입력", "bp 단위 한도"], ["결과", "충당금 / 대손금 반영"]],
    B7: [["포지션 입력", "외화 화폐성 자산/부채"], ["비교", "장부가액과 세무가액"], ["결과", "평가손익 조정"]],
    B8: [["포지션 입력", "재고자산 / 유가증권 평가"], ["비교", "장부가액과 세무가액"], ["결과", "평가 유보 반영"]],
    B9: [["이자 거래", money.format(context.transactions.filter((item) => item.category === "INTEREST").length)], ["차입금 평균", "가중평균 차입금과 이자율"], ["결과", "손금불산입 이자 구분"]],
    B10: [["업무용 차량", money.format(businessAssetCount)], ["운행 기록", money.format(vehicleCount)], ["한도", "업무사용비율 기반 가산"]],
    B11: [["이월연도", "발생연도별 잔액"], ["한도", "과세소득 대비 공제한도"], ["결과", "사용액과 소멸 추적"]],
    B12: [["공제 세트", "세액공제 / 감면 목록"], ["한도", "산출세액 기준"], ["결과", "최종 세액 영향"]],
    B13: [["과세표준", "최저한세 비교"], ["입력", "공제 후 일반세액"], ["결과", "추가 최저한세"]],
    B14: [["가산세 세트", "유형 / 지연 / 감면"], ["산식", "기준액 x 세율 x 기간"], ["결과", "납부세액 반영"]],
    B15: [["자본 변동", "자본금 / 이익잉여금 / 적립금"], ["연동", "자본금과 적립금 명세"], ["결과", "유보 총액과 항목"]],
    B16: [["외국소득", "소득 / 비용 / 원천징수"], ["배분", "국내사업장 귀속"], ["결과", "국내 과세표준과 세액"]],
    B17: [["법인 집합", "연결법인별 과세소득"], ["제거", "내부거래 제거"], ["결과", "연결 과세표준"]],
  };
  const rows = [
    [uiText(locale, "워크플로 상태", "Workflow status"), statusLabel(context.workStatus || "DRAFT", locale)],
    [uiText(locale, "잠금 모드", "Lock mode"), context.lockMode || "OPEN"],
    [uiText(locale, "진행률", "Progress"), `${context.progress ?? 0}%`],
    ...(highlightRows[spec.code] || []),
  ];
  return table([uiText(locale, "구분", "Focus"), uiText(locale, "내용", "Detail")], rows.map(([left, right]) => row([escapeHtml(left), escapeHtml(right)])), t(locale, "grid.empty"));
}

function renderAdjustmentModuleForm(spec, context, locale) {
  const vehicleAsset = context.assets.find((item) => item.is_business_vehicle);
  const transaction = context.transactions[0];
  const label = (ko, en) => escapeHtml(uiText(locale, ko, en));
  const defaultText = (ko, en) => escapeHtml(uiText(locale, ko, en));
  const forms = {
    B1: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("회계상 소득", "Accounting income")} <input id="adjB1AccountingIncome" type="number" value="500000000" /></label><label>${label("가산 금액", "Addback amount")} <input id="adjB1AddbackAmount" type="number" value="12000000" /></label><label>${label("차감 금액", "Deduction amount")} <input id="adjB1DeductionAmount" type="number" value="3000000" /></label></form>`,
    B2: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("기부금 차감 전 과세소득", "Taxable income before donation")} <input id="adjB2TaxableIncome" type="number" value="480000000" /></label><label>${label("원천 기부금 거래", "Donation rows in source")} <input type="text" value="${escapeHtml(String(context.transactions.filter((item) => item.category === "DONATION").length))}" readonly /></label></form>`,
    B3: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("수입금액", "Gross revenue")} <input id="adjB3GrossRevenue" type="number" value="3000000000" /></label><label>${label("수입 구분", "Revenue category")} <input id="adjB3RevenueCategory" value="${defaultText("국내매출", "domestic")}" /></label></form>`,
    B4: `<div class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><p class="empty">${escapeHtml(uiText(locale, "가져온 자산과 내용연수 데이터를 사용해 장부-세무 상각 차이를 자동 계산합니다.", "Uses imported assets and depreciation life data to calculate book-tax gaps automatically."))}</p></div>`,
    B5: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("장부상 충당금", "Book reserve")} <input id="adjB5BookReserve" type="number" value="90000000" /></label><label>${label("추계액", "Estimated liability")} <input id="adjB5EstimatedLiability" type="number" value="65000000" /></label><label>${label("외부 적립금", "External fund")} <input id="adjB5ExternalFund" type="number" value="10000000" /></label></form>`,
    B6: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("장부상 충당금", "Book reserve")} <input id="adjB6BookReserve" type="number" value="5000000" /></label><label>${label("채권 잔액", "Receivable balance")} <input id="adjB6ReceivableBalance" type="number" value="100000000" /></label><label>${label("율(bp)", "Rate (bps)")} <input id="adjB6RateBps" type="number" value="100" /></label></form>`,
    B7: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("포지션 코드", "Position code")} <input id="adjB7PositionCode" value="FX01" /></label><label>${label("포지션명", "Position name")} <input id="adjB7PositionName" value="${defaultText("USD 매출채권", "USD receivable")}" /></label><label>${label("장부가액", "Book amount")} <input id="adjB7BookAmount" type="number" value="120000000" /></label><label>${label("세무가액", "Tax amount")} <input id="adjB7TaxAmount" type="number" value="100000000" /></label></form>`,
    B8: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("포지션 코드", "Position code")} <input id="adjB8PositionCode" value="INV01" /></label><label>${label("포지션명", "Position name")} <input id="adjB8PositionName" value="${defaultText("재고평가충당금", "Inventory reserve")}" /></label><label>${label("장부가액", "Book amount")} <input id="adjB8BookAmount" type="number" value="90000000" /></label><label>${label("세무가액", "Tax amount")} <input id="adjB8TaxAmount" type="number" value="70000000" /></label></form>`,
    B9: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("가중평균 차입금", "Weighted average loan balance")} <input id="adjB9LoanBalance" type="number" value="120000000" /></label><label>${label("가중평균 이자율(bp)", "Weighted average interest rate (bps)")} <input id="adjB9InterestRateBps" type="number" value="460" /></label><label>${label("원천 거래", "Source transaction")} <input type="text" value="${escapeHtml(transaction?.partner_name || uiText(locale, "이자 원천 거래", "Interest source rows"))}" readonly /></label></form>`,
    B10: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("업무사용비율(bp)", "Business use (bps)")} <input id="adjB10BusinessUseBps" type="number" value="7200" /></label><label>${label("차량 자산", "Vehicle asset")} <input type="text" value="${escapeHtml(vehicleAsset?.asset_name || uiText(locale, "차량 자산 없음", "No vehicle asset"))}" readonly /></label></form>`,
    B11: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("결손금 공제 전 과세소득", "Taxable income before loss")} <input id="adjB11TaxableIncome" type="number" value="200000000" /></label><label>${label("발생연도", "Origin year")} <input id="adjB11OriginYear" type="number" value="2025" /></label><label>${label("원금액", "Original amount")} <input id="adjB11OriginalAmount" type="number" value="100000000" /></label><label>${label("잔액", "Remaining amount")} <input id="adjB11RemainingAmount" type="number" value="100000000" /></label><label>${label("소멸연도", "Expiry year")} <input id="adjB11ExpiryYear" type="number" value="2035" /></label></form>`,
    B12: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("과세표준", "Tax base")} <input id="adjB12TaxBase" type="number" value="500000000" /></label><label>${label("산출세액", "Calculated tax")} <input id="adjB12CalculatedTax" type="number" value="70000000" /></label><label>${label("공제 유형", "Credit type")} <input id="adjB12CreditType" value="RND" /></label><label>${label("공제 대상 금액", "Credit base")} <input id="adjB12BaseAmount" type="number" value="100000000" /></label><label>${label("율(bp)", "Rate (bps)")} <input id="adjB12RateBps" type="number" value="2500" /></label></form>`,
    B13: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("과세표준", "Tax base")} <input id="adjB13TaxBase" type="number" value="500000000" /></label><label>${label("공제 후 일반세액", "Regular tax after credits")} <input id="adjB13RegularTax" type="number" value="30000000" /></label><label>${label("최저한세율(bp)", "Minimum tax rate (bps)")} <input id="adjB13RateBps" type="number" value="1000" /></label></form>`,
    B14: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("가산세 유형", "Penalty type")} <input id="adjB14PenaltyType" value="UNDER_REPORTED" /></label><label>${label("기준세액", "Tax base")} <input id="adjB14TaxBase" type="number" value="100000000" /></label><label>${label("율(bp)", "Rate (bps)")} <input id="adjB14RateBps" type="number" value="1000" /></label><label>${label("지연일수", "Days late")} <input id="adjB14DaysLate" type="number" value="1" /></label><label>${label("감면율(bp)", "Reduction (bps)")} <input id="adjB14ReductionBps" type="number" value="5000" /></label></form>`,
    B15: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("변동 유형", "Change type")} <input id="adjB15ChangeType" value="PAID_IN_CAPITAL" /></label><label>${label("변동 금액", "Change amount")} <input id="adjB15Amount" type="number" value="50000000" /></label><label>${label("설명", "Description")} <input id="adjB15Description" value="${defaultText("유상증자", "capital increase")}" /></label></form>`,
    B16: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("소득 유형", "Income type")} <input id="adjB16IncomeType" value="INTEREST" /></label><label>${label("총수입금액", "Gross amount")} <input id="adjB16GrossAmount" type="number" value="100000000" /></label><label>${label("귀속 비용", "Attributable expense")} <input id="adjB16Expense" type="number" value="20000000" /></label><label>${label("국내사업장 배분(bp)", "PE allocation (bps)")} <input id="adjB16PeBps" type="number" value="10000" /></label><label>${label("원천징수세액", "Withholding tax")} <input id="adjB16WithholdingTax" type="number" value="5000000" /></label></form>`,
    B17: `<form class="stack adjustment-module-form" data-adjustment-submit="${escapeHtml(spec.code)}"><label>${label("법인 코드", "Entity code")} <input id="adjB17EntityCode" value="PARENT" /></label><label>${label("법인명", "Entity name")} <input id="adjB17EntityName" value="${defaultText("모회사", "Parent")}" /></label><label>${label("지분율(bp)", "Ownership (bps)")} <input id="adjB17OwnershipBps" type="number" value="10000" /></label><label>${label("과세소득", "Taxable income")} <input id="adjB17TaxableIncome" type="number" value="100000000" /></label></form>`,
  };
  return `<div class="stack">${forms[spec.code] || ""}<div class="button-row"><button class="primary-btn" type="button" data-run-adjustment="${escapeHtml(spec.code)}">${escapeHtml(t(locale, "common.run"))}</button><button class="secondary-btn" type="button" data-adjustment-route="ws/form:linkage">${escapeHtml(uiText(locale, "서식 연동", "Form linkage"))}</button><button class="secondary-btn" type="button" data-adjustment-route="ws/val:issues">${escapeHtml(t(locale, "common.jump"))} ${escapeHtml(t(locale, "route.ws.val.issues"))}</button></div></div>`;
}

function renderAdjustmentRunSummary(spec, lastRun, locale) {
  if (!lastRun) {
    return `<p class="empty">${escapeHtml(uiText(locale, `${spec.code} 모듈을 실행하면 계산 요약, 적용 법령, 후속 세액 영향이 표시됩니다.`, `Run the ${spec.code} module to populate calculation summary, law banner, and downstream tax impact.`))}</p>`;
  }
  return `${metrics([
    [uiText(locale, "항목", "Items"), money.format(lastRun.items?.length || 0)],
    [uiText(locale, "가산", "Addbacks"), money.format(lastRun.addbacks || 0)],
    [uiText(locale, "차감", "Deductions"), money.format(lastRun.deductions || 0)],
    [uiText(locale, "스냅샷", "Snapshot"), money.format(lastRun.snapshot_id || 0)],
  ])}${table([uiText(locale, "필드", "Field"), uiText(locale, "값", "Value")], summarizeAdjustmentRunRows(lastRun, locale).map(([label, value]) => row([escapeHtml(label), escapeHtml(value)])), t(locale, "grid.empty"))}`;
}

function summarizeAdjustmentRunRows(lastRun, locale) {
  const rows = [
    [uiText(locale, "모듈", "Module"), lastRun.module_code || "-"],
    [uiText(locale, "법령 버전", "Law version"), lastRun.law_banner?.law?.version_code || "-"],
    [uiText(locale, "생성 유보", "Reserves created"), money.format(lastRun.reserves_created?.length || 0)],
  ];
  if (typeof lastRun.calculated_tax === "number") rows.push([uiText(locale, "산출세액", "Calculated tax"), money.format(lastRun.calculated_tax)]);
  if (typeof lastRun.determined_tax === "number") rows.push([uiText(locale, "결정세액", "Determined tax"), money.format(lastRun.determined_tax)]);
  if (typeof lastRun.taxable_income === "number") rows.push([uiText(locale, "과세소득", "Taxable income"), money.format(lastRun.taxable_income)]);
  if (Array.isArray(lastRun.donation_carryforwards)) rows.push([uiText(locale, "기부금 이월", "Donation carryforward"), money.format(lastRun.donation_carryforwards.length)]);
  if (lastRun.details) rows.push([uiText(locale, "상세 구간", "Detail sections"), money.format(Object.keys(lastRun.details).length)]);
  return rows;
}

function renderAdjustmentDataContext(context, locale) {
  return table([uiText(locale, "원천", "Source"), uiText(locale, "건수", "Count")], [
    row([uiText(locale, "현재 모듈 행", "Current module rows"), money.format(context.currentRows.length)]),
    row([uiText(locale, "자산 행", "Asset rows"), money.format(context.assets.length)]),
    row([uiText(locale, "거래 행", "Transaction rows"), money.format(context.transactions.length)]),
    row([uiText(locale, "차량 운행기록", "Vehicle logs"), money.format(context.vehicleLogs.length)]),
    row([uiText(locale, "작업 상태", "Work status"), statusLabel(context.workStatus || "DRAFT", locale)]),
    row([uiText(locale, "잠금 모드", "Lock mode"), context.lockMode || "OPEN"]),
  ], t(locale, "grid.empty"));
}

function collectAdjustmentPayload(moduleCode) {
  switch (moduleCode) {
    case "B1":
      return { accounting_income: numberValue("adjB1AccountingIncome"), items: [{ section: "ADD", item_code: "B1_SAMPLE_ADD", item_name: "샘플 가산", amount: numberValue("adjB1AddbackAmount") }, { section: "DEDUCT", item_code: "B1_SAMPLE_DEDUCT", item_name: "샘플 차감", amount: numberValue("adjB1DeductionAmount") }] };
    case "B2":
      return { taxable_income_before_donation: numberValue("adjB2TaxableIncome") };
    case "B3":
      return { gross_revenue: numberValue("adjB3GrossRevenue"), revenue_breakdowns: [{ revenue_category: textValue("adjB3RevenueCategory") || "국내매출", amount: numberValue("adjB3GrossRevenue") }] };
    case "B4":
      return {};
    case "B5":
      return { book_reserve: numberValue("adjB5BookReserve"), estimated_liability: numberValue("adjB5EstimatedLiability"), external_fund: numberValue("adjB5ExternalFund") };
    case "B6":
      return { book_reserve: numberValue("adjB6BookReserve"), receivable_balance: numberValue("adjB6ReceivableBalance"), rate_bps: numberValue("adjB6RateBps") };
    case "B7":
      return { positions: [{ item_code: textValue("adjB7PositionCode"), item_name: textValue("adjB7PositionName"), book_amount: numberValue("adjB7BookAmount"), tax_amount: numberValue("adjB7TaxAmount"), monetary: true }] };
    case "B8":
      return { positions: [{ item_code: textValue("adjB8PositionCode"), item_name: textValue("adjB8PositionName"), book_amount: numberValue("adjB8BookAmount"), tax_amount: numberValue("adjB8TaxAmount"), monetary: false }] };
    case "B9":
      return { weighted_average_loan_balance: numberValue("adjB9LoanBalance"), weighted_average_interest_rate_bps: numberValue("adjB9InterestRateBps") };
    case "B10":
      return { business_use_bps: numberValue("adjB10BusinessUseBps") };
    case "B11":
      return { taxable_income_before_loss: numberValue("adjB11TaxableIncome"), loss_carryforwards: [{ origin_year: numberValue("adjB11OriginYear"), original_amount: numberValue("adjB11OriginalAmount"), remaining_amount: numberValue("adjB11RemainingAmount"), expires_year: numberValue("adjB11ExpiryYear") }] };
    case "B12":
      return { tax_base: numberValue("adjB12TaxBase"), calculated_tax: numberValue("adjB12CalculatedTax"), credits: [{ credit_type: textValue("adjB12CreditType"), base_amount: numberValue("adjB12BaseAmount"), rate_bps: numberValue("adjB12RateBps") }] };
    case "B13":
      return { tax_base: numberValue("adjB13TaxBase"), regular_tax_after_credits: numberValue("adjB13RegularTax"), minimum_tax_rate_bps: numberValue("adjB13RateBps") };
    case "B14":
      return { penalties: [{ penalty_type: textValue("adjB14PenaltyType"), tax_base: numberValue("adjB14TaxBase"), rate_bps: numberValue("adjB14RateBps"), days_late: numberValue("adjB14DaysLate"), reduction_bps: numberValue("adjB14ReductionBps") }] };
    case "B15":
      return { capital_changes: [{ change_date: today(), change_type: textValue("adjB15ChangeType"), amount: numberValue("adjB15Amount"), description: textValue("adjB15Description") }] };
    case "B16":
      return { foreign_incomes: [{ income_type: textValue("adjB16IncomeType"), gross_amount: numberValue("adjB16GrossAmount"), attributable_expense: numberValue("adjB16Expense"), pe_allocation_bps: numberValue("adjB16PeBps"), withholding_tax: numberValue("adjB16WithholdingTax") }] };
    case "B17":
      return { consolidated_entities: [{ entity_code: textValue("adjB17EntityCode"), entity_name: textValue("adjB17EntityName"), ownership_bps: numberValue("adjB17OwnershipBps"), taxable_income: numberValue("adjB17TaxableIncome") }], eliminations: [] };
    default:
      return sampleAdjustmentBody(moduleCode, adjustmentTaxonomy.find((item) => item.code === moduleCode)?.module);
  }
}

function numberValue(id) {
  return Number(document.getElementById(id)?.value || 0);
}

function textValue(id) {
  return document.getElementById(id)?.value?.trim() || "";
}

async function renderAdjustments(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const selectedCode = env.leafSuffix || "B1";
  const selectedModule = adjustmentTaxonomy.find((item) => item.code === selectedCode) || adjustmentTaxonomy[0];
  const [adjustments, reserves, history, currentRows, assets, transactions, vehicleLogs] = await Promise.all([
    request(`${root}/adjustments`),
    request(`${root}/reserves`),
    request(`${root}/adjustments/history`).catch(() => []),
    request(`${root}/${adjustmentModulePath(selectedCode)}`).catch(() => []),
    request(`${root}/tax-data/assets`).catch(() => []),
    request(`${root}/tax-data/transactions`).catch(() => []),
    request(`${root}/vehicle-usage-logs`).catch(() => []),
  ]);
  const itemGrids = {
    [selectedCode]: { rows: currentRows },
  };
  const shellContext = {
    assets,
    transactions,
    vehicleLogs,
    currentRows,
    workStatus: env.context?.status || "DRAFT",
    lockMode: env.context?.lockMode || "OPEN",
    progress: env.context?.progress ?? 0,
  };
  const evidenceItem = currentRows[0];
  const evidenceAttachments = evidenceItem
    ? await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`).catch(() => [])
    : [];
  const lastRun = adjustmentRunState.get(selectedCode) || null;
  const selectedTitle = uiText(env.locale, `${selectedModule.code} ${selectedModule.ko}`, `${selectedModule.code} ${selectedModule.en}`);
  const selectedSubtitle = uiText(env.locale, adjustmentFamilyLabel(selectedModule.module, env.locale), `${selectedModule.en} / ${selectedModule.module}`);
  env.outlet.innerHTML = `
    <section class="leaf-workbench adjustment-workbench" data-stage="adjustment" data-adjustment-stage="${escapeHtml(selectedCode)}" data-module-code="${escapeHtml(selectedCode)}" data-leaf-key="ws/adj:${escapeHtml(selectedCode)}">
      ${metrics([
        [uiText(env.locale, "조정", "Adjustments"), adjustments.length],
        [uiText(env.locale, "유보", "Reserves"), reserves.length],
        [uiText(env.locale, "가산", "Addbacks"), money.format(adjustments.filter((item) => item.direction === "ADD").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
        [uiText(env.locale, "차감", "Deductions"), money.format(adjustments.filter((item) => item.direction === "DEDUCT").reduce((sum, item) => sum + Number(item.amount || 0), 0))],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(uiText(env.locale, "세무조정", "Adjustment workbench"))}</span>
            <h2>${escapeHtml(selectedTitle)}</h2>
            <p>${escapeHtml(selectedSubtitle)}</p>
          </div>
          <div>
            <p>${escapeHtml(statusLabel(shellContext.workStatus, env.locale))} / ${escapeHtml(shellContext.lockMode)}</p>
            <p>${escapeHtml(t(env.locale, "field.progress"))} ${escapeHtml(shellContext.progress)}%</p>
          </div>
        </div>
        <div class="grid four adjustment-module-grid">
          ${renderAdjustmentModuleNavigator(selectedCode, env.locale)}
        </div>
      </article>
      <section class="grid two adjustment-shell">
        <article class="panel">
          <div class="panel-head">
            <div><h2>${escapeHtml(uiText(env.locale, "모듈 입력", "Module shell"))}</h2><p>${escapeHtml(selectedModule.code)} / ${escapeHtml(t(env.locale, `route.ws.adj.${selectedModule.code}`))}</p></div>
            <div class="button-row">
              <button class="primary-btn compact" type="button" data-run-adjustment="${escapeHtml(selectedCode)}">${escapeHtml(uiText(env.locale, "모듈 실행", "Run module"))}</button>
              <button class="secondary-btn compact" type="button" data-adjustment-route="ws/form:form3">FORM3</button>
              <button class="secondary-btn compact" type="button" data-adjustment-route="ws/form:linkage">${escapeHtml(uiText(env.locale, "연동", "Linkage"))}</button>
            </div>
          </div>
          ${renderAdjustmentModuleHighlights(selectedModule, shellContext, env.locale)}
          ${renderAdjustmentModuleForm(selectedModule, shellContext, env.locale)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "실행 요약", "Run summary"))}</h2><p>${escapeHtml(uiText(env.locale, "마지막 계산 결과와 후속 세액 영향", "Last calculation and downstream impact"))}</p></div>
          ${renderAdjustmentRunSummary(selectedModule, lastRun, env.locale)}
          ${renderAdjustmentDataContext(shellContext, env.locale)}
        </article>
      </section>
      <section class="grid two">
        ${renderDataGrid({ id: selectedCode, title: `${selectedCode} ${t(env.locale, "grid.itemGrid")}`, subtitle: t(env.locale, `route.ws.adj.${selectedCode}`), rows: currentRows, columns: adjustmentGridColumns, importable: true, runLabelKey: "common.run", locale: env.locale })}
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "조정 결과", "Adjustment results"))}</h2></div>
          ${table([uiText(env.locale, "코드", "Code"), uiText(env.locale, "방향", "Direction"), uiText(env.locale, "금액", "Amount"), uiText(env.locale, "상태", "Status")], adjustments.map((item) => row([
            escapeHtml(item.adj_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.status),
          ])))}
        </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "유보 요약", "Reserve summary"))}</h2></div>
          ${table([uiText(env.locale, "코드", "Code"), uiText(env.locale, "방향", "Direction"), uiText(env.locale, "금액", "Amount"), uiText(env.locale, "모듈", "Module")], reserves.map((item) => row([
            escapeHtml(item.reserve_code),
            escapeHtml(item.direction),
            money.format(item.amount),
            escapeHtml(item.source_module),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "조정 이력", "Adjustment history"))}</h2></div>
          ${table([uiText(env.locale, "모듈", "Module"), uiText(env.locale, "작업", "Action"), uiText(env.locale, "항목", "Item"), uiText(env.locale, "변경시각", "Changed")], history.slice(0, 20).map((item) => row([
            escapeHtml(item.source_module),
            escapeHtml(item.action),
            escapeHtml(item.new_data?.item_code || item.old_data?.item_code || "-"),
            escapeHtml(item.changed_at),
          ])))}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "증빙 첨부", "Evidence attachments"))}</h2></div>
        ${table([uiText(env.locale, "파일", "File"), uiText(env.locale, "유형", "Type"), uiText(env.locale, "저장 URL", "Storage URL"), uiText(env.locale, "업로드", "Uploaded")], evidenceAttachments.map((item) => row([
          escapeHtml(item.file_name),
          escapeHtml(item.content_type),
          escapeHtml(item.storage_url || "-"),
          escapeHtml(item.created_at),
        ])))}
        <form id="adjustmentEvidenceForm" class="stack">
          <label>${escapeHtml(uiText(env.locale, "파일명", "File name"))} <input id="evidenceFileName" value="evidence.pdf" /></label>
          <label>${escapeHtml(uiText(env.locale, "저장 URL", "Storage URL"))} <input id="evidenceStorageUrl" value="evidence/${Date.now()}.pdf" /></label>
          <button class="primary-btn" type="submit" ${evidenceItem ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "증빙 첨부", "Attach evidence"))}</button>
        </form>
      </article>
    </section>`;

  bindDataGridActions({
    grids: itemGrids,
    onRun: async (moduleCode) => {
      const result = await runAdjustment(root, moduleCode, collectAdjustmentPayload(moduleCode));
      adjustmentRunState.set(moduleCode, result);
      await renderAdjustments(env);
    },
    onImport: async (moduleCode, payload) => {
      await request(`${root}/${adjustmentModulePath(moduleCode)}`, {
        method: "POST",
        body: JSON.stringify(normalizeAdjustmentImportPayload(moduleCode, payload)),
      });
      await renderAdjustments(env);
    },
  });
  document.querySelectorAll("[data-run-adjustment]").forEach((button) => {
    button.addEventListener("click", async () => {
      const moduleCode = button.dataset.runAdjustment;
      const result = await runAdjustment(root, moduleCode, collectAdjustmentPayload(moduleCode));
      adjustmentRunState.set(moduleCode, result);
      await renderAdjustments(env);
    });
  });
  document.querySelectorAll("[data-adjustment-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.adjustmentRoute));
  });
  document.getElementById("adjustmentEvidenceForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!evidenceItem) return;
    await request(`${root}/adjustments/items/${evidenceItem.adjustment_item_id}/attachments`, {
      method: "POST",
      body: JSON.stringify({
        file_name: document.getElementById("evidenceFileName").value,
        content_type: "application/pdf",
        storage_url: document.getElementById("evidenceStorageUrl").value,
        memo: "Uploaded from adjustment grid",
        uploaded_by: env.auth.user.login_id,
        adjustment_item_id: evidenceItem.adjustment_item_id,
      }),
    });
    await renderAdjustments(env);
  });
}

async function runAdjustment(root, moduleCode, payload = null) {
  if (moduleCode === "B1") {
    return request(`${root}/adjustments/income`, {
      method: "POST",
      body: JSON.stringify(payload || sampleAdjustmentBody("B1", "income")),
    });
  }
  const [code, , family] = adjustmentModules.find(([code]) => code === moduleCode);
  const path = adjustmentModulePath(moduleCode);
  return request(`${root}/${path}`, { method: "POST", body: JSON.stringify(payload || sampleAdjustmentBody(code, family)) });
}

async function renderFormsLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [attachments, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/FORM3/preview`).catch(() => null),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>별지/부속서류</h2><div class="button-row">
          ${["FORM3", "FORM15", "FORM22", "FORM32", "FORM50"].map((code) => `<button class="primary-btn compact" data-generate-form="${code}" type="button">${code}</button>`).join("")}
          <button id="downloadForms" class="secondary-btn compact" type="button">ZIP</button>
        </div></div>
        ${table(["서식", "상태", "검증", "금액"], attachments.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.status),
          money.format(item.validation_count),
          money.format(item.total_amount),
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>별지 3호 미리보기</h2><button id="downloadForm3" class="secondary-btn compact" type="button">PDF</button></div>
        ${preview ? table(["필드", "값", "원천"], preview.fields.map((field) => row([
          escapeHtml(field.label),
          escapeHtml(field.value),
          escapeHtml(field.source),
        ]))) : "<p class=\"empty\">FORM3 생성 전입니다.</p>"}
      </article>
    </section>`;
  document.querySelectorAll("[data-generate-form]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/forms/${button.dataset.generateForm}`, { method: "POST", body: "{}" });
      await renderForms(env);
    });
  });
  document.getElementById("downloadForms").addEventListener("click", () => downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip"));
  document.getElementById("downloadForm3").addEventListener("click", () => downloadBinary(`${root}/forms/FORM3/pdf`, "FORM3.pdf"));
}

async function renderValidationLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [rules, taxData, efile] = await Promise.all([
    request(`${routeRoot(env)}/validation/rules`),
    request(`${root}/tax-data/validation`),
    request(`${root}/efilings/precheck`).catch(() => null),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["규칙", rules.length], ["차변/대변", taxData.balanced ? "일치" : "불일치"], ["전자신고", efile?.valid ? "가능" : "확인 필요"], ["오류", "-"]])}
      <article class="panel">
        <div class="panel-head"><h2>통합 검증</h2><button id="runValidation" class="primary-btn" type="button">실행</button></div>
        <div id="validationResult">${renderValidationOverview(taxData, efile)}</div>
      </article>
    </section>`;
  document.getElementById("runValidation").addEventListener("click", async () => {
    const result = await request(`${root}/validation/run`, { method: "POST", body: "{}" });
    document.getElementById("validationResult").innerHTML = renderValidationResult(root, result);
    bindDismissButtons(root);
  });
}

function renderValidationResultLegacy(root, result) {
  return `
    ${metrics([["실행 규칙", result.executed_rules], ["오류", result.error_count], ["경고", result.warn_count], ["정보", result.info_count]])}
    ${table(["등급", "규칙", "메시지", ""], result.issues.map((issue) => row([
      `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
      escapeHtml(issue.rule_code),
      escapeHtml(issue.message),
      `<button class="secondary-btn compact" data-dismiss-issue="${issue.issue_id}" type="button">무시</button>`,
    ])), "검증 이슈가 없습니다.")}
    <p class="empty">검증 결과: ${result.pass ? "통과" : "확인 필요"}</p>`;
}

function bindDismissButtons(root) {
  document.querySelectorAll("[data-dismiss-issue]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/validation/issues/${button.dataset.dismissIssue}/dismiss`, {
        method: "POST",
        body: JSON.stringify({ reason: "user dismissed from validation screen" }),
      });
      button.closest("tr").remove();
    });
  });
}

async function renderApprovalLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [queue, workflow] = await Promise.all([
    request(`${routeRoot(env)}/workflow/queue?assignee=me`),
    request(`${root}/workflow`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>결재 대기함</h2></div>
        ${table(["고객사", "사업연도", "대기일"], queue.map((item) => row([escapeHtml(item.customer_name), escapeHtml(item.year_label), `${item.pending_days}일`])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>결재 처리</h2></div>
        <form id="workflowForm" class="stack">
          <label>의견 <textarea id="workflowComment">검토 완료</textarea></label>
          <label>Approvers <input id="workflowApprovers" value="${escapeHtml(env.auth.user.login_id)}" /></label>
          <div class="button-row">
            <button class="secondary-btn" type="button" data-status="IN_REVIEW">결재 요청</button>
            <button class="primary-btn" type="button" data-status="APPROVED">승인</button>
            <button class="danger-btn" type="button" data-status="DRAFT">반려</button>
          </div>
        </form>
        ${table(["작업", "상태", "사용자", "의견"], workflow.events.map((event) => row([escapeHtml(event.action), escapeHtml(event.to_status), escapeHtml(event.actor), escapeHtml(event.comment || "-")])))} 
      </article>
    </section>`;
  document.querySelectorAll("[data-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      const updated = await request(`${root}/status`, {
        method: "POST",
        body: JSON.stringify({ status: button.dataset.status, actor: env.auth.user.login_id, approver: env.auth.user.login_id, approvers: document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean), comment: document.getElementById("workflowComment").value }),
      });
      env.setContext({ status: updated.status, progress: progressForStatus(updated.status), lockMode: updated.locked_at ? "LOCKED" : "OPEN" });
      await renderApproval(env);
    });
  });
}

async function renderPrintLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [attachments, printHistory] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/print-history`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>출력</h2><button id="printBundle" class="primary-btn" type="button">일괄 ZIP</button></div>
        ${table(["서식", "상태", "PDF"], attachments.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.status),
          `<button class="secondary-btn compact" data-download-form="${escapeHtml(item.form_code)}" type="button">다운로드</button>`,
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Print History</h2></div>
        ${table(["Form", "Watermark", "Printed by", "Printed at"], printHistory.map((item) => row([
          escapeHtml(item.form_code),
          escapeHtml(item.watermark),
          escapeHtml(item.printed_by),
          escapeHtml(item.printed_at),
        ])))}
      </article>
    </section>`;
  document.getElementById("printBundle").addEventListener("click", () => downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip"));
  document.querySelectorAll("[data-download-form]").forEach((button) => {
    button.addEventListener("click", () => downloadBinary(`${root}/forms/${button.dataset.downloadForm}/pdf`, `${button.dataset.downloadForm}.pdf`));
  });
}

async function renderEfilingLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const [spec, precheck, history] = await Promise.all([
    request(`${root}/efilings/format-spec`),
    request(`${root}/efilings/precheck`),
    request(`${root}/efilings`),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["레코드", precheck.record_count], ["검증", precheck.valid ? "통과" : "확인"], ["체크섬", precheck.checksum_preview], ["파일", history.length]])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>전자신고 생성</h2><button id="createEfile" class="primary-btn" type="button">생성</button></div>
          ${env.auth.user.use_2fa ? `<label>OTP <input id="efileOtp" inputmode="numeric" autocomplete="one-time-code" placeholder="2FA code" /></label>` : ""}
          ${table(["코드", "등급", "메시지"], asArray(precheck.issues).map((issue) => row([escapeHtml(issue.validation_code), escapeHtml(issue.severity), escapeHtml(issue.message)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>생성 이력</h2></div>
          ${table(["ID", "상태", "파일"], history.map((item) => row([
            escapeHtml(item.efiling_id),
            escapeHtml(item.status),
            `<button class="secondary-btn compact" data-download-efile="${item.efiling_id}" type="button">다운로드</button>`,
          ])))}
        </article>
      </section>
      <article class="panel">${table(["레코드", "필드", "길이", "원천"], spec.slice(0, 30).map((field) => row([escapeHtml(field.record_type), escapeHtml(field.field_name), escapeHtml(field.byte_length), escapeHtml(field.source_path || "-")])))}</article>
    </section>`;
  document.getElementById("createEfile").addEventListener("click", async () => {
    await request(`${root}/efilings`, { method: "POST", body: JSON.stringify({ max_attempts: 3, otp: document.getElementById("efileOtp")?.value || null }) });
    await renderEfiling(env);
  });
  document.querySelectorAll("[data-download-efile]").forEach((button) => {
    button.addEventListener("click", () => downloadBinary(`${routeRoot(env)}/efilings/${button.dataset.downloadEfile}/file`, `efiling-${button.dataset.downloadEfile}.txt`));
  });
}

async function renderPostHistoryLegacy(env) {
  const root = routeRoot(env);
  const years = await request(`${root}/business-years`);
  const efilings = hasWorkContext(env.context) ? await request(`${workRoot(env)}/efilings`) : [];
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>사업연도</h2></div>
        ${table(["ID", "사업연도", "상태", "잠금"], years.map((by) => row([escapeHtml(by.by_id), escapeHtml(by.year_label), pill(by.status), escapeHtml(by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN"))])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>전자신고 이력</h2></div>
        ${table(["접수 ID", "상태", "레코드", "체크섬"], efilings.map((item) => row([escapeHtml(item.efiling_id), escapeHtml(item.status), escapeHtml(item.total_records), escapeHtml(item.checksum)])))}
      </article>
    </section>`;
}

async function renderPostAmendLegacy(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const preview = await request(`${root}/amendment-preview`);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>차이 미리보기</h2></div>
        ${table(["영역", "필드", "현재"], asArray(preview.differences).map((item) => row([escapeHtml(item.area), escapeHtml(item.field), escapeHtml(JSON.stringify(item.current_value))])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>잠금 해제</h2></div>
        <form id="unlockForm" class="stack">
          <label>버전 기준
            <select id="unlockMode"><option value="FILED_VERSION">신고시점 버전</option><option value="CURRENT">최신 버전</option></select>
          </label>
          <label>사유 <textarea id="unlockReason">수정신고 착수</textarea></label>
          <button class="primary-btn" type="submit">해제</button>
        </form>
      </article>
    </section>`;
  document.getElementById("unlockForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${root}/unlock`, {
      method: "POST",
      body: JSON.stringify({ reason: document.getElementById("unlockReason").value, version_mode: document.getElementById("unlockMode").value, actor: env.auth.user.login_id }),
    });
    env.setContext({ byId: by.by_id, fy: String(by.year_label || env.context.fy || ""), period: `${by.start_date || ""} ~ ${by.end_date || ""}`, status: by.status, progress: progressForStatus(by.status), lockMode: by.lock_mode || "AMENDMENT_UNLOCK" });
    await renderPostAmend(env);
  });
}

function statusIn(status, allowed) {
  return allowed.includes(String(status || "").toUpperCase());
}

function renderStageRouteButtons(activeLeaf, routes, locale) {
  return routes.map((routeKey) => `
    <button class="${routeKey === activeLeaf ? "primary-btn" : "secondary-btn"} compact" type="button" data-stage-route="${escapeHtml(routeKey)}">
      ${escapeHtml(t(locale, routeKeyToLabelKey(routeKey)))}
    </button>`).join("");
}

function formatWorkbenchValue(value) {
  if (value == null) return "-";
  if (typeof value === "number") return money.format(value);
  if (typeof value === "boolean") return value ? "Y" : "N";
  if (typeof value === "object") return JSON.stringify(value);
  return String(value);
}

function parseManualFieldValue(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return "";
  if (/^-?\d+(\.\d+)?$/.test(raw)) return Number(raw);
  if (raw === "true") return true;
  if (raw === "false") return false;
  if (raw === "null") return null;
  try {
    if ((raw.startsWith("{") && raw.endsWith("}")) || (raw.startsWith("[") && raw.endsWith("]"))) {
      return JSON.parse(raw);
    }
  } catch {}
  return raw;
}

function formSourceLeaf(field) {
  const ref = String(field?.source_ref || field?.source || field?.field_path || "").toLowerCase();
  if (ref.includes("form")) return "ws/form:preview";
  if (ref.includes("adjust") || ref.includes("reserve")) return "ws/adj:B1";
  if (ref.includes("asset")) return "ws/info:assets";
  if (ref.includes("vehicle")) return "ws/info:vehicle";
  if (ref.includes("transaction") || ref.includes("revenue") || ref.includes("expense")) return "ws/info:transactions";
  return "ws/info:fs";
}

function validationIssueLeaf(issue) {
  const target = String(issue?.target_path || issue?.area || issue?.message || "").toLowerCase();
  if (/b1[0-7]|adjust|reserve/.test(target)) return "ws/adj:B1";
  if (target.includes("form")) return "ws/form:preview";
  if (target.includes("vehicle")) return "ws/info:vehicle";
  if (target.includes("asset")) return "ws/info:assets";
  if (target.includes("transaction")) return "ws/info:transactions";
  return "ws/info:fs";
}

function efilingIssueLeaf(issue) {
  const target = String(issue?.field_path || issue?.message || "").toLowerCase();
  if (target.includes("biz") || target.includes("corp")) return "ws/info:fs";
  if (target.includes("tax") || target.includes("form3")) return "ws/form:preview";
  return "ws/val:issues";
}

function validationCounts(issues) {
  return {
    errors: issues.filter((issue) => issue.severity === "ERROR" && issue.status !== "DISMISSED").length,
    warns: issues.filter((issue) => issue.severity === "WARN" && issue.status !== "DISMISSED").length,
    infos: issues.filter((issue) => issue.severity === "INFO" && issue.status !== "DISMISSED").length,
  };
}

function renderValidationResult(result) {
  return `
    ${metrics([["Executed rules", result.executed_rules], ["Errors", result.error_count], ["Warnings", result.warn_count], ["Infos", result.info_count]])}
    ${table(["Severity", "Rule", "Message"], asArray(result.issues).map((issue) => row([
      `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
      escapeHtml(issue.rule_code),
      escapeHtml(issue.message),
    ])), "No validation issues.")}
    <p class="empty">Validation status: ${result.pass ? "PASS" : "ACTION REQUIRED"}</p>`;
}

const VALIDATION_ROUTES = ["ws/val:run", "ws/val:issues", "ws/val:rules"];

async function loadValidationWorkbenchData(env) {
  const root = workRoot(env);
  const [rules, taxData, efile, issues] = await Promise.all([
    request(`${routeRoot(env)}/validation/rules`),
    request(`${root}/tax-data/validation`),
    request(`${root}/efilings/precheck`).catch(() => null),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const byId = env.context?.byId || env.context?.businessYearId || "default";
  const normalizedIssues = asArray(issues);
  const counts = validationCounts(normalizedIssues);
  return {
    root,
    rules: asArray(rules),
    taxData: taxData || {},
    efile,
    issues: normalizedIssues,
    byId,
    lastResult: validationRunState.get(byId) || null,
    counts,
    approvalBlocked: counts.errors > 0,
  };
}

function renderValidationHeader(env, activeLeaf, title, description, data, actions = "") {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge ${data.approvalBlocked ? "warn" : "ok"}">${data.approvalBlocked ? "Validation blocked" : "Ready for approval"}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, VALIDATION_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function renderValidationGateSummary(taxData, efile) {
  return table(["Check", "State"], [
    row(["Financial data balanced", taxData.balanced ? "OK" : "CHECK"]),
    row(["Asset rows", money.format(taxData.asset_count || 0)]),
    row(["Vehicle logs", money.format(taxData.business_vehicle_count || 0)]),
    row(["Transaction rows", money.format(taxData.transaction_count || 0)]),
    row(["E-file precheck", efile?.valid ? "READY" : "CHECK"]),
  ]);
}

function bindValidationRouteButtons(env) {
  localizeRenderedOutlet(env.outlet, env.locale);
  document.querySelectorAll("[data-stage-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.stageRoute));
  });
}

const FORM_ROUTES = ["ws/form:form3", "ws/form:attachments", "ws/form:preview", "ws/form:linkage"];

async function loadFormsWorkbenchData(env, formCode = null) {
  const root = workRoot(env);
  const selectedFormCode = formCode || env.context?.selectedFormCode || "FORM3";
  const [attachments, linkage, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/linkage-check`).catch(() => ({ balanced: false, differences: [] })),
    request(`${root}/forms/${selectedFormCode}/preview`).catch(() => null),
  ]);
  const normalizedAttachments = asArray(attachments);
  const selectedAttachment = normalizedAttachments.find((item) => item.form_code === selectedFormCode) || normalizedAttachments[0] || null;
  return {
    root,
    selectedFormCode,
    attachments: normalizedAttachments,
    linkage: linkage || { balanced: false, differences: [] },
    preview,
    selectedAttachment,
    editableFields: asArray(preview?.fields).filter((field) => field.editable).slice(0, 6),
    canPrint: statusIn(env.context?.status, ["APPROVED", "FILED", "AMENDED"]),
  };
}

function renderFormsMetrics(data) {
  return metrics([
    ["Forms", data.attachments.length],
    ["Generated", data.attachments.filter((item) => item.generated).length],
    ["Linkage", data.linkage?.balanced ? "BALANCED" : "CHECK"],
    ["Preview validations", money.format(asArray(data.preview?.validations).length)],
  ]);
}

function renderFormsHeader(env, activeLeaf, title, description, data, actions = "") {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Form review workbench</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)} / ${escapeHtml(statusLabel(env.context?.status || "DRAFT", env.locale))} / ${escapeHtml(env.context?.lockMode || "OPEN")}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, FORM_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function renderFormCatalogTable(data) {
  return table(["Form", "Status", "Validations", "Amount", "Updated", ""], data.attachments.map((item) => row([
    escapeHtml(item.form_code),
    escapeHtml(item.status),
    money.format(item.validation_count),
    money.format(item.total_amount),
    escapeHtml(item.updated_at || "-"),
    `<div class="button-row"><button class="secondary-btn compact" type="button" data-select-form="${escapeHtml(item.form_code)}">Preview</button><button class="secondary-btn compact" type="button" data-generate-form="${escapeHtml(item.form_code)}">Generate</button></div>`,
  ])), "No forms generated yet.");
}

function renderFormPreviewTable(data, limit = 12) {
  return data.preview ? table(["Field", "Value", "Source", ""], asArray(data.preview.fields).slice(0, limit).map((field) => row([
    escapeHtml(field.label),
    escapeHtml(formatWorkbenchValue(field.value)),
    escapeHtml(field.source_ref || field.source || "-"),
    `<button class="secondary-btn compact" type="button" data-form-source-jump="${escapeHtml(formSourceLeaf(field))}">Jump</button>`,
  ])), "No preview fields.") : '<p class="empty">Generate the selected form to review preview fields.</p>';
}

function renderFormValidations(data) {
  return table(["Severity", "Rule", "Message"], asArray(data.preview?.validations).map((issue) => row([
    `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
    escapeHtml(issue.rule_code),
    escapeHtml(issue.message),
  ])), "No form validation issues.");
}

function renderFormLinkageTable(data) {
  return table(["Source", "Target", "Delta"], asArray(data.linkage?.differences).map((item) => row([
    escapeHtml(item.source),
    escapeHtml(item.target),
    escapeHtml(formatWorkbenchValue(item.delta)),
  ])), "No linkage differences.");
}

function renderFormOverridePanel(data) {
  return data.editableFields.length ? `
    <form id="formOverrideForm" class="stack" data-form-override="${escapeHtml(data.selectedFormCode)}">
      ${data.editableFields.map((field) => `<label>${escapeHtml(field.label)} <input data-form-edit-field="${escapeHtml(field.field_path)}" value="${escapeHtml(formatWorkbenchValue(field.value))}" /></label>`).join("")}
      <label>Reason <textarea id="formOverrideReason">Manual review adjustment</textarea></label>
      <button class="primary-btn" type="submit">Save overrides</button>
    </form>` : '<p class="empty">No editable fields in the current preview.</p>';
}

function bindFormsRouteButtons(env) {
  localizeRenderedOutlet(env.outlet, env.locale);
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
}

function bindFormsCommonActions(env, data, rerender) {
  bindFormsRouteButtons(env);
  document.getElementById("selectedFormCode")?.addEventListener("change", (event) => {
    env.setContext({ selectedFormCode: event.target.value });
    rerender();
  });
  document.querySelectorAll("[data-select-form]").forEach((button) => button.addEventListener("click", () => {
    env.setContext({ selectedFormCode: button.dataset.selectForm });
    env.navigate("ws/form:preview");
  }));
  document.querySelectorAll("[data-form-source-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.formSourceJump)));
  document.querySelectorAll("[data-generate-form]").forEach((button) => button.addEventListener("click", async () => {
    await request(`${data.root}/forms/${button.dataset.generateForm}`, { method: "POST", body: "{}" });
    await rerender();
  }));
  document.querySelectorAll("[data-form-pdf]").forEach((button) => button.addEventListener("click", () => {
    if (!button.disabled) downloadBinary(`${data.root}/forms/${button.dataset.formPdf}/pdf`, `${button.dataset.formPdf}.pdf`);
  }));
  document.getElementById("formOverrideForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const fields = Object.fromEntries([...document.querySelectorAll("[data-form-edit-field]")].map((input) => [input.dataset.formEditField, parseManualFieldValue(input.value)]));
    await request(`${data.root}/forms/${data.selectedFormCode}`, {
      method: "PUT",
      body: JSON.stringify({ fields, reason: document.getElementById("formOverrideReason").value, changed_by: env.auth.user.login_id }),
    });
    await rerender();
  });
}

async function renderForms(env) {
  return renderFormsForm3(env);
}

async function renderFormsForm3(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadFormsWorkbenchData(env, "FORM3");
  env.setContext({ selectedFormCode: "FORM3" });
  env.outlet.innerHTML = `
    <section class="leaf-workbench forms-workbench" data-stage="forms" data-form-stage="form3" data-leaf-key="ws/form:form3" data-form-code="FORM3">
      ${renderFormsMetrics(data)}
      ${renderFormsHeader(
        env,
        "ws/form:form3",
        "FORM3 review",
        "Generate and review the main corporate tax return form.",
        data,
        `<button class="primary-btn compact" type="button" data-generate-form="FORM3">Generate FORM3</button><button class="secondary-btn compact" type="button" data-form-pdf="FORM3" ${data.canPrint ? "" : "disabled"}>PDF</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>FORM3 preview</h2><p>${escapeHtml(data.selectedAttachment?.form_name || "FORM3")}</p></div>
          ${renderFormPreviewTable(data, 14)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>FORM3 validation</h2><p>Validation results tied to the main form.</p></div>
          ${renderFormValidations(data)}
        </article>
      </section>
    </section>`;
  bindFormsCommonActions(env, data, () => renderFormsForm3(env));
}

async function renderFormsAttachments(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadFormsWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench forms-workbench" data-stage="forms" data-form-stage="attachments" data-leaf-key="ws/form:attachments" data-form-code="${escapeHtml(data.selectedFormCode)}">
      ${renderFormsMetrics(data)}
      ${renderFormsHeader(
        env,
        "ws/form:attachments",
        "Attachment catalog",
        "Generate, select, and route into review for each attached form.",
        data,
        `<select id="selectedFormCode">${data.attachments.map((item) => `<option value="${item.form_code}" ${item.form_code === data.selectedFormCode ? "selected" : ""}>${escapeHtml(item.form_code)}</option>`).join("")}</select><button class="primary-btn compact" type="button" data-generate-form="${escapeHtml(data.selectedFormCode)}">Generate selected</button>`
      )}
      <article class="panel">
        <div class="panel-head"><h2>Form catalog</h2><p>Generated status, validation count, and amount by form.</p></div>
        ${renderFormCatalogTable(data)}
        <p class="empty">${data.canPrint ? "Approved or filed work can be printed." : "PDF output is gated until approval is complete."}</p>
      </article>
    </section>`;
  bindFormsCommonActions(env, data, () => renderFormsAttachments(env));
}

async function renderFormsPreview(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadFormsWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench forms-workbench" data-stage="forms" data-form-stage="preview" data-leaf-key="ws/form:preview" data-form-code="${escapeHtml(data.selectedFormCode)}">
      ${renderFormsMetrics(data)}
      ${renderFormsHeader(
        env,
        "ws/form:preview",
        `${data.selectedFormCode} preview`,
        "Inspect field values, source references, and editable manual overrides.",
        data,
        `<select id="selectedFormCode">${data.attachments.map((item) => `<option value="${item.form_code}" ${item.form_code === data.selectedFormCode ? "selected" : ""}>${escapeHtml(item.form_code)}</option>`).join("")}</select><button class="secondary-btn compact" type="button" data-form-pdf="${escapeHtml(data.selectedFormCode)}" ${data.canPrint ? "" : "disabled"}>PDF</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Preview and source</h2><p>${escapeHtml(data.selectedAttachment?.form_name || data.selectedFormCode)}</p></div>
          ${renderFormPreviewTable(data, 16)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Manual overrides</h2><p>Editable preview fields write back as manual overrides.</p></div>
          ${renderFormOverridePanel(data)}
          ${data.preview ? table(["Change", "By", "Reason", "At"], asArray(data.preview.history).slice(0, 10).map((item) => row([
            escapeHtml(item.change_type),
            escapeHtml(item.changed_by),
            escapeHtml(item.reason || "-"),
            escapeHtml(item.changed_at),
          ])), "No form edit history.") : ""}
        </article>
      </section>
    </section>`;
  bindFormsCommonActions(env, data, () => renderFormsPreview(env));
}

async function renderFormsLinkage(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadFormsWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench forms-workbench" data-stage="forms" data-form-stage="linkage" data-leaf-key="ws/form:linkage" data-form-code="${escapeHtml(data.selectedFormCode)}">
      ${renderFormsMetrics(data)}
      ${renderFormsHeader(
        env,
        "ws/form:linkage",
        "Form linkage check",
        data.linkage?.balanced ? "No form-to-form linkage differences are currently reported." : "Review linkage differences and jump to the affected form source.",
        data
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Linkage differences</h2><p>Cross-form deltas that affect validation readiness.</p></div>
          ${renderFormLinkageTable(data)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Selected form validation</h2><p>Validation issues from the active form preview.</p></div>
          ${renderFormValidations(data)}
          <div class="button-row">
            <button class="secondary-btn compact" type="button" data-stage-route="ws/form:attachments">Open catalog</button>
            <button class="secondary-btn compact" type="button" data-stage-route="ws/val:issues">Validation issues</button>
          </div>
        </article>
      </section>
    </section>`;
  bindFormsCommonActions(env, data, () => renderFormsLinkage(env));
}

async function renderValidation(env) {
  return renderValidationRun(env);
}

async function renderValidationRun(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadValidationWorkbenchData(env);
  const activeLeaf = "ws/val:run";
  env.outlet.innerHTML = `
    <section class="leaf-workbench validation-workbench" data-stage="validation" data-validation-stage="run" data-leaf-key="ws/val:run">
      ${metrics([
        ["Rules", data.rules.length],
        ["Open errors", data.counts.errors],
        ["Open warnings", data.counts.warns],
        ["E-file precheck", data.efile?.valid ? "READY" : "CHECK"],
      ])}
      ${renderValidationHeader(
        env,
        activeLeaf,
        "Validation run",
        data.approvalBlocked ? "Approval request is blocked until open errors are cleared." : "Run validation and continue to approval when no error remains.",
        data,
        `<button id="runValidation" class="primary-btn compact" type="button">Run validation</button><button id="jumpApproval" class="secondary-btn compact" type="button" ${data.approvalBlocked ? "disabled" : ""}>Request approval</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Run result</h2><p>Tax data consistency and filing readiness.</p></div>
          <div id="validationResult">${data.lastResult ? renderValidationResult(data.lastResult) : renderValidationOverview(data.taxData, data.efile)}</div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Gate summary</h2><p>Approval and filing depend on these checks.</p></div>
          ${renderValidationGateSummary(data.taxData, data.efile)}
          <div class="button-row">
            <button class="secondary-btn compact" type="button" data-stage-route="ws/val:issues">Open issues</button>
            <button class="secondary-btn compact" type="button" data-stage-route="ws/val:rules">Rule catalog</button>
          </div>
        </article>
      </section>
    </section>`;
  bindValidationRouteButtons(env);
  document.getElementById("runValidation")?.addEventListener("click", async () => {
    const result = await request(`${data.root}/validation/run`, { method: "POST", body: "{}" });
    validationRunState.set(data.byId, result);
    await renderValidationRun(env);
  });
  document.getElementById("jumpApproval")?.addEventListener("click", () => env.navigate("ws/appr:request"));
}

async function renderValidationIssues(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadValidationWorkbenchData(env);
  const dismissedCount = data.issues.filter((issue) => issue.status === "DISMISSED").length;
  env.outlet.innerHTML = `
    <section class="leaf-workbench validation-workbench" data-stage="validation" data-validation-stage="issues" data-leaf-key="ws/val:issues">
      ${metrics([
        ["Open errors", data.counts.errors],
        ["Open warnings", data.counts.warns],
        ["Open infos", data.counts.infos],
        ["Dismissed", dismissedCount],
      ])}
      ${renderValidationHeader(
        env,
        "ws/val:issues",
        "Validation issues",
        "Review blocking and non-blocking issues, then jump to the source screen for correction.",
        data
      )}
      <article class="panel">
        <div class="panel-head"><h2>Issue triage</h2><p>Dismiss non-blocking issues or jump to the source screen.</p></div>
        ${table(["Severity", "Rule", "Message", "Status", ""], data.issues.map((issue) => row([
          `<span class="badge ${issue.severity === "ERROR" ? "error" : issue.severity === "WARN" ? "warn" : "info"}">${escapeHtml(issue.severity)}</span>`,
          escapeHtml(issue.rule_code || "-"),
          escapeHtml(issue.message),
          escapeHtml(issue.status || "OPEN"),
          `<div class="button-row"><button class="secondary-btn compact" type="button" data-validation-jump="${escapeHtml(validationIssueLeaf(issue))}">Jump</button><button class="secondary-btn compact" type="button" data-dismiss-issue="${escapeHtml(issue.issue_id)}" ${issue.issue_id ? "" : "disabled"}>Dismiss</button></div>`,
        ])), "No validation issues.")}
      </article>
    </section>`;
  bindValidationRouteButtons(env);
  document.querySelectorAll("[data-validation-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.validationJump)));
  document.querySelectorAll("[data-dismiss-issue]").forEach((button) => button.addEventListener("click", async () => {
    await request(`${data.root}/validation/issues/${button.dataset.dismissIssue}/dismiss`, {
      method: "POST",
      body: JSON.stringify({ reason: "dismissed from validation workbench", dismissed_by: env.auth.user.login_id }),
    });
    await renderValidationIssues(env);
  }));
}

async function renderValidationRules(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadValidationWorkbenchData(env);
  const activeRules = data.rules.filter((rule) => rule.active).length;
  const errorRules = data.rules.filter((rule) => rule.severity === "ERROR").length;
  const warnRules = data.rules.filter((rule) => rule.severity === "WARN").length;
  env.outlet.innerHTML = `
    <section class="leaf-workbench validation-workbench" data-stage="validation" data-validation-stage="rules" data-leaf-key="ws/val:rules">
      ${metrics([
        ["Rules", data.rules.length],
        ["Active", activeRules],
        ["Error rules", errorRules],
        ["Warning rules", warnRules],
      ])}
      ${renderValidationHeader(
        env,
        "ws/val:rules",
        "Validation rules",
        "Review the active rule catalog used by the validation run.",
        data
      )}
      <article class="panel">
        <div class="panel-head"><h2>Rule catalog</h2><p>Active and inactive validation rules for this tenant.</p></div>
        ${table(["Rule", "Severity", "Area", "Active"], data.rules.map((rule) => row([
          escapeHtml(rule.rule_code),
          escapeHtml(rule.severity),
          escapeHtml(rule.area),
          rule.active ? "Y" : "N",
        ])), "No rules loaded.")}
      </article>
    </section>`;
  bindValidationRouteButtons(env);
}

const APPROVAL_ROUTES = ["ws/appr:request", "ws/appr:inbox", "ws/appr:rejected"];

async function loadApprovalWorkbenchData(env) {
  const root = workRoot(env);
  const [queue, workflow, issues] = await Promise.all([
    request(`${routeRoot(env)}/workflow/queue?assignee=me`),
    request(`${root}/workflow`),
    request(`${root}/validation/issues`).catch(() => []),
  ]);
  const counts = validationCounts(asArray(issues));
  return {
    root,
    queue: asArray(queue),
    workflow: workflow || {},
    issues: asArray(issues),
    counts,
    approvalBlocked: counts.errors > 0,
    currentStatus: env.context?.status || workflow?.business_year?.status || "DRAFT",
  };
}

function renderApprovalHeader(env, activeLeaf, title, description, data, actions = "") {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge ${data.approvalBlocked ? "warn" : "ok"}">${data.approvalBlocked ? "Validation blocked" : "Ready for approval"}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, APPROVAL_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function approvalLineApprovers(workflow, env) {
  return asArray(workflow.approval_lines).map((line) => line.approver_login_id).filter(Boolean).join(",") || env.auth.user.login_id;
}

function renderApprovalLines(workflow) {
  return table(["Approver", "Status", "Acted at", "Comment"], asArray(workflow.approval_lines).map((line) => row([
    escapeHtml(line.approver_login_id),
    escapeHtml(line.status),
    escapeHtml(line.acted_at || "-"),
    escapeHtml(line.comment || "-"),
  ])), "No approval lines.");
}

function renderWorkflowEvents(events, emptyLabel = "No workflow events.") {
  return table(["Action", "From", "To", "Actor", "Comment"], asArray(events).map((event) => row([
    escapeHtml(event.action),
    escapeHtml(event.from_status || "-"),
    escapeHtml(event.to_status),
    escapeHtml(event.actor),
    escapeHtml(event.comment || "-"),
  ])), emptyLabel);
}

function bindApprovalRouteButtons(env) {
  document.querySelectorAll("[data-stage-route]").forEach((button) => {
    button.addEventListener("click", () => env.navigate(button.dataset.stageRoute));
  });
}

async function updateWorkflowStatus(env, data, status, options = {}) {
  const updated = await request(`${data.root}/status`, {
    method: "POST",
    body: JSON.stringify({
      status,
      actor: env.auth.user.login_id,
      approver: env.auth.user.login_id,
      approvers: options.approvers || approvalLineApprovers(data.workflow, env).split(",").map((item) => item.trim()).filter(Boolean),
      comment: options.comment || "",
    }),
  });
  env.setContext({ status: updated.status, progress: progressForStatus(updated.status), lockMode: updated.locked_at ? "LOCKED" : "OPEN" });
}

async function renderApproval(env) {
  return renderApprovalRequest(env);
}

async function renderApprovalRequest(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadApprovalWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench approval-workbench" data-stage="approval" data-approval-stage="request" data-leaf-key="ws/appr:request">
      ${metrics([
        ["Approval lines", asArray(data.workflow.approval_lines).length],
        ["Open errors", data.counts.errors],
        ["Queue", data.queue.length],
        ["Current status", statusLabel(data.currentStatus, env.locale)],
      ])}
      ${renderApprovalHeader(
        env,
        "ws/appr:request",
        "Approval request",
        data.approvalBlocked ? "Validation errors must be resolved before requesting approval." : "Select approvers and submit this work for review.",
        data
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Request review</h2><p>Creates approval lines and moves the business year into review.</p></div>
          <form id="workflowForm" class="stack">
            <label>Comment <textarea id="workflowComment">Validation reviewed and ready for approval.</textarea></label>
            <label>Approvers <input id="workflowApprovers" value="${escapeHtml(approvalLineApprovers(data.workflow, env))}" /></label>
            <div class="button-row">
              <button class="primary-btn" type="button" id="requestWorkflow" ${data.approvalBlocked ? "disabled" : ""}>Request review</button>
              <button class="secondary-btn" type="button" data-stage-route="ws/val:issues" ${data.approvalBlocked ? "" : "disabled"}>Resolve validation</button>
            </div>
          </form>
          ${renderApprovalLines(data.workflow)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Workflow timeline</h2><p>Current business year approval events.</p></div>
          ${renderWorkflowEvents(data.workflow.events)}
        </article>
      </section>
    </section>`;
  bindApprovalRouteButtons(env);
  document.getElementById("requestWorkflow")?.addEventListener("click", async () => {
    const approvers = document.getElementById("workflowApprovers").value.split(",").map((item) => item.trim()).filter(Boolean);
    const comment = document.getElementById("workflowComment").value;
    await request(`${data.root}/workflow/request`, {
      method: "POST",
      body: JSON.stringify({
        approvers,
        comment,
        requested_by: env.auth.user.login_id,
      }),
    });
    await updateWorkflowStatus(env, data, "IN_REVIEW", { approvers, comment });
    await renderApprovalRequest(env);
  });
}

async function renderApprovalInbox(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadApprovalWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench approval-workbench" data-stage="approval" data-approval-stage="inbox" data-leaf-key="ws/appr:inbox">
      ${metrics([
        ["Queue", data.queue.length],
        ["Approval lines", asArray(data.workflow.approval_lines).length],
        ["Open errors", data.counts.errors],
        ["Current status", statusLabel(data.currentStatus, env.locale)],
      ])}
      ${renderApprovalHeader(
        env,
        "ws/appr:inbox",
        "Approval inbox",
        data.approvalBlocked ? "Validation errors remain open, so approval action is disabled." : "Review queue items and approve or return the current business year.",
        data
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>My queue</h2><p>Business years currently waiting for my review.</p></div>
          ${table(["Customer", "Year", "Status", "Pending days"], data.queue.map((item) => row([
            escapeHtml(item.customer_name),
            escapeHtml(item.year_label),
            escapeHtml(item.status),
            money.format(item.pending_days),
          ])), "No items in queue.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Decision</h2><p>Approve this work or return it to draft with a review comment.</p></div>
          <form id="workflowDecisionForm" class="stack">
            <label>Decision comment <textarea id="workflowDecisionComment">Review completed from approval inbox.</textarea></label>
            <label>Approvers <input id="workflowDecisionApprovers" value="${escapeHtml(approvalLineApprovers(data.workflow, env))}" /></label>
            <div class="button-row">
              <button class="primary-btn" type="button" data-status="APPROVED" ${data.approvalBlocked ? "disabled" : ""}>Approve</button>
              <button class="danger-btn" type="button" data-status="DRAFT">Return to draft</button>
            </div>
          </form>
          ${renderApprovalLines(data.workflow)}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Workflow timeline</h2><p>Request and decision audit trail.</p></div>
        ${renderWorkflowEvents(data.workflow.events)}
      </article>
    </section>`;
  bindApprovalRouteButtons(env);
  document.querySelectorAll("[data-status]").forEach((button) => {
    button.addEventListener("click", async () => {
      await updateWorkflowStatus(env, data, button.dataset.status, {
        approvers: document.getElementById("workflowDecisionApprovers").value.split(",").map((item) => item.trim()).filter(Boolean),
        comment: document.getElementById("workflowDecisionComment").value,
      });
      await renderApprovalInbox(env);
    });
  });
}

async function renderApprovalRejected(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadApprovalWorkbenchData(env);
  const rejectedEvents = asArray(data.workflow.events).filter((event) => {
    const action = String(event.action || "").toUpperCase();
    const toStatus = String(event.to_status || "").toUpperCase();
    return action.includes("REJECT") || action.includes("RETURN") || toStatus === "DRAFT";
  });
  env.outlet.innerHTML = `
    <section class="leaf-workbench approval-workbench" data-stage="approval" data-approval-stage="rejected" data-leaf-key="ws/appr:rejected">
      ${metrics([
        ["Rejected events", rejectedEvents.length],
        ["Open errors", data.counts.errors],
        ["Approval lines", asArray(data.workflow.approval_lines).length],
        ["Current status", statusLabel(data.currentStatus, env.locale)],
      ])}
      ${renderApprovalHeader(
        env,
        "ws/appr:rejected",
        "Rejected work",
        "Review return reasons, reopen validation or request approval again after corrections.",
        data,
        `<button class="secondary-btn compact" type="button" data-stage-route="ws/val:run">Run validation</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Return reasons</h2><p>Events that moved the work back for correction.</p></div>
          ${renderWorkflowEvents(rejectedEvents, "No rejected or returned workflow events.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Next action</h2><p>Use the related stage once corrections are complete.</p></div>
          ${table(["Action", "Route"], [
            row(["Fix validation issues", '<button class="secondary-btn compact" type="button" data-stage-route="ws/val:issues">Validation issues</button>']),
            row(["Request approval again", '<button class="primary-btn compact" type="button" data-stage-route="ws/appr:request">Approval request</button>']),
            row(["Open inbox", '<button class="secondary-btn compact" type="button" data-stage-route="ws/appr:inbox">Approval inbox</button>']),
          ])}
          ${renderApprovalLines(data.workflow)}
        </article>
      </section>
    </section>`;
  bindApprovalRouteButtons(env);
}

const PRINT_ROUTES = ["ws/print:preview", "ws/print:bulk", "ws/print:history"];

async function loadPrintWorkbenchData(env) {
  if (!requireWorkContext(env)) return;
  const root = workRoot(env);
  const selectedFormCode = env.context?.selectedPrintFormCode || env.context?.selectedFormCode || "FORM3";
  const [attachments, printHistory, preview] = await Promise.all([
    request(`${root}/forms/attachments`),
    request(`${root}/forms/print-history`).catch(() => []),
    request(`${root}/forms/${selectedFormCode}/preview`).catch(() => null),
  ]);
  const printable = statusIn(env.context?.status, ["APPROVED", "FILED", "AMENDED"]);
  const watermark = statusIn(env.context?.status, ["FILED"]) ? "FILED" : statusIn(env.context?.status, ["APPROVED"]) ? "APPROVED" : statusIn(env.context?.status, ["AMENDED"]) ? "AMENDED" : "DRAFT";
  return {
    root,
    selectedFormCode,
    attachments: asArray(attachments),
    printHistory: asArray(printHistory),
    preview,
    printable,
    watermark,
  };
}

function renderPrintHeader(env, activeLeaf, title, description, data, actions = "") {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge ${data.printable ? "ok" : "warn"}">${data.printable ? "PDF ready" : "Approval required"}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, PRINT_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function renderPrintMetrics(data, env) {
  return metrics([
    ["Printable forms", data.attachments.filter((item) => item.generated).length],
    ["Watermark", data.watermark],
    ["Print history", data.printHistory.length],
    ["Workflow status", statusLabel(env.context?.status || "DRAFT", env.locale)],
  ]);
}

function renderPrintableFormsTable(data, showBulkAction = false) {
  return table(["Form", "Status", "Validations", ""], data.attachments.map((item) => row([
    escapeHtml(item.form_code),
    escapeHtml(item.status),
    money.format(item.validation_count),
    `<button class="secondary-btn compact" data-download-form="${escapeHtml(item.form_code)}" type="button" ${data.printable ? "" : "disabled"}>PDF</button>`,
  ])), showBulkAction ? "No forms are available for bundle output." : "No form attachments.");
}

function renderPrintHistoryTable(printHistory) {
  return table(["Form", "Watermark", "Printed by", "File", "Printed at"], asArray(printHistory).map((item) => row([
    escapeHtml(item.form_code || "-"),
    escapeHtml(item.watermark || "-"),
    escapeHtml(item.printed_by || "-"),
    escapeHtml(item.file_name || "-"),
    escapeHtml(item.created_at || item.printed_at || "-"),
  ])), "No print history.");
}

function bindPrintRouteButtons(env) {
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
}

function bindPrintDownloads(data) {
  document.getElementById("printBundle")?.addEventListener("click", () => {
    if (!data.printable) return;
    downloadBinary(`${data.root}/forms/pdf-bundle/download`, "forms.zip");
  });
  document.querySelectorAll("[data-download-form]").forEach((button) => button.addEventListener("click", () => {
    if (!data.printable) return;
    downloadBinary(`${data.root}/forms/${button.dataset.downloadForm}/pdf`, `${button.dataset.downloadForm}.pdf`);
  }));
}

async function renderPrint(env) {
  return renderPrintPreview(env);
}

async function renderPrintPreview(env) {
  const data = await loadPrintWorkbenchData(env);
  if (!data) return;
  env.outlet.innerHTML = `
    <section class="leaf-workbench print-workbench" data-stage="print" data-print-stage="preview" data-leaf-key="ws/print:preview" data-print-form="${escapeHtml(data.selectedFormCode)}">
      ${renderPrintMetrics(data, env)}
      ${renderPrintHeader(
        env,
        "ws/print:preview",
        "PDF preview",
        data.printable ? "Preview the selected form with the current watermark and download an individual PDF." : "PDF preview is visible, but download is disabled until approval is complete.",
        data,
        `<button class="secondary-btn compact" data-download-form="${escapeHtml(data.selectedFormCode)}" type="button" ${data.printable ? "" : "disabled"}>Download PDF</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Form selector</h2><p>Choose the form used by the preview panel.</p></div>
          <label>Form
            <select id="selectedPrintFormCode">${data.attachments.map((item) => `<option value="${item.form_code}" ${item.form_code === data.selectedFormCode ? "selected" : ""}>${escapeHtml(item.form_code)}</option>`).join("")}</select>
          </label>
          ${renderPrintableFormsTable(data)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Preview snapshot</h2><p>Watermark: ${escapeHtml(data.watermark)}</p></div>
          ${data.preview ? table(["Field", "Value", "Source"], asArray(data.preview.fields).slice(0, 10).map((field) => row([
            escapeHtml(field.label),
            escapeHtml(formatWorkbenchValue(field.value)),
            escapeHtml(field.source_ref || field.source || "-"),
          ])), "No preview fields.") : '<p class="empty">No preview is available for the selected form.</p>'}
        </article>
      </section>
    </section>`;
  bindPrintRouteButtons(env);
  document.getElementById("selectedPrintFormCode")?.addEventListener("change", (event) => {
    env.setContext({ selectedPrintFormCode: event.target.value });
    renderPrintPreview(env);
  });
  bindPrintDownloads(data);
}

async function renderPrintBulk(env) {
  const data = await loadPrintWorkbenchData(env);
  if (!data) return;
  env.outlet.innerHTML = `
    <section class="leaf-workbench print-workbench" data-stage="print" data-print-stage="bulk" data-leaf-key="ws/print:bulk">
      ${renderPrintMetrics(data, env)}
      ${renderPrintHeader(
        env,
        "ws/print:bulk",
        "Bulk PDF output",
        data.printable ? "Generate a ZIP bundle or download selected form PDFs." : "Bundle output is disabled until approval is complete.",
        data,
        `<button id="printBundle" class="primary-btn compact" type="button" ${data.printable ? "" : "disabled"}>Print bundle</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Bundle targets</h2><p>Generated forms included in bulk output.</p></div>
          ${renderPrintableFormsTable(data, true)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Output readiness</h2><p>Bulk output depends on approval and generated form state.</p></div>
          ${table(["Check", "State"], [
            row(["Workflow status", statusLabel(env.context?.status || "DRAFT", env.locale)]),
            row(["Watermark", data.watermark]),
            row(["Generated forms", money.format(data.attachments.filter((item) => item.generated).length)]),
            row(["Download enabled", data.printable ? "Y" : "N"]),
          ])}
        </article>
      </section>
    </section>`;
  bindPrintRouteButtons(env);
  bindPrintDownloads(data);
}

async function renderPrintHistory(env) {
  const data = await loadPrintWorkbenchData(env);
  if (!data) return;
  env.outlet.innerHTML = `
    <section class="leaf-workbench print-workbench" data-stage="print" data-print-stage="history" data-leaf-key="ws/print:history">
      ${renderPrintMetrics(data, env)}
      ${renderPrintHeader(
        env,
        "ws/print:history",
        "Print history",
        "Review generated output files and the watermark used at download time.",
        data
      )}
      <article class="panel">
        <div class="panel-head"><h2>Print history</h2><p>Audit trail of downloaded output files.</p></div>
        ${renderPrintHistoryTable(data.printHistory)}
      </article>
    </section>`;
  bindPrintRouteButtons(env);
}

const EFILING_ROUTES = ["ws/file:precheck", "ws/file:generate", "ws/file:submit", "ws/file:done"];

async function loadEfilingWorkbenchData(env) {
  const root = workRoot(env);
  const [spec, precheck, history, latest] = await Promise.all([
    request(`${root}/efilings/format-spec`),
    request(`${root}/efilings/precheck`),
    request(`${root}/efilings`),
    request(`${root}/efilings/latest`).catch(() => null),
  ]);
  const filedLocked = statusIn(env.context?.status, ["FILED"]);
  const efileEnabled = statusIn(env.context?.status, ["APPROVED", "AMENDED"]);
  return {
    root,
    spec: asArray(spec),
    precheck: precheck || {},
    history: asArray(history),
    latest,
    latestHistory: asArray(history)[0] || latest || null,
    filedLocked,
    efileEnabled,
  };
}

function renderEfilingMetrics(data, locale = "ko") {
  return metrics([
    [uiText(locale, "레코드 수", "Record count"), data.precheck.record_count],
    [uiText(locale, "사전점검", "Precheck"), data.precheck.valid ? "READY" : "CHECK"],
    [uiText(locale, "체크섬", "Checksum"), data.precheck.checksum_preview],
    [uiText(locale, "파일", "Files"), data.history.length],
  ]);
}

function renderEfilingHeader(env, activeLeaf, title, description, data, actions = "") {
  const statusText = data.efileEnabled
    ? uiText(env.locale, "신고 가능", "Filing open")
    : data.filedLocked
      ? uiText(env.locale, "신고 잠금", "Filed locked")
      : uiText(env.locale, "승인 필요", "Approval required");
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge ${data.efileEnabled ? "ok" : "warn"}">${escapeHtml(statusText)}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, EFILING_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function renderEfilingPrecheckIssues(precheck, locale = "ko") {
  return table([uiText(locale, "코드", "Code"), uiText(locale, "심각도", "Severity"), uiText(locale, "메시지", "Message"), ""], asArray(precheck.issues).map((issue) => row([
    escapeHtml(issue.validation_code),
    escapeHtml(issue.severity),
    escapeHtml(issue.message),
    `<button class="secondary-btn compact" type="button" data-efile-jump="${escapeHtml(efilingIssueLeaf(issue))}">${escapeHtml(uiText(locale, "이동", "Jump"))}</button>`,
  ])), uiText(locale, "사전점검 이슈가 없습니다.", "No precheck issues."));
}

function renderEfilingLatestTable(data, locale = "ko") {
  const latestHistory = data.latestHistory;
  return latestHistory ? table([uiText(locale, "필드", "Field"), uiText(locale, "값", "Value")], [
    row([uiText(locale, "전자신고 ID", "E-filing id"), escapeHtml(latestHistory.efiling_id)]),
    row([uiText(locale, "상태", "Status"), escapeHtml(latestHistory.status)]),
    row([uiText(locale, "레코드", "Records"), escapeHtml(latestHistory.total_records || data.precheck.record_count)]),
    row([uiText(locale, "체크섬", "Checksum"), escapeHtml(latestHistory.checksum || data.precheck.checksum_preview)]),
    row([uiText(locale, "제출시각", "Submitted at"), escapeHtml(latestHistory.submitted_at || "-")]),
  ]) : `<p class="empty">${escapeHtml(uiText(locale, "먼저 전자신고 파일을 생성하세요.", "Generate a filing file first."))}</p>`;
}

function renderEfilingHistoryTable(history, locale = "ko") {
  return table(["ID", uiText(locale, "상태", "Status"), uiText(locale, "레코드", "Records"), uiText(locale, "체크섬", "Checksum"), ""], asArray(history).map((item) => row([
    escapeHtml(item.efiling_id),
    escapeHtml(item.status),
    escapeHtml(item.total_records),
    escapeHtml(item.checksum),
    `<button class="secondary-btn compact" data-download-efile="${item.efiling_id}" type="button">${escapeHtml(uiText(locale, "다운로드", "Download"))}</button>`,
  ])), uiText(locale, "전자신고 이력이 없습니다.", "No e-filing history."));
}

function bindEfilingRouteButtons(env) {
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
  document.querySelectorAll("[data-efile-jump]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.efileJump)));
  document.getElementById("goPrint")?.addEventListener("click", () => env.navigate("ws/print:preview"));
}

function bindEfilingDownloads(env, data) {
  document.getElementById("downloadLatestEfile")?.addEventListener("click", () => {
    if (!data.latestHistory) return;
    downloadBinary(`${routeRoot(env)}/efilings/${data.latestHistory.efiling_id}/file`, `efiling-${data.latestHistory.efiling_id}.txt`);
  });
  document.querySelectorAll("[data-download-efile]").forEach((button) => button.addEventListener("click", () => {
    downloadBinary(`${routeRoot(env)}/efilings/${button.dataset.downloadEfile}/file`, `efiling-${button.dataset.downloadEfile}.txt`);
  }));
}

function bindEfilingCreate(env, data, rerender) {
  document.getElementById("createEfile")?.addEventListener("click", async () => {
    const root = data.root;
    await request(`${root}/efilings`, { method: "POST", body: JSON.stringify({ max_attempts: 3, otp: document.getElementById("efileOtp")?.value || null }) });
    await rerender();
  });
}

function bindEfilingSubmit(env, data, rerender) {
  document.getElementById("submitEfile")?.addEventListener("click", async () => {
    const root = data.root;
    const latestHistory = data.latestHistory;
    if (!latestHistory) return;
    await request(`${root}/efilings/${latestHistory.efiling_id}/submit`, {
      method: "POST",
      body: JSON.stringify({ otp: document.getElementById("efileOtp")?.value || null, actor: env.auth.user.login_id }),
    });
    const by = await request(`${root}/status`, {
      method: "POST",
      body: JSON.stringify({ status: "FILED", actor: env.auth.user.login_id, approver: env.auth.user.login_id, comment: uiText(env.locale, "전자신고 제출", "e-filing submitted") }),
    });
    env.setContext({ status: by.status, progress: progressForStatus(by.status), lockMode: by.locked_at ? "LOCKED" : "OPEN" });
    await rerender();
  });
}

async function renderEfiling(env) {
  return renderEfilingPrecheck(env);
}

async function renderEfilingPrecheck(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadEfilingWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench efiling-workbench" data-stage="efiling" data-efile-stage="precheck" data-leaf-key="ws/file:precheck">
      ${renderEfilingMetrics(data, env.locale)}
      ${renderEfilingHeader(
        env,
        "ws/file:precheck",
        uiText(env.locale, "전자신고 사전점검", "E-filing precheck"),
        data.precheck.valid ? uiText(env.locale, "파일 생성 전 사전점검이 완료되었습니다.", "Precheck is ready for file generation.") : uiText(env.locale, "전자신고 파일 생성 전에 사전점검 이슈를 해결하세요.", "Resolve precheck issues before generating the filing file."),
        data,
        `<button id="goPrint" class="secondary-btn compact" type="button">${escapeHtml(uiText(env.locale, "출력 단계 열기", "Open print stage"))}</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "사전점검", "Precheck"))}</h2><p>${escapeHtml(uiText(env.locale, "파일 생성 전 이슈를 해결해야 합니다.", "Issues must be resolved before generation."))}</p></div>
          ${renderEfilingPrecheckIssues(data.precheck, env.locale)}
          <div class="button-row">
            <button class="primary-btn" type="button" data-stage-route="ws/file:generate" ${(data.efileEnabled && data.precheck.valid) ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "파일 생성으로 이동", "Continue to generation"))}</button>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "준비 상태", "Readiness"))}</h2><p>${escapeHtml(uiText(env.locale, "생성 가능 여부는 승인 상태와 사전점검 결과에 따라 결정됩니다.", "Generation depends on approval state and precheck result."))}</p></div>
          ${table([uiText(env.locale, "점검", "Check"), uiText(env.locale, "상태", "State")], [
            row([uiText(env.locale, "승인 상태", "Approval state"), data.efileEnabled ? "OPEN" : "BLOCKED"]),
            row([uiText(env.locale, "신고 잠금", "Filed lock"), data.filedLocked ? "LOCKED" : "OPEN"]),
            row([uiText(env.locale, "사전점검", "Precheck"), data.precheck.valid ? "READY" : "CHECK"]),
            row([uiText(env.locale, "레코드 수", "Record count"), money.format(data.precheck.record_count || 0)]),
          ])}
        </article>
      </section>
    </section>`;
  bindEfilingRouteButtons(env);
}

async function renderEfilingGenerate(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadEfilingWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench efiling-workbench" data-stage="efiling" data-efile-stage="generate" data-leaf-key="ws/file:generate">
      ${renderEfilingMetrics(data, env.locale)}
      ${renderEfilingHeader(
        env,
        "ws/file:generate",
        uiText(env.locale, "전자신고 파일 생성", "E-filing file generation"),
        data.efileEnabled ? uiText(env.locale, "사전점검 통과 후 전자신고 텍스트 파일을 생성합니다.", "Generate the text file after precheck passes.") : uiText(env.locale, "승인 완료 전에는 파일 생성이 차단됩니다.", "File generation remains blocked until approval is complete."),
        data,
        `<button id="createEfile" class="primary-btn compact" type="button" ${(data.efileEnabled && data.precheck.valid) ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "파일 생성", "Generate file"))}</button>`
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "생성 파일", "Generated file"))}</h2><p>${escapeHtml(uiText(env.locale, "최근 생성 파일과 체크섬 미리보기입니다.", "Latest generated file and checksum preview."))}</p></div>
          ${renderEfilingLatestTable(data, env.locale)}
          <div class="button-row">
            <button id="downloadLatestEfile" class="secondary-btn" type="button" ${data.latestHistory ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "최신 파일 다운로드", "Download latest"))}</button>
            <button class="primary-btn" type="button" data-stage-route="ws/file:submit" ${(data.efileEnabled && data.latestHistory) ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "제출로 이동", "Continue to submit"))}</button>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "파일 포맷", "Format spec"))}</h2><p>${escapeHtml(uiText(env.locale, "전자신고 레이아웃 대표 일부입니다.", "Representative excerpt of the e-file layout."))}</p></div>
          ${table([uiText(env.locale, "레코드", "Record"), uiText(env.locale, "필드", "Field"), uiText(env.locale, "길이", "Length"), uiText(env.locale, "원천", "Source")], data.spec.slice(0, 20).map((field) => row([
            escapeHtml(field.record_type),
            escapeHtml(field.field_name),
            escapeHtml(field.byte_length),
            escapeHtml(field.source_path || "-"),
          ])), uiText(env.locale, "포맷 행이 없습니다.", "No format spec rows."))}
        </article>
      </section>
    </section>`;
  bindEfilingRouteButtons(env);
  bindEfilingCreate(env, data, () => renderEfilingGenerate(env));
  bindEfilingDownloads(env, data);
}

async function renderEfilingSubmit(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadEfilingWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench efiling-workbench" data-stage="efiling" data-efile-stage="submit" data-leaf-key="ws/file:submit">
      ${renderEfilingMetrics(data, env.locale)}
      ${renderEfilingHeader(
        env,
        "ws/file:submit",
        uiText(env.locale, "전자신고 제출", "E-filing submission"),
        data.latestHistory ? uiText(env.locale, "최근 생성 파일을 제출하고 사업연도를 신고 완료로 잠급니다.", "Submit the latest generated file and lock the business year as filed.") : uiText(env.locale, "제출 전에 전자신고 파일을 생성하세요.", "Generate a filing file before submission."),
        data
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "제출", "Submission"))}</h2><p>${escapeHtml(uiText(env.locale, "최근 생성 파일과 접수 상태입니다.", "Latest generated file and receipt status."))}</p></div>
          ${env.auth.user.use_2fa ? `<label>OTP <input id="efileOtp" inputmode="numeric" autocomplete="one-time-code" placeholder="2FA code" /></label>` : ""}
          ${renderEfilingLatestTable(data, env.locale)}
          <div class="button-row">
            <button id="submitEfile" class="primary-btn" type="button" ${(data.efileEnabled && data.latestHistory) ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "제출 및 잠금", "Submit and lock"))}</button>
            <button id="downloadLatestEfile" class="secondary-btn" type="button" ${data.latestHistory ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "최신 파일 다운로드", "Download latest"))}</button>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "제출 게이트", "Submission gate"))}</h2><p>${escapeHtml(uiText(env.locale, "승인, 생성 파일, 잠금 상태를 확인합니다.", "Approval, generated file, and lock state."))}</p></div>
          ${table([uiText(env.locale, "게이트", "Gate"), uiText(env.locale, "상태", "State")], [
            row([uiText(env.locale, "승인", "Approval"), data.efileEnabled ? "OPEN" : "BLOCKED"]),
            row([uiText(env.locale, "생성 파일", "Generated file"), data.latestHistory ? "READY" : "MISSING"]),
            row([uiText(env.locale, "신고 잠금", "Filed lock"), data.filedLocked ? "LOCKED" : "OPEN"]),
            row([uiText(env.locale, "사전점검", "Precheck"), data.precheck.valid ? "READY" : "CHECK"]),
          ])}
        </article>
      </section>
    </section>`;
  bindEfilingRouteButtons(env);
  bindEfilingSubmit(env, data, () => renderEfilingSubmit(env));
  bindEfilingDownloads(env, data);
}

async function renderEfilingDone(env) {
  if (!requireWorkContext(env)) return;
  const data = await loadEfilingWorkbenchData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench efiling-workbench" data-stage="efiling" data-efile-stage="done" data-leaf-key="ws/file:done">
      ${renderEfilingMetrics(data, env.locale)}
      ${renderEfilingHeader(
        env,
        "ws/file:done",
        uiText(env.locale, "전자신고 접수", "E-filing receipt"),
        uiText(env.locale, "생성 파일, 제출 상태, 최종 접수 정보를 확인합니다.", "Review generated files, submission status, and final receipt information."),
        data
      )}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "최신 접수", "Latest receipt"))}</h2><p>${escapeHtml(uiText(env.locale, "가장 최근 생성 또는 제출된 신고 산출물입니다.", "Most recent generated or submitted filing artifact."))}</p></div>
          ${renderEfilingLatestTable(data, env.locale)}
          <div class="button-row">
            <button id="downloadLatestEfile" class="secondary-btn" type="button" ${data.latestHistory ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "최신 파일 다운로드", "Download latest"))}</button>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "신고 이력", "Filing history"))}</h2><p>${escapeHtml(uiText(env.locale, "생성 파일과 상태 변경 타임라인입니다.", "Generated files and status timeline."))}</p></div>
          ${renderEfilingHistoryTable(data.history, env.locale)}
        </article>
      </section>
    </section>`;
  bindEfilingRouteButtons(env);
  bindEfilingDownloads(env, data);
}

async function renderPostHistory(env) {
  const root = routeRoot(env);
  const workRootPath = hasWorkContext(env.context) ? workRoot(env) : null;
  const [years, efilings, notifications] = await Promise.all([
    request(`${root}/business-years`),
    workRootPath ? request(`${workRootPath}/efilings`).catch(() => []) : Promise.resolve([]),
    request(`${root}/notifications`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-history-workbench" data-stage="post-history">
      ${metrics([
        [uiText(env.locale, "사업연도", "Business years"), years.length],
        [uiText(env.locale, "신고완료 연도", "Filed years"), years.filter((by) => by.status === "FILED").length],
        [uiText(env.locale, "전자신고 이력", "E-filing history"), efilings.length],
        [uiText(env.locale, "알림", "Notifications"), notifications.length],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "사업연도 타임라인", "Business year timeline"))}</h2><p>${escapeHtml(uiText(env.locale, "신고완료, 수정신고, 진행 중 사업연도를 추적합니다.", "Track filed, amended, and open years."))}</p></div>
          ${table(["ID", uiText(env.locale, "연도", "Year"), uiText(env.locale, "상태", "Status"), uiText(env.locale, "잠금", "Lock"), ""], years.map((by) => row([
            escapeHtml(by.by_id),
            escapeHtml(by.year_label),
            pill(by.status, env.locale),
            escapeHtml(by.lock_mode || (by.locked_at ? "LOCKED" : "OPEN")),
            `<button class="secondary-btn compact" type="button" data-open-amend="${escapeHtml(by.by_id)}">${escapeHtml(uiText(env.locale, "수정신고", "Amend"))}</button>`,
          ])), uiText(env.locale, "사업연도가 없습니다.", "No business years."))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "신고 산출물", "Filed artifacts"))}</h2><p>${escapeHtml(uiText(env.locale, "현재 작업 컨텍스트의 최신 전자신고 산출물입니다.", "Latest e-filing output for the active work context."))}</p></div>
          ${table([uiText(env.locale, "접수 ID", "Receipt id"), uiText(env.locale, "상태", "Status"), uiText(env.locale, "레코드", "Records"), uiText(env.locale, "체크섬", "Checksum")], efilings.map((item) => row([
            escapeHtml(item.efiling_id),
            escapeHtml(item.status),
            escapeHtml(item.total_records),
            escapeHtml(item.checksum),
          ])), uiText(env.locale, "현재 작업 컨텍스트의 전자신고 이력이 없습니다.", "No e-filing history in the current work context."))}
          ${table([uiText(env.locale, "심각도", "Severity"), uiText(env.locale, "제목", "Title"), uiText(env.locale, "상태", "Status")], notifications.slice(0, 8).map((item) => row([
            escapeHtml(item.severity),
            escapeHtml(item.title),
            escapeHtml(item.status),
          ])), uiText(env.locale, "신고 후 알림이 없습니다.", "No post-filing notifications."))}
        </article>
      </section>
    </section>`;
  document.querySelectorAll("[data-open-amend]").forEach((button) => button.addEventListener("click", () => env.navigate("post/amend:unlock")));
}

const POST_AMEND_ROUTES = ["post/amend:unlock", "post/amend:version", "post/amend:diff", "post/amend:resubmit", "post/correction"];

async function loadPostAmendWorkbenchData(env) {
  if (!requireWorkContext(env)) return null;
  const root = workRoot(env);
  const [preview, versionMode] = await Promise.all([
    request(`${root}/amendment-preview`),
    request(`${root}/amendment-version-mode`).catch(() => ({ mode: "AMENDMENT", versions: [] })),
  ]);
  return { root, preview, versionMode };
}

function renderPostAmendMetrics(data) {
  const preview = data.preview || {};
  const versionMode = data.versionMode || {};
  const locale = data.locale || "ko";
  return metrics([
    [uiText(locale, "현재 상태", "Current status"), escapeHtml(preview.current_status || "-")],
    [uiText(locale, "잠금", "Locked"), preview.locked ? "Y" : "N"],
    [uiText(locale, "차이", "Differences"), asArray(preview.differences).length],
    [uiText(locale, "버전 모드", "Version mode"), escapeHtml(versionMode.mode || versionMode.version_mode || "AMENDMENT")],
  ]);
}

function renderPostAmendHeader(env, activeLeaf, title, description, data, actions = "") {
  const locked = data.preview?.locked;
  return `
    ${renderPostAmendMetrics(data)}
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge ${locked ? "warn" : "ok"}">${escapeHtml(locked ? uiText(env.locale, "신고 잠금", "Locked return") : uiText(env.locale, "수정신고 가능", "Unlocked for amendment"))}</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">
          ${renderStageRouteButtons(activeLeaf, POST_AMEND_ROUTES, env.locale)}
          ${actions}
        </div>
      </div>
    </article>`;
}

function renderPostAmendDifferenceTable(preview, locale = "ko") {
  return table([uiText(locale, "영역", "Area"), uiText(locale, "필드", "Field"), uiText(locale, "원신고", "Original"), uiText(locale, "현재", "Current"), uiText(locale, "설명", "Description")], asArray(preview?.differences).map((item) => row([
    escapeHtml(item.area),
    escapeHtml(item.field),
    escapeHtml(formatWorkbenchValue(item.original_value)),
    escapeHtml(formatWorkbenchValue(item.current_value)),
    escapeHtml(item.description || "-"),
  ])), uiText(locale, "수정신고 차이가 없습니다.", "No amendment differences."));
}

function renderPostAmendVersionTable(versionMode, locale = "ko") {
  return table([uiText(locale, "버전", "Version"), uiText(locale, "라벨", "Label"), uiText(locale, "잠금", "Locked")], asArray(versionMode?.versions).map((item) => row([
    escapeHtml(item.version),
    escapeHtml(item.label),
    item.locked ? `<span class="badge warn">${escapeHtml(uiText(locale, "잠금", "Locked"))}</span>` : `<span class="badge ok">${escapeHtml(uiText(locale, "열림", "Open"))}</span>`,
  ])), uiText(locale, "버전 메타데이터가 없습니다.", "No version metadata."));
}

function renderPostAmendVersionSelect(selected = "FILED_VERSION", locale = "ko") {
  const selectedValue = String(selected || "FILED_VERSION");
  return `
    <select id="unlockMode">
      <option value="FILED_VERSION" ${selectedValue === "FILED_VERSION" ? "selected" : ""}>${escapeHtml(uiText(locale, "신고 당시 버전", "Filed version"))}</option>
      <option value="CURRENT" ${selectedValue === "CURRENT" ? "selected" : ""}>${escapeHtml(uiText(locale, "현재 최신 버전", "Current latest"))}</option>
    </select>`;
}

function bindPostAmendRouteButtons(env) {
  document.querySelectorAll("[data-stage-route]").forEach((button) => button.addEventListener("click", () => env.navigate(button.dataset.stageRoute)));
}

function bindPostAmendUnlock(env, data, rerender) {
  document.getElementById("unlockForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const by = await request(`${data.root}/unlock`, {
      method: "POST",
      body: JSON.stringify({
        reason: document.getElementById("unlockReason").value,
        version_mode: document.getElementById("unlockMode").value,
        actor: env.auth.user.login_id,
      }),
    });
    env.setContext({
      byId: by.by_id,
      fy: String(by.year_label || env.context.fy || ""),
      period: `${by.start_date || ""} ~ ${by.end_date || ""}`,
      status: by.status,
      progress: progressForStatus(by.status),
      lockMode: by.lock_mode || "AMENDMENT_UNLOCK",
    });
    await rerender(env);
  });
}

function bindPostAmendResubmit(env, data, rerender) {
  document.getElementById("resubmitAmendment")?.addEventListener("click", async () => {
    const approvers = String(document.getElementById("resubmitApprovers")?.value || "")
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    await request(`${data.root}/resubmit`, {
      method: "POST",
      body: JSON.stringify({
        actor: env.auth.user.login_id,
        reason: document.getElementById("resubmitReason")?.value || uiText(env.locale, "수정신고 재제출", "amendment resubmission"),
        version_mode: document.getElementById("unlockMode")?.value || "CURRENT",
        approvers,
      }),
    });
    await rerender(env);
  });
  document.getElementById("goValidationFromAmend")?.addEventListener("click", () => env.navigate("ws/val:run"));
}

async function renderPostAmend(env) {
  await renderPostAmendUnlock(env);
}

async function renderPostAmendUnlock(env) {
  const data = await loadPostAmendWorkbenchData(env);
  if (!data) return;
  data.locale = env.locale;
  const activeLeaf = "post/amend:unlock";
  const selectedMode = data.versionMode.mode || data.versionMode.version_mode || "FILED_VERSION";
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend" data-amend-stage="unlock" data-leaf-key="post/amend:unlock">
      ${renderPostAmendHeader(env, activeLeaf, uiText(env.locale, "수정신고 잠금 해제", "Unlock for amendment"), uiText(env.locale, "기록 사유와 버전 기준을 남기고 신고 완료 사업연도를 다시 엽니다.", "Reopen a filed business year with a recorded reason and version basis."), data)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "잠금 해제 요청", "Unlock request"))}</h2><p>${escapeHtml(uiText(env.locale, "신고 완료 사업연도를 다시 열기 전에 수정 기준 버전을 선택합니다.", "Choose the amendment baseline before reopening the filed year."))}</p></div>
          <form id="unlockForm" class="stack">
            <label>${escapeHtml(uiText(env.locale, "버전 모드", "Version mode"))} ${renderPostAmendVersionSelect(selectedMode, env.locale)}</label>
            <label>${escapeHtml(uiText(env.locale, "사유", "Reason"))} <textarea id="unlockReason">${escapeHtml(uiText(env.locale, "수정신고 착수", "Amendment filing kickoff"))}</textarea></label>
            <button class="primary-btn" type="submit" ${data.preview.locked ? "" : "disabled"}>${escapeHtml(uiText(env.locale, "수정신고 잠금 해제", "Unlock for amendment"))}</button>
          </form>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "신고 잠금", "Filing lock"))}</h2><p>${escapeHtml(uiText(env.locale, "수정신고 작업 전 현재 신고 상태입니다.", "Current filing state before amendment work begins."))}</p></div>
          ${table([uiText(env.locale, "속성", "Property"), uiText(env.locale, "값", "Value")], [
            row([uiText(env.locale, "현재 상태", "Current status"), escapeHtml(data.preview.current_status || "-")]),
            row([uiText(env.locale, "잠금", "Locked"), data.preview.locked ? "Y" : "N"]),
            row([uiText(env.locale, "잠금 모드", "Lock mode"), escapeHtml(env.context.lockMode || "-")]),
            row([uiText(env.locale, "사업연도", "Business year"), escapeHtml(env.context.fy || "-")]),
          ])}
        </article>
      </section>
    </section>`;
  bindPostAmendRouteButtons(env);
  bindPostAmendUnlock(env, data, renderPostAmendUnlock);
}

async function renderPostAmendVersion(env) {
  const data = await loadPostAmendWorkbenchData(env);
  if (!data) return;
  data.locale = env.locale;
  const activeLeaf = "post/amend:version";
  const selectedMode = data.versionMode.mode || data.versionMode.version_mode || "FILED_VERSION";
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend" data-amend-stage="version" data-leaf-key="post/amend:version">
      ${renderPostAmendHeader(env, activeLeaf, uiText(env.locale, "수정신고 버전 기준", "Amendment version basis"), uiText(env.locale, "수정 재계산에 사용할 신고 당시 스냅샷과 현재 버전 선택지를 확인합니다.", "Review the filed snapshot and current-version options used for amendment recalculation."), data)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "버전 모드", "Version mode"))}</h2><p>${escapeHtml(uiText(env.locale, "선택한 사업연도에서 반환된 메타데이터입니다.", "Metadata returned for the selected business year."))}</p></div>
          ${table([uiText(env.locale, "속성", "Property"), uiText(env.locale, "값", "Value")], [
            row([uiText(env.locale, "모드", "Mode"), escapeHtml(selectedMode)]),
            row([uiText(env.locale, "현재 상태", "Current status"), escapeHtml(data.versionMode.current_status || data.preview.current_status || "-")]),
            row([uiText(env.locale, "원 사업연도", "Original business year"), escapeHtml(data.versionMode.original_by_id || "-")]),
            row([uiText(env.locale, "수정 순번", "Amendment sequence"), escapeHtml(data.versionMode.amendment_sequence || "-")]),
            row([uiText(env.locale, "사유", "Reason"), escapeHtml(data.versionMode.amendment_reason || "-")]),
          ])}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "버전 후보", "Version candidates"))}</h2><p>${escapeHtml(uiText(env.locale, "선택 가능한 법령/서식 버전 기준입니다.", "Available law/form version basis choices."))}</p></div>
          ${renderPostAmendVersionTable(data.versionMode, env.locale)}
        </article>
      </section>
    </section>`;
  bindPostAmendRouteButtons(env);
}

async function renderPostAmendDiff(env) {
  const data = await loadPostAmendWorkbenchData(env);
  if (!data) return;
  data.locale = env.locale;
  const activeLeaf = "post/amend:diff";
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend" data-amend-stage="diff" data-leaf-key="post/amend:diff">
      ${renderPostAmendHeader(env, activeLeaf, uiText(env.locale, "수정신고 차이 보고서", "Amendment difference report"), uiText(env.locale, "재제출 전 현재 작업 데이터와 원 신고 기준을 비교합니다.", "Compare current work data with the filed baseline before resubmission."), data)}
      <article class="panel">
        <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "차이 보고서", "Difference report"))}</h2><p>${escapeHtml(uiText(env.locale, "원 신고 기준 대비 현재 상태입니다.", "Current state compared with the filed baseline."))}</p></div>
        ${renderPostAmendDifferenceTable(data.preview, env.locale)}
      </article>
    </section>`;
  bindPostAmendRouteButtons(env);
}

async function renderPostAmendResubmit(env) {
  const data = await loadPostAmendWorkbenchData(env);
  if (!data) return;
  data.locale = env.locale;
  const activeLeaf = "post/amend:resubmit";
  const selectedMode = data.versionMode.mode || data.versionMode.version_mode || "CURRENT";
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend" data-amend-stage="resubmit" data-leaf-key="post/amend:resubmit">
      ${renderPostAmendHeader(env, activeLeaf, uiText(env.locale, "수정신고 재제출", "Resubmit amendment"), uiText(env.locale, "선택한 버전 기준으로 수정신고서를 다시 검토 단계로 보냅니다.", "Move the amended return back into review with the selected version basis."), data)}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "재제출 요청", "Resubmission request"))}</h2><p>${escapeHtml(uiText(env.locale, "수정신고서를 검토 및 전자신고 준비 단계로 보냅니다.", "Send the amended return to review and e-filing preparation."))}</p></div>
          <div class="stack">
            <label>${escapeHtml(uiText(env.locale, "버전 모드", "Version mode"))} ${renderPostAmendVersionSelect(selectedMode, env.locale)}</label>
            <label>${escapeHtml(uiText(env.locale, "사유", "Reason"))} <textarea id="resubmitReason">${escapeHtml(uiText(env.locale, "수정신고 재제출", "amendment resubmission"))}</textarea></label>
            <label>${escapeHtml(uiText(env.locale, "승인자", "Approvers"))} <input id="resubmitApprovers" placeholder="${escapeHtml(uiText(env.locale, "쉼표로 구분한 로그인 ID", "comma-separated login IDs"))}" /></label>
            <div class="button-row">
              <button id="resubmitAmendment" class="primary-btn" type="button">${escapeHtml(uiText(env.locale, "수정신고 재제출", "Resubmit amendment"))}</button>
              <button id="goValidationFromAmend" class="secondary-btn" type="button">${escapeHtml(uiText(env.locale, "검증 열기", "Open validation"))}</button>
            </div>
          </div>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "검토 점검", "Review checks"))}</h2><p>${escapeHtml(uiText(env.locale, "수정신고 검토 묶음에 포함될 차이입니다.", "Differences that will travel with the amendment review packet."))}</p></div>
          ${renderPostAmendDifferenceTable(data.preview, env.locale)}
        </article>
      </section>
    </section>`;
  bindPostAmendRouteButtons(env);
  bindPostAmendResubmit(env, data, renderPostAmendResubmit);
}

async function renderPostCorrection(env) {
  const activeLeaf = "post/correction";
  const root = routeRoot(env);
  const claims = await request(`${root}/correction-claims`).catch(() => []);
  env.outlet.innerHTML = `
    <section class="leaf-workbench post-amend-workbench" data-stage="post-amend" data-amend-stage="correction" data-leaf-key="post/correction">
      ${metrics([
        [uiText(env.locale, "청구", "Claims"), asArray(claims).length],
        [uiText(env.locale, "초안", "Drafts"), asArray(claims).filter((item) => String(item.status).toUpperCase() === "DRAFT").length],
        [uiText(env.locale, "환급 합계", "Refund total"), money.format(asArray(claims).reduce((sum, item) => sum + Number(item.refund_amount || 0), 0))],
        [uiText(env.locale, "테넌트", "Tenant"), escapeHtml(env.tenant)],
      ])}
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge neutral">${escapeHtml(uiText(env.locale, "경정청구", "Correction claim"))}</span>
            <h2>${escapeHtml(uiText(env.locale, "경정청구 요청", "Correction request"))}</h2>
            <p>${escapeHtml(uiText(env.locale, "현재 작업 컨텍스트 밖에서 환급 중심 경정청구를 작성하고 추적합니다.", "Prepare and track refund-oriented correction claims outside the active work context."))}</p>
          </div>
          <div class="button-row">${renderStageRouteButtons(activeLeaf, POST_AMEND_ROUTES, env.locale)}</div>
        </div>
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "경정청구", "Correction claims"))}</h2><p>${escapeHtml(uiText(env.locale, "테넌트에 저장된 경정청구 요청입니다.", "Saved correction requests for the tenant."))}</p></div>
          ${table([uiText(env.locale, "청구", "Claim"), uiText(env.locale, "상태", "Status"), uiText(env.locale, "환급", "Refund")], asArray(claims).map((item) => row([
            escapeHtml(item.claim_id),
            escapeHtml(item.status),
            money.format(Number(item.refund_amount || 0)),
          ])), uiText(env.locale, "경정청구가 없습니다.", "No correction claims."))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(uiText(env.locale, "새 경정청구", "New correction request"))}</h2><p>${escapeHtml(uiText(env.locale, "예상 환급액이 있는 경정청구 초안을 기록합니다.", "Record a draft claim with expected refund amount."))}</p></div>
          <form id="correctionClaimForm" class="stack">
            <label>${escapeHtml(uiText(env.locale, "청구 유형", "Claim type"))} <input id="correctionClaimType" value="${escapeHtml(uiText(env.locale, "환급", "refund"))}" /></label>
            <label>${escapeHtml(uiText(env.locale, "환급 예상액", "Refund amount"))} <input id="correctionRefundAmount" inputmode="numeric" value="1200000" /></label>
            <label>${escapeHtml(uiText(env.locale, "사유", "Reason"))} <textarea id="correctionReason">${escapeHtml(uiText(env.locale, "경정청구 초안", "Correction claim draft"))}</textarea></label>
            <button class="primary-btn" type="submit">${escapeHtml(uiText(env.locale, "경정청구 저장", "Save correction request"))}</button>
            <p id="correctionSaveStatus" class="muted"></p>
          </form>
        </article>
      </section>
    </section>`;
  bindPostAmendRouteButtons(env);
  document.getElementById("correctionClaimForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const saved = await request(`${root}/correction-claims`, {
      method: "POST",
      body: JSON.stringify({
        claim_type: document.getElementById("correctionClaimType")?.value || "refund",
        refund_amount: Number(document.getElementById("correctionRefundAmount")?.value || 0),
        reason: document.getElementById("correctionReason")?.value || "",
      }),
    });
    const status = document.getElementById("correctionSaveStatus");
    if (status) status.textContent = `${uiText(env.locale, "저장된 청구", "Saved claim")} ${saved.claim_id || ""}`;
  });
}

async function renderAlerts(env) {
  const root = routeRoot(env);
  const notifications = await request(`${root}/notifications`);
  env.outlet.innerHTML = `
    <section class="panel">
      <div class="panel-head"><h2>알림 센터</h2></div>
      ${table(["등급", "제목", "상태", ""], notifications.map((item) => row([
        `<span class="badge ${item.severity === "WARN" ? "warn" : "info"}">${escapeHtml(item.severity)}</span>`,
        escapeHtml(item.title),
        escapeHtml(item.status),
        `<button class="secondary-btn compact" data-read-notification="${item.notification_id}" type="button">읽음</button>`,
      ])))}
    </section>`;
  document.querySelectorAll("[data-read-notification]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${root}/notifications/${button.dataset.readNotification}`, { method: "PATCH", body: JSON.stringify({ status: "READ" }) });
      await renderAlerts(env);
    });
  });
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderYearCompare(env) {
  const rows = await request(`${routeRoot(env)}/reports/year-comparison`);
  const max = Math.max(1, ...rows.map((item) => Math.abs(Number(item.total_adjustment_amount || 0))));
  env.outlet.innerHTML = `
    <section class="panel" data-report-leaf="year-compare">
      <div class="panel-head"><h2>사업연도 비교</h2></div>
      <div class="mini-chart">
        ${rows.map((item) => chartRow(`${item.customer_id} / ${item.year_label}`, item.total_adjustment_amount, max)).join("")}
      </div>
      ${table(["고객사", "사업연도", "상태", "조정합계", "유보"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), escapeHtml(item.status), money.format(item.total_adjustment_amount), money.format(item.reserve_count)])))}
    </section>`;
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderTaxBurden(env) {
  const [rows, industry] = await Promise.all([
    request(`${routeRoot(env)}/reports/tax-burden`),
    request(`${routeRoot(env)}/reports/industry-statistics`).catch(() => []),
  ]);
  const max = Math.max(1, ...rows.map((item) => Number(item.total_tax_due || 0)));
  env.outlet.innerHTML = `
    <section class="grid two" data-report-leaf="tax-burden">
      <article class="panel">
        <div class="panel-head"><h2>세부담 분석</h2></div>
        <div class="mini-chart">${rows.map((item) => chartRow(`${item.customer_id} / ${item.year_label}`, item.total_tax_due, max)).join("")}</div>
        ${table(["고객사", "사업연도", "과세표준", "총부담세액", "실효세율"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), money.format(item.taxable_income), money.format(item.total_tax_due), `${(item.effective_tax_rate_bps / 100).toFixed(2)}%`])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Industry Statistics</h2></div>
        ${table(["Industry", "SME", "Customers", "Avg tax"], industry.map((item) => row([
          escapeHtml(item.industry_code),
          item.is_sme ? "Y" : "N",
          money.format(item.customer_count),
          money.format(item.average_tax_due),
        ])))}
      </article>
    </section>`;
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderReserveTrend(env) {
  const root = routeRoot(env);
  const [rows, lossExpiry, userReports] = await Promise.all([
    request(`${root}/reports/reserve-trend`),
    request(`${root}/reports/loss-expiry`).catch(() => []),
    request(`${root}/reports/user-defined`).catch(() => []),
  ]);
  const max = Math.max(1, ...rows.map((item) => Number(item.amount || 0)));
  env.outlet.innerHTML = `
    <section class="grid" data-report-leaf="reserve-trend">
      <article class="panel">
        <div class="panel-head"><h2>유보 잔액 추이</h2></div>
        <div class="mini-chart">${rows.map((item) => chartRow(`${item.reserve_code} / ${item.year_label}`, item.amount, max)).join("")}</div>
        ${table(["고객사", "사업연도", "유보코드", "구분", "금액"], rows.map((item) => row([escapeHtml(item.customer_id), escapeHtml(item.year_label), escapeHtml(item.reserve_code), escapeHtml(item.direction), money.format(item.amount)])))}
      </article>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Loss Expiry</h2></div>
          ${table(["Customer", "Origin", "Expires", "Remaining"], lossExpiry.map((item) => row([
            escapeHtml(item.customer_name),
            escapeHtml(item.origin_year),
            escapeHtml(item.expires_year),
            money.format(item.remaining_amount),
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>User Reports</h2><button id="createLossReport" class="primary-btn compact" type="button">Loss report</button></div>
          ${table(["Name", "Source", "Updated"], userReports.map((item) => row([
            escapeHtml(item.report_name),
            escapeHtml(item.source),
            escapeHtml(item.updated_at),
          ])))}
        </article>
      </section>
    </section>`;
  document.getElementById("createLossReport").addEventListener("click", async () => {
    await request(`${root}/reports/user-defined`, {
      method: "POST",
      body: JSON.stringify({ report_name: `${uiText(env.locale, "결손금 만료", "Loss expiry")} ${today()}`, source: "LOSS_EXPIRY", columns: ["customer_name", "origin_year", "expires_year", "remaining_amount"], filters: {} }),
    });
    await renderReserveTrend(env);
  });
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderLossExpiryReport(env) {
  const rows = await request(`${routeRoot(env)}/reports/loss-expiry`);
  const max = Math.max(1, ...rows.map((item) => Number(item.remaining_amount || 0)));
  env.outlet.innerHTML = `
    <section class="grid" data-report-leaf="loss-expiry">
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Report</span>
            <h2>이월결손금 만료 예측</h2>
            <p class="empty">만료연도별 잔액과 고객사별 대응 대상을 분리해서 확인합니다.</p>
          </div>
          <button class="secondary-btn compact" type="button" data-report-route="report:reserve-trend">유보 추이</button>
        </div>
        <div class="mini-chart">
          ${rows.map((item) => chartRow(`${item.customer_name || item.customer_id} / ${item.expires_year}`, item.remaining_amount, max)).join("")}
        </div>
        ${table(["고객사", "발생연도", "만료연도", "잔액", "만료까지"], rows.map((item) => row([
          escapeHtml(item.customer_name || item.customer_id || "-"),
          escapeHtml(item.origin_year || "-"),
          escapeHtml(item.expires_year || "-"),
          money.format(item.remaining_amount || 0),
          `${money.format(item.years_until_expiry || 0)}년`,
        ])), "만료 예정 이월결손금이 없습니다.")}
      </article>
    </section>`;
  env.outlet.querySelector("[data-report-route]")?.addEventListener("click", (event) => env.navigate(event.currentTarget.dataset.reportRoute));
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderIndustryStatsReport(env) {
  const rows = await request(`${routeRoot(env)}/reports/industry-statistics`);
  const max = Math.max(1, ...rows.map((item) => Number(item.total_tax_due || item.average_tax_due || item.effective_tax_rate_bps || 0)));
  env.outlet.innerHTML = `
    <section class="grid" data-report-leaf="industry-stats">
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Report</span>
            <h2>업종별 통계</h2>
            <p class="empty">업종, 중소기업 여부, 고객사 수, 평균 세액을 별도 화면에서 비교합니다.</p>
          </div>
          <button class="secondary-btn compact" type="button" data-report-route="report:tax-burden">세부담 분석</button>
        </div>
        <div class="mini-chart">
          ${rows.map((item) => chartRow(`${item.industry_code}${item.is_sme ? " / SME" : ""}`, item.total_tax_due || item.average_tax_due || item.effective_tax_rate_bps, max)).join("")}
        </div>
        ${table(["업종", "중소기업", "고객사", "사업연도", "총 세액", "평균 세액"], rows.map((item) => row([
          escapeHtml(item.industry_code || "-"),
          item.is_sme ? "Y" : "N",
          money.format(item.customer_count || item.company_count || 0),
          money.format(item.business_year_count || 0),
          money.format(item.total_tax_due || 0),
          money.format(item.average_tax_due || 0),
        ])), "업종별 통계 데이터가 없습니다.")}
      </article>
    </section>`;
  env.outlet.querySelector("[data-report-route]")?.addEventListener("click", (event) => env.navigate(event.currentTarget.dataset.reportRoute));
  localizeRenderedOutlet(env.outlet, env.locale);
}

async function renderCustomReports(env) {
  const root = routeRoot(env);
  const [reports, userReports] = await Promise.all([
    request(`${root}/reports/custom`).catch(() => []),
    request(`${root}/reports/user-defined`).catch(() => []),
  ]);
  const rows = [...asArray(reports), ...asArray(userReports)];
  env.outlet.innerHTML = `
    <section class="grid" data-report-leaf="custom">
      <article class="panel">
        <div class="panel-head">
          <div>
            <span class="badge info">Report</span>
            <h2>사용자 정의 리포트</h2>
            <p class="empty">저장된 리포트 정의를 조회하고 대표 리포트를 생성해 즉시 실행합니다.</p>
          </div>
          <button id="createCustomReport" class="primary-btn compact" type="button">리포트 생성</button>
        </div>
        ${table(["리포트", "소스", "컬럼", "상태", ""], rows.map((item) => row([
          escapeHtml(item.report_name || item.name || `Report ${item.report_id || ""}`),
          escapeHtml(item.source || "CUSTOM"),
          money.format(item.column_count || asArray(item.columns).length || 0),
          item.active === false ? "INACTIVE" : "ACTIVE",
          `<button class="secondary-btn compact" type="button" data-open-custom-report="${escapeHtml(item.report_id || "")}">열기</button>`,
        ])), "사용자 정의 리포트가 없습니다.")}
        ${renderLeafActionResult()}
      </article>
    </section>`;
  env.outlet.querySelector("#createCustomReport")?.addEventListener("click", async () => {
    await request(`${root}/reports/user-defined`, {
      method: "POST",
      body: JSON.stringify({
        report_name: `${uiText(env.locale, "사용자 리포트", "Custom report")} ${today()}`,
        source: "TAX_BURDEN",
        columns: ["customer_name", "year_label", "total_tax_due"],
        filters: {},
      }),
    });
    await renderCustomReports(env);
  });
  env.outlet.querySelectorAll("[data-open-custom-report]").forEach((button) => {
    button.addEventListener("click", async () => {
      const reportId = button.dataset.openCustomReport;
      if (!reportId) return;
      const detail = await request(`${root}/reports/custom/${encodeURIComponent(reportId)}`);
      setLeafActionMessage(`${detail.report_id || reportId}: ${asArray(detail.rows).length} rows`, false, env.locale);
    });
  });
  localizeRenderedOutlet(env.outlet, env.locale);
}

function chartRow(label, value, max) {
  const width = Math.min(100, Math.round((Math.abs(Number(value || 0)) / max) * 100));
  return `<div class="chart-row"><strong>${escapeHtml(label)}</strong><div class="bar-track"><span style="width:${width}%"></span></div><span>${money.format(value || 0)}</span></div>`;
}

async function renderAdminTenants(env) {
  const tenants = await request("/api/tenants");
  const canManage = env.auth?.user?.roles?.includes("SUPER_ADMIN");
  const locale = env.locale;
  const planCounts = tenants.reduce((acc, item) => {
    acc[item.plan || "STANDARD"] = (acc[item.plan || "STANDARD"] || 0) + 1;
    return acc;
  }, {});
  env.outlet.innerHTML = `
    <section class="leaf-workbench leaf-typology" data-typology="grid" data-leaf-key="admin/tenant:list">
      <section class="panel leaf-summary" data-leaf-block="summary">
      <div class="panel-head">
        <div><span class="badge info">${escapeHtml(t(locale, "leaf.workbench"))}</span><h2>${escapeHtml(t(locale, "route.admin.tenant.list"))}</h2><p>admin/tenant:list / admin:READ</p></div>
      </div>
      ${metrics([
        [t(locale, "field.tenantName"), tenants.length],
        [statusLabel("ACTIVE", locale), tenants.filter((item) => item.status === "ACTIVE").length],
        [statusLabel("SUSPENDED", locale), tenants.filter((item) => item.status === "SUSPENDED").length],
        ["ENTERPRISE", planCounts.ENTERPRISE || 0],
      ])}
      </section>
      <article class="panel leaf-table" data-leaf-block="table">
        <div class="panel-head">
          <div><h2>${escapeHtml(t(locale, "route.admin.tenant.list"))}</h2><p>${escapeHtml(t(locale, "leaf.count", { count: tenants.length, description: t(locale, "typology.grid.description") }))}</p></div>
          <div class="panel-head-actions" data-leaf-block="toolbar">
          <div data-leaf-block="filters">
            <label>${escapeHtml(t(locale, "common.search"))} <input type="search" data-tenant-filter="q" placeholder="${escapeHtml(t(locale, "field.tenantCode"))}/${escapeHtml(t(locale, "field.tenantName"))}" /></label>
            <label>${escapeHtml(t(locale, "field.status"))} <select data-tenant-filter="status"><option value="ALL">${escapeHtml(statusLabel("ALL", locale))}</option><option value="ACTIVE">${escapeHtml(statusLabel("ACTIVE", locale))}</option><option value="SUSPENDED">${escapeHtml(statusLabel("SUSPENDED", locale))}</option><option value="CLOSED">${escapeHtml(statusLabel("CLOSED", locale))}</option></select></label>
            <button class="secondary-btn compact" type="button" data-tenant-filter-reset>${escapeHtml(t(locale, "common.reset"))}</button>
          </div>
            <button class="primary-btn compact" type="submit" form="tenantForm" ${canManage ? "" : "disabled"}>${escapeHtml(t(locale, "common.addPrefix"))}</button>
          </div>
        </div>
        ${table([t(locale, "field.code"), t(locale, "field.name"), t(locale, "field.status"), t(locale, "field.plan"), t(locale, "field.maxUsers"), t(locale, "common.actions")], tenants.map((item) => row([
          escapeHtml(item.tenant_code),
          escapeHtml(item.tenant_name),
          canManage ? `<select data-tenant-status="${escapeHtml(item.tenant_code)}"><option value="ACTIVE" ${item.status === "ACTIVE" ? "selected" : ""}>${escapeHtml(statusLabel("ACTIVE", locale))}</option><option value="SUSPENDED" ${item.status === "SUSPENDED" ? "selected" : ""}>${escapeHtml(statusLabel("SUSPENDED", locale))}</option><option value="CLOSED" ${item.status === "CLOSED" ? "selected" : ""}>${escapeHtml(statusLabel("CLOSED", locale))}</option></select>` : pill(item.status, locale),
          canManage ? `<select data-tenant-plan="${escapeHtml(item.tenant_code)}"><option ${item.plan === "FREE" ? "selected" : ""}>FREE</option><option ${item.plan === "STANDARD" ? "selected" : ""}>STANDARD</option><option ${item.plan === "PRO" ? "selected" : ""}>PRO</option><option ${item.plan === "ENTERPRISE" ? "selected" : ""}>ENTERPRISE</option></select>` : escapeHtml(item.plan || "STANDARD"),
          escapeHtml(item.max_users),
          canManage ? `<button class="secondary-btn compact" type="button" data-save-tenant="${escapeHtml(item.tenant_code)}">${escapeHtml(t(locale, "common.save"))}</button>` : "",
        ])))}
      </article>
      <article class="panel tenant-create-panel">
        <div class="panel-head"><h2>${escapeHtml(t(locale, "common.add"))} ${escapeHtml(t(locale, "field.tenantName"))}</h2><span class="badge info">${escapeHtml(t(locale, "common.addPrefix"))}</span></div>
        <form id="tenantForm" class="stack">
          <label>${escapeHtml(t(locale, "field.code"))} <input id="tenantCodeInput" value="tenant${Date.now().toString(36).slice(-4)}" /></label>
          <label>${escapeHtml(t(locale, "field.name"))} <input id="tenantNameInput" value="${escapeHtml(t(locale, "common.add"))} ${escapeHtml(t(locale, "field.tenantName"))}" /></label>
          <label>${escapeHtml(t(locale, "field.bizRegNo"))} <input id="tenantBizInput" value="1234567890" /></label>
          <label>${escapeHtml(t(locale, "field.plan"))} <select id="tenantPlanInput"><option>STANDARD</option><option>PRO</option><option>ENTERPRISE</option><option>FREE</option></select></label>
          <label>${escapeHtml(t(locale, "field.allowedIps"))} <input id="tenantAllowedIpsInput" placeholder="203.0.113.10/32" /></label>
          <label>${escapeHtml(t(locale, "field.contractStart"))} <input id="tenantStartInput" type="date" value="${today()}" /></label>
        </form>
      </article>
    </section>`;
  const applyTenantFilters = () => {
    const query = document.querySelector('[data-tenant-filter="q"]')?.value.toLowerCase() || "";
    const status = document.querySelector('[data-tenant-filter="status"]')?.value || "ALL";
    document.querySelectorAll('[data-leaf-block="table"] tbody tr').forEach((tr) => {
      const text = tr.textContent.toLowerCase();
      const rowStatus = tr.querySelector("[data-tenant-status]")?.value || tr.children[2]?.textContent.trim() || "";
      tr.style.display = (!query || text.includes(query)) && (status === "ALL" || rowStatus === status) ? "" : "none";
    });
  };
  document.querySelectorAll("[data-tenant-filter]").forEach((control) => control.addEventListener("input", applyTenantFilters));
  document.querySelector("[data-tenant-filter-reset]")?.addEventListener("click", () => {
    const q = document.querySelector('[data-tenant-filter="q"]');
    const status = document.querySelector('[data-tenant-filter="status"]');
    if (q) q.value = "";
    if (status) status.value = "ALL";
    applyTenantFilters();
  });
  document.querySelectorAll("[data-save-tenant]").forEach((button) => {
    button.addEventListener("click", async () => {
      const code = button.dataset.saveTenant;
      const status = document.querySelector(`[data-tenant-status="${CSS.escape(code)}"]`)?.value;
      const plan = document.querySelector(`[data-tenant-plan="${CSS.escape(code)}"]`)?.value;
      await request(`/api/tenants/${code}/status`, { method: "PATCH", body: JSON.stringify({ status }) });
      await request(`/api/tenants/${code}/plan`, { method: "PATCH", body: JSON.stringify({ plan }) });
      await renderAdminTenants(env);
    });
  });
  document.getElementById("tenantForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    if (!canManage) return;
    await request("/api/tenants", {
      method: "POST",
      body: JSON.stringify({ tenant_code: document.getElementById("tenantCodeInput").value, tenant_name: document.getElementById("tenantNameInput").value, biz_reg_no: document.getElementById("tenantBizInput").value, contract_start: document.getElementById("tenantStartInput").value, contract_end: null, allowed_ips: document.getElementById("tenantAllowedIpsInput").value || null, max_users: 10, plan: document.getElementById("tenantPlanInput").value }),
    });
    await renderAdminTenants(env);
  });
}

async function renderAdminCustomers(env) {
  return renderAdminCustomersWorkbench(env);
}

const ADMIN_CUSTOMER_ROUTES = ["admin/cust:list", "admin/cust:by-master", "admin/cust:agent"];

function adminCustomerHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Customer master administration</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_CUSTOMER_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminCustomerData(env) {
  const root = routeRoot(env);
  const [customers, businessYears, taxAgents] = await Promise.all([
    request(`${root}/customers`).catch(() => []),
    request(`${root}/business-years`).catch(() => []),
    request(`${root}/tax-agents`).catch(() => []),
  ]);
  return { root, customers, businessYears, taxAgents };
}

function adminCustomerDisplayName(customers, customerId) {
  const customer = customers.find((item) => String(item.customer_id) === String(customerId));
  return customer?.customer_name || customer?.customer_code || customerId || "-";
}

function adminCustomerOptions(customers, selectedId) {
  return customers.map((item) => `
    <option value="${escapeHtml(item.customer_id)}" ${String(item.customer_id) === String(selectedId) ? "selected" : ""}>${escapeHtml(item.customer_name)} / ${escapeHtml(item.customer_code)}</option>`).join("");
}

async function renderAdminCustomerList(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerData(env);
  const { root } = data;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-list" data-leaf-key="admin/cust:list">
      ${adminCustomerHeader(env, activeLeaf, "Customer registry", "Maintain corporate taxpayer master data, registration numbers, SME flags, and enabled work scopes.")}
      ${metrics([
        ["Customers", money.format(data.customers.length)],
        ["Active", money.format(data.customers.filter((item) => item.status !== "INACTIVE").length)],
        ["SME", money.format(data.customers.filter((item) => item.is_sme).length)],
        ["Work scopes", money.format(new Set(data.customers.flatMap((item) => asArray(item.work_scopes))).size)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Customer master</h2><p>Tenant customer records and filing work scope coverage.</p></div>
          ${table(["Code", "Customer", "Biz no.", "Industry", "SME", "Work scopes", "Status"], data.customers.map((item) => row([
            escapeHtml(item.customer_code),
            escapeHtml(item.customer_name),
            escapeHtml(item.biz_reg_no),
            escapeHtml(item.industry_code || "-"),
            item.is_sme ? "Y" : "N",
            escapeHtml(asArray(item.work_scopes).join(", ") || "-"),
            escapeHtml(item.status || "ACTIVE"),
          ])), "No customers registered.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Register customer</h2><p>Create a customer master record with default corporate tax work scopes.</p></div>
          <form id="customerForm" class="stack">
            <label>Code <input id="custCode" value="C${Date.now().toString(36).slice(-4).toUpperCase()}" /></label>
            <label>Name <input id="custName" value="신규 고객사" /></label>
            <label>Business number <input id="custBiz" value="2208112345" /></label>
            <label>Industry code <input id="custIndustry" value="62010" /></label>
            <label>SME <select id="custSme"><option value="true">Y</option><option value="false">N</option></select></label>
            <label>Work scopes <input id="custScopes" value="INFO, ADJUST, FORM, VALIDATE, APPROVE, PRINT, EFILE, POST" /></label>
            <button class="primary-btn" type="submit">Register customer</button>
          </form>
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#customerForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/customers`, {
      method: "POST",
      body: JSON.stringify({
        customer_code: env.outlet.querySelector("#custCode").value,
        customer_name: env.outlet.querySelector("#custName").value,
        biz_reg_no: env.outlet.querySelector("#custBiz").value,
        corp_reg_no: null,
        industry_code: env.outlet.querySelector("#custIndustry").value || null,
        is_sme: env.outlet.querySelector("#custSme").value === "true",
        work_scopes: env.outlet.querySelector("#custScopes").value.split(",").map((item) => item.trim().toUpperCase()).filter(Boolean),
      }),
    });
    setLeafActionMessage("Customer registered.", false, env.locale);
    await renderAdminCustomerList(env);
  });
}

async function renderAdminBusinessYearMaster(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerData(env);
  const { root } = data;
  const firstCustomer = data.customers[0] || null;
  const currentYear = new Date().getFullYear();
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="business-year-master" data-leaf-key="admin/cust:by-master">
      ${adminCustomerHeader(env, activeLeaf, "Business-year master", "Create and review customer business-year workspaces, filing periods, statuses, and lock modes.")}
      ${metrics([
        ["Business years", money.format(data.businessYears.length)],
        ["Customers", money.format(data.customers.length)],
        ["Filed", money.format(data.businessYears.filter((item) => item.status === "FILED").length)],
        ["Locked", money.format(data.businessYears.filter((item) => item.locked_at).length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Business-year registry</h2><p>Filing workspace periods and current lifecycle state.</p></div>
          ${table(["Customer", "Year", "Period", "Status", "Lock", "Updated"], data.businessYears.map((item) => row([
            escapeHtml(adminCustomerDisplayName(data.customers, item.customer_id)),
            escapeHtml(item.year_label),
            `${escapeHtml(formatDate(item.start_date))} ~ ${escapeHtml(formatDate(item.end_date))}`,
            escapeHtml(item.status || "-"),
            escapeHtml(item.lock_mode || (item.locked_at ? "LOCKED" : "OPEN")),
            escapeHtml(formatDateTime(item.updated_at)),
          ])), "No business years registered.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create business year</h2><p>Open a new filing workspace for the selected customer.</p></div>
          <form id="businessYearForm" class="stack">
            <label>Customer <select id="byCustomer" ${data.customers.length ? "" : "disabled"}>${adminCustomerOptions(data.customers, firstCustomer?.customer_id)}</select></label>
            <label>Year <input id="byYear" type="number" value="${currentYear - 1}" /></label>
            <label>Start date <input id="byStart" type="date" value="${currentYear - 1}-01-01" /></label>
            <label>End date <input id="byEnd" type="date" value="${currentYear - 1}-12-31" /></label>
            <button class="primary-btn" type="submit" ${data.customers.length ? "" : "disabled"}>Create business year</button>
          </form>
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#businessYearForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/business-years`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: Number(env.outlet.querySelector("#byCustomer").value),
        year_label: Number(env.outlet.querySelector("#byYear").value),
        start_date: env.outlet.querySelector("#byStart").value,
        end_date: env.outlet.querySelector("#byEnd").value,
        carry_forward_from_by_id: null,
      }),
    });
    setLeafActionMessage("Business year created.", false, env.locale);
    await renderAdminBusinessYearMaster(env);
  });
}

async function renderAdminTaxAgentContracts(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerData(env);
  const { root } = data;
  const firstCustomer = data.customers[0] || null;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="tax-agent-contracts" data-leaf-key="admin/cust:agent">
      ${adminCustomerHeader(env, activeLeaf, "Tax agent contracts", "Track delegated tax agent contracts, customer assignments, contract periods, and active status.")}
      ${metrics([
        ["Agents", money.format(data.taxAgents.length)],
        ["Active", money.format(data.taxAgents.filter((item) => item.status === "ACTIVE").length)],
        ["Customers", money.format(data.customers.length)],
        ["Delegations", money.format(data.taxAgents.filter((item) => item.customer_id || item.assigned_customer_id).length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Contract registry</h2><p>External or delegated tax agents by tenant and customer assignment.</p></div>
          ${table(["Agent", "Customer", "Tenant", "Status", "Contract period", "Notes"], data.taxAgents.map((item) => row([
            escapeHtml(item.agent_name || item.name || "-"),
            escapeHtml(adminCustomerDisplayName(data.customers, item.customer_id || item.assigned_customer_id)),
            escapeHtml(item.tenant_code || tenantCode(env)),
            escapeHtml(item.status || "ACTIVE"),
            `${escapeHtml(formatDate(item.contract_start || item.start_date))} ~ ${escapeHtml(formatDate(item.contract_end || item.end_date))}`,
            escapeHtml(item.notes || item.memo || "-"),
          ])), "No tax agent contracts.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Save tax agent contract</h2><p>Register or update delegated tax agent metadata for the tenant.</p></div>
          <form id="taxAgentForm" class="stack">
            <label>Customer <select id="agentCustomer" ${data.customers.length ? "" : "disabled"}>${adminCustomerOptions(data.customers, firstCustomer?.customer_id)}</select></label>
            <label>Agent name <input id="agentName" value="${escapeHtml(uiText(env.locale, "EY 세무대리인", "EY Tax Agent"))}" /></label>
            <label>Status <select id="agentStatus"><option value="ACTIVE">ACTIVE</option><option value="SUSPENDED">SUSPENDED</option><option value="EXPIRED">EXPIRED</option></select></label>
            <label>Contract start <input id="agentStart" type="date" value="${new Date().getFullYear()}-01-01" /></label>
            <label>Contract end <input id="agentEnd" type="date" value="${new Date().getFullYear()}-12-31" /></label>
            <label>Notes <input id="agentNotes" value="${escapeHtml(uiText(env.locale, "법인세 신고 대리 계약", "Corporate tax filing delegation"))}" /></label>
            <button class="primary-btn" type="submit">Save contract</button>
          </form>
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#taxAgentForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/tax-agents`, {
      method: "POST",
      body: JSON.stringify({
        customer_id: Number(env.outlet.querySelector("#agentCustomer")?.value || 0) || null,
        agent_name: env.outlet.querySelector("#agentName").value,
        status: env.outlet.querySelector("#agentStatus").value,
        contract_start: env.outlet.querySelector("#agentStart").value,
        contract_end: env.outlet.querySelector("#agentEnd").value,
        notes: env.outlet.querySelector("#agentNotes").value,
      }),
    });
    setLeafActionMessage("Tax agent contract saved.", false, env.locale);
    await renderAdminTaxAgentContracts(env);
  });
}

async function renderAdminCustomersWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/cust:by-master") return renderAdminBusinessYearMaster(env);
  if (activeLeaf === "admin/cust:agent") return renderAdminTaxAgentContracts(env);
  return renderAdminCustomerList(env);
}

const ADMIN_SECURITY_ROUTES = ["admin/sec:users", "admin/sec:roles", "admin/sec:matrix", "admin/sec:mask", "admin/sec:scope"];

function renderAdminRouteButtons(activeLeaf, routes, locale) {
  return routes.map((key) => {
    const meta = routeMeta(key, locale);
    return `<button class="${key === activeLeaf ? "primary-btn" : "secondary-btn"} compact" type="button" data-admin-route="${escapeHtml(key)}">${escapeHtml(meta.title)}</button>`;
  }).join("");
}

function adminSecurityHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Security and permission controls</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_SECURITY_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminSecurityData(env) {
  const root = routeRoot(env);
  const adminRoot = root.replace("/api/tenants", "/api/admin/tenants");
  const [users, customers, roles, permissions, functionCodes, roleMenuFunctions, maskingPolicies, dataScopes, loginHistory, systemSettings] = await Promise.all([
    request(`${adminRoot}/users`).catch(() => []),
    request(`${root}/customers`).catch(() => []),
    request("/api/admin/roles").catch(() => []),
    request("/api/admin/role-permissions").catch(() => []),
    request("/api/admin/function-codes").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
    request("/api/admin/field-masking").catch(() => []),
    request("/api/admin/data-scope").catch(() => []),
    request("/api/login-history").catch(() => []),
    request("/api/system-settings").catch(() => []),
  ]);
  return { root, adminRoot, users, customers, roles, permissions, functionCodes, roleMenuFunctions, maskingPolicies, dataScopes, loginHistory, systemSettings };
}

async function renderAdminUsers(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminSecurityData(env);
  const firstCustomer = data.customers[0];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security-users" data-leaf-key="admin/sec:users">
      ${adminSecurityHeader(env, activeLeaf, "User management", "Register tenant users, assign tax roles, unlock accounts, and review 2FA status.")}
      ${metrics([
        ["Users", money.format(data.users.length)],
        ["Active", money.format(data.users.filter((item) => item.status === "ACTIVE").length)],
        ["2FA", money.format(data.users.filter((item) => item.use_2fa).length)],
        ["Customers", money.format(data.customers.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>User registry</h2><p>Tenant login, status, assigned roles, and account recovery actions.</p></div>
          ${table(["ID", "Name", "Status", "2FA", "Roles", ""], data.users.map((item) => row([
            escapeHtml(item.login_id),
            escapeHtml(item.user_name),
            escapeHtml(item.status),
            item.use_2fa ? "Y" : "N",
            escapeHtml(asArray(item.roles).join(", ")),
            `<button class="secondary-btn compact" data-unlock-user="${escapeHtml(item.login_id)}" type="button">Unlock</button>`,
          ])), "No users found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>User registration</h2><p>Creates a TAX_EXPERT user and grants the first customer work scope.</p></div>
          <form id="userForm" class="stack">
            <label>ID <input id="userLogin" value="u${Date.now().toString(36).slice(-4)}" /></label>
            <label>Name <input id="userName" value="${escapeHtml(uiText(env.locale, "세무 담당자", "Tax staff"))}" /></label>
            <label>Password <input id="userPassword" value="ChangeMe123!" /></label>
            <label><input id="userUse2fa" type="checkbox" /> Use 2FA</label>
            <label>TOTP Secret <input id="userTotpSecret" placeholder="Base32 or raw secret" /></label>
            <button class="primary-btn" type="submit" ${firstCustomer ? "" : "disabled"}>Register</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#userForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${data.adminRoot}/users`, {
      method: "POST",
      body: JSON.stringify({
        login_id: env.outlet.querySelector("#userLogin").value,
        password: env.outlet.querySelector("#userPassword").value,
        user_name: env.outlet.querySelector("#userName").value,
        use_2fa: env.outlet.querySelector("#userUse2fa").checked,
        totp_secret: env.outlet.querySelector("#userTotpSecret").value || null,
        roles: ["TAX_EXPERT"],
        customer_access: [{ customer_id: firstCustomer.customer_id, access_level: "OWNER", is_primary: true, work_scopes: ["INFO", "ADJUST", "FORM"] }],
      }),
    });
    await renderAdminUsers(env);
  });
  env.outlet.querySelectorAll("[data-unlock-user]").forEach((button) => {
    button.addEventListener("click", async () => {
      await request(`${data.adminRoot}/users/${button.dataset.unlockUser}/status`, {
        method: "POST",
        body: JSON.stringify({ status: "ACTIVE", locked: false, reason: "admin unlock" }),
      });
      await renderAdminUsers(env);
    });
  });
}

async function renderAdminRoleCatalog(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminSecurityData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security-roles" data-leaf-key="admin/sec:roles">
      ${adminSecurityHeader(env, activeLeaf, "Role master", "Review system roles and the permission volume assigned to each role.")}
      ${metrics([
        ["Roles", money.format(data.roles.length)],
        ["System roles", money.format(data.roles.filter((item) => item.system_role).length)],
        ["Functions", money.format(data.functionCodes.length)],
        ["Menu grants", money.format(data.roleMenuFunctions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Role catalog</h2><p>Role master rows used by menu and function permission checks.</p></div>
          ${table(["Role", "Name", "System", "Permissions"], data.roles.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.role_name),
            item.system_role ? "Y" : "N",
            money.format(data.permissions.filter((permission) => permission.role_code === item.role_code).length),
          ])), "No roles found.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Function catalog</h2><p>Actions that can be enabled per menu.</p></div>
          ${table(["Code", "Name", "Sort"], data.functionCodes.map((item) => row([
            escapeHtml(item.function_code),
            escapeHtml(item.function_name),
            money.format(item.sort_order),
          ])), "No function codes found.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminPermissionMatrix(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminSecurityData(env);
  const selectedRole = data.roles.find((item) => item.role_code === "TAX_EXPERT") || data.roles[0] || null;
  const rolePermissions = data.permissions.filter((item) => item.role_code === selectedRole?.role_code);
  const roleMenuFunctions = data.roleMenuFunctions.filter((item) => item.role_code === selectedRole?.role_code);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security-matrix" data-leaf-key="admin/sec:matrix">
      ${adminSecurityHeader(env, activeLeaf, "Permission matrix", "Manage role to module and menu-function grants used by runtime permission checks.")}
      ${metrics([
        ["Selected role", selectedRole?.role_code || "-"],
        ["Module grants", money.format(rolePermissions.length)],
        ["Menu grants", money.format(roleMenuFunctions.length)],
        ["All grants", money.format(data.permissions.length + data.roleMenuFunctions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Module permission matrix</h2><button id="saveExpertPerm" class="primary-btn compact" type="button">${escapeHtml(selectedRole?.role_code || "Role")} baseline</button></div>
          ${table(["Role", "Module", "Function", "Effect"], rolePermissions.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.module_code),
            escapeHtml(item.function_code),
            escapeHtml(item.effect),
          ])), "No permissions for the selected role.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Menu function grants</h2><p>Menu x function permissions for visible action buttons.</p></div>
          ${table(["Role", "Menu", "Function", "Effect"], roleMenuFunctions.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.menu_key),
            escapeHtml(item.function_code),
            escapeHtml(item.effect),
          ])), "No role-menu function grants.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#saveExpertPerm")?.addEventListener("click", async () => {
    await request("/api/admin/roles/TAX_EXPERT/permissions", {
      method: "PUT",
      body: JSON.stringify({ permissions: [
        { module_code: "tax-data", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "adjustment", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "efiling", function_code: "EFILE", effect: "ALLOW" },
      ] }),
    });
    await renderAdminPermissionMatrix(env);
  });
}

async function renderAdminFieldMasking(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminSecurityData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security-mask" data-leaf-key="admin/sec:mask">
      ${adminSecurityHeader(env, activeLeaf, "Field masking policy", "Set field-level privacy defaults for sensitive customer and user data.")}
      ${metrics([
        ["Mask rules", money.format(data.maskingPolicies.length)],
        ["Roles", money.format(new Set(data.maskingPolicies.map((item) => item.role)).size)],
        ["Login events", money.format(data.loginHistory.length)],
        ["Policy endpoint", "field-masking"],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Masking policies</h2><p>Field, masking behavior, and role allowed to reveal values.</p></div>
          ${table(["Field", "Policy", "Role"], data.maskingPolicies.map((item) => row([
            escapeHtml(item.field),
            escapeHtml(item.policy),
            escapeHtml(item.role),
          ])), "No masking policy configured.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Policy edit</h2><p>Updates the representative business registration masking rule.</p></div>
          <form id="maskingPolicyForm" class="stack">
            <label>Field <input id="maskFieldInput" value="${escapeHtml(data.maskingPolicies[0]?.field || "biz_reg_no")}" /></label>
            <label>Policy <input id="maskPolicyInput" value="${escapeHtml(data.maskingPolicies[0]?.policy || "partial")}" /></label>
            <label>Reveal role <input id="maskRoleInput" value="${escapeHtml(data.maskingPolicies[0]?.role || "staff")}" /></label>
            <button class="primary-btn" type="submit">Save masking policy</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#maskingPolicyForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/field-masking", {
      method: "PUT",
      body: JSON.stringify([{
        field: env.outlet.querySelector("#maskFieldInput").value || "biz_reg_no",
        policy: env.outlet.querySelector("#maskPolicyInput").value || "partial",
        role: env.outlet.querySelector("#maskRoleInput").value || "staff",
      }]),
    });
    await renderAdminFieldMasking(env);
  });
}

async function renderAdminDataScope(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminSecurityData(env);
  const timeoutSetting = data.systemSettings.find((item) => item.setting_key === "session_timeout_minutes")?.setting_value || "60";
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="security-scope" data-leaf-key="admin/sec:scope">
      ${adminSecurityHeader(env, activeLeaf, "Data scope policy", "Control tenant and customer visibility rules that guard row-level access.")}
      ${metrics([
        ["Scope rules", money.format(data.dataScopes.length)],
        ["Customers", money.format(data.customers.length)],
        ["Users", money.format(data.users.length)],
        ["Session timeout", timeoutSetting],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Data scopes</h2><p>Tenant and customer visibility rules.</p></div>
          ${table(["Scope", "Rule"], data.dataScopes.map((item) => row([
            escapeHtml(item.scope),
            escapeHtml(item.rule),
          ])), "No data scope rules configured.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Scope edit</h2><p>Updates a representative tenant scope rule.</p></div>
          <form id="dataScopeForm" class="stack">
            <label>Scope <input id="scopeInput" value="${escapeHtml(data.dataScopes[0]?.scope || "tenant")}" /></label>
            <label>Rule <input id="scopeRuleInput" value="${escapeHtml(data.dataScopes[0]?.rule || `session_timeout=${timeoutSetting}`)}" /></label>
            <button class="primary-btn" type="submit">Save data scope</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#dataScopeForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/data-scope", {
      method: "PUT",
      body: JSON.stringify([{
        scope: env.outlet.querySelector("#scopeInput").value || "tenant",
        rule: env.outlet.querySelector("#scopeRuleInput").value || `session_timeout=${timeoutSetting}`,
      }]),
    });
    await renderAdminDataScope(env);
  });
}

async function renderAdminRolesWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/sec:users") return renderAdminUsers(env);
  if (activeLeaf === "admin/sec:roles") return renderAdminRoleCatalog(env);
  if (activeLeaf === "admin/sec:mask") return renderAdminFieldMasking(env);
  if (activeLeaf === "admin/sec:scope") return renderAdminDataScope(env);
  return renderAdminPermissionMatrix(env);
}

const ADMIN_MENU_GOVERNANCE_ROUTES = ["admin/sec:menus", "admin/sec:functions"];

function adminMenuGovernanceHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Menu and function governance</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_MENU_GOVERNANCE_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminMenuGovernanceData() {
  const [menus, menuFunctions, roleMenuFunctions, functionCodes, legacyFunctions, roles] = await Promise.all([
    request("/api/admin/menus").catch(() => []),
    request("/api/admin/menu-functions").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
    request("/api/admin/function-codes").catch(() => []),
    request("/api/admin/functions").catch(() => []),
    request("/api/admin/roles").catch(() => []),
  ]);
  return { menus, menuFunctions, roleMenuFunctions, functionCodes, legacyFunctions, roles };
}

function menuFunctionList(menuFunctions, menuKey) {
  return menuFunctions
    .filter((item) => item.menu_key === menuKey)
    .map((item) => item.function_code)
    .join(", ");
}

async function renderAdminMenuManagement(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const locale = env.locale || currentDocumentLocale();
  const data = await loadAdminMenuGovernanceData();
  const featureFlagged = data.menus.filter((item) => item.feature_flag).length;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="menu-management" data-leaf-key="admin/sec:menus">
      ${adminMenuGovernanceHeader(env, activeLeaf, "Menu management", "Control route exposure, feature flags, permission gates, and enabled status for every menu node.")}
      ${metrics([
        ["Menu nodes", money.format(data.menus.length)],
        ["Enabled", money.format(data.menus.filter((item) => item.enabled).length)],
        ["Feature flags", money.format(featureFlagged)],
        ["Menu functions", money.format(data.menuFunctions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Menu registry</h2><p>Permission gate and feature flag by menu leaf.</p></div>
          ${table(["Menu", "Parent", "Label", "Permission", "Feature flag", "Enabled", ""], data.menus.map((item) => row([
            escapeHtml(item.menu_key),
            escapeHtml(item.parent_key || "-"),
            escapeHtml(item.label),
            escapeHtml([item.required_perm_module, item.required_perm_function].filter(Boolean).join(":") || "-"),
            `<input value="${escapeHtml(item.feature_flag || "")}" data-menu-flag="${escapeHtml(item.menu_key)}" />`,
            item.enabled ? "Y" : "N",
            `<button class="secondary-btn compact" data-save-menu="${escapeHtml(item.menu_key)}" type="button">${escapeHtml(t(locale, "common.save"))}</button>`,
          ])), "No admin menus.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Menu action coverage</h2><p>Which actions are currently attached to each menu leaf.</p></div>
          ${table(["Menu", "Label", "Functions"], data.menus.filter((item) => item.leaf_key || !data.menus.some((child) => child.parent_key === item.menu_key)).map((item) => row([
            escapeHtml(item.menu_key),
            escapeHtml(item.label || "-"),
            escapeHtml(menuFunctionList(data.menuFunctions, item.menu_key) || "-"),
          ])), "No leaf menus.")}
        </article>
      </section>
      ${renderLeafActionResult()}
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelectorAll("[data-save-menu]").forEach((button) => {
    button.addEventListener("click", async () => {
      const input = env.outlet.querySelector(`[data-menu-flag="${CSS.escape(button.dataset.saveMenu)}"]`);
      await request(`/api/admin/menus/${button.dataset.saveMenu}`, {
        method: "PUT",
        body: JSON.stringify({ feature_flag: input?.value || null, enabled: true }),
      });
      setLeafActionMessage("Menu node saved.", false, env.locale);
      await renderAdminMenuManagement(env);
    });
  });
}

async function renderAdminFunctionCatalog(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminMenuGovernanceData();
  const selectedMenu = data.menus.find((item) => item.menu_key === "admin/sec:functions") || data.menus[0] || null;
  const selectedFunctions = selectedMenu ? menuFunctionList(data.menuFunctions, selectedMenu.menu_key) : "";
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="function-catalog" data-leaf-key="admin/sec:functions">
      ${adminMenuGovernanceHeader(env, activeLeaf, "Function code management", "Review action codes and maintain menu-to-function bindings used by permission checks.")}
      ${metrics([
        ["Function codes", money.format(data.functionCodes.length || data.legacyFunctions.length)],
        ["Menu functions", money.format(data.menuFunctions.length)],
        ["Role bindings", money.format(data.roleMenuFunctions.length)],
        ["Roles", money.format(data.roles.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Function catalog</h2><p>Canonical action codes referenced by menus, roles, and APIs.</p></div>
          ${table(["Code", "Name", "Active", "Sort"], (data.functionCodes.length ? data.functionCodes : data.legacyFunctions).map((item) => row([
            escapeHtml(item.function_code || item.code || "-"),
            escapeHtml(item.function_name || item.label || item.name || "-"),
            item.active === false || item.enabled === false ? "N" : "Y",
            escapeHtml(item.sort_order ?? "-"),
          ])), "No function codes.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Menu function assignment</h2><p>Replaces the allowed function set for one menu node.</p></div>
          <form id="menuFunctionForm" class="stack">
            <label>Menu <select id="menuFunctionMenu" ${data.menus.length ? "" : "disabled"}>${data.menus.map((item) => `<option value="${escapeHtml(item.menu_key)}" ${item.menu_key === selectedMenu?.menu_key ? "selected" : ""}>${escapeHtml(item.menu_key)} / ${escapeHtml(item.label || "-")}</option>`).join("")}</select></label>
            <label>Functions <input id="menuFunctionCodes" value="${escapeHtml(selectedFunctions || "READ")}" placeholder="READ, UPDATE" /></label>
            <button class="primary-btn" type="submit" ${data.menus.length ? "" : "disabled"}>Save menu functions</button>
          </form>
          ${table(["Menu", "Function", "Label", "Enabled"], data.menuFunctions.map((item) => row([
            escapeHtml(item.menu_key),
            escapeHtml(item.function_code),
            escapeHtml(item.function_name || item.label || "-"),
            item.enabled ? "Y" : "N",
          ])), "No menu functions.")}
          ${renderLeafActionResult()}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head"><h2>Role to menu function matrix</h2><p>Role-level grants and denials that use the function catalog.</p></div>
        ${table(["Role", "Menu", "Function", "Effect"], data.roleMenuFunctions.map((item) => row([
          escapeHtml(item.role_code),
          escapeHtml(item.menu_key),
          escapeHtml(item.function_code),
          escapeHtml(item.effect),
        ])), "No role-menu function grants.")}
      </article>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#menuFunctionMenu")?.addEventListener("change", (event) => {
    const input = env.outlet.querySelector("#menuFunctionCodes");
    if (input) input.value = menuFunctionList(data.menuFunctions, event.target.value) || "READ";
  });
  env.outlet.querySelector("#menuFunctionForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const menuKey = env.outlet.querySelector("#menuFunctionMenu").value;
    const functions = env.outlet.querySelector("#menuFunctionCodes").value
      .split(",")
      .map((item) => item.trim().toUpperCase())
      .filter(Boolean);
    await request(`/api/admin/menus/${encodeURIComponent(menuKey)}/functions`, {
      method: "PUT",
      body: JSON.stringify({ functions }),
    });
    setLeafActionMessage("Menu functions saved.", false, env.locale);
    await renderAdminFunctionCatalog(env);
  });
}

async function renderAdminMenusWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/sec:functions") return renderAdminFunctionCatalog(env);
  return renderAdminMenuManagement(env);
}

const ADMIN_CUSTOMER_ACCESS_ROUTES = ["admin/cacc:assign", "admin/cacc:groups", "admin/cacc:rules", "admin/cacc:delegate", "admin/cacc:override"];

function adminCustomerAccessHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Customer access controls</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_CUSTOMER_ACCESS_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminCustomerAccessData(env) {
  const root = routeRoot(env);
  const adminRoot = root.replace("/api/tenants", "/api/admin/tenants");
  const [users, customers, delegations, customerGroups, customerRules, adminDelegations, overrides] = await Promise.all([
    request(`${adminRoot}/users`).catch(() => []),
    request(`${root}/customers`).catch(() => []),
    request(`${root}/access-delegations`).catch(() => []),
    request("/api/admin/customer-groups").catch(() => []),
    request("/api/admin/customer-rules").catch(() => []),
    request("/api/admin/access-delegations").catch(() => []),
    request("/api/admin/customer-access/override").catch(() => []),
  ]);
  return { root, adminRoot, users, customers, delegations, customerGroups, customerRules, adminDelegations, overrides };
}

function customerName(customers, customerId) {
  const customer = customers.find((item) => String(item.customer_id) === String(customerId));
  return customer?.customer_name || customer?.customer_code || customerId || "-";
}

function customerAccessAssignments(data) {
  return data.users.flatMap((user) => asArray(user.customer_access).map((access) => ({
    user,
    access,
    customerName: customerName(data.customers, access.customer_id),
  })));
}

function accessLevelMetrics(assignments) {
  const counts = new Map();
  assignments.forEach(({ access }) => {
    const level = access.access_level || "UNKNOWN";
    counts.set(level, (counts.get(level) || 0) + 1);
  });
  return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

async function renderAdminCustomerAssignment(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerAccessData(env);
  const assignments = customerAccessAssignments(data);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access-assign" data-leaf-key="admin/cacc:assign">
      ${adminCustomerAccessHeader(env, activeLeaf, "Customer assignment", "Review user-to-customer access, customer work scopes, and effective access distribution.")}
      ${metrics([
        ["Users", money.format(data.users.length)],
        ["Customers", money.format(data.customers.length)],
        ["Assignments", money.format(assignments.length)],
        ["Blocked", money.format(assignments.filter(({ access }) => access.access_level === "BLOCKED").length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Individual customer assignments</h2><p>User, customer, access level, primary flag, and allowed work scopes.</p></div>
          ${table(["User", "Customer", "Access", "Primary", "Scopes"], assignments.map(({ user, access, customerName: displayName }) => row([
            escapeHtml(user.login_id),
            escapeHtml(displayName),
            escapeHtml(access.access_level || "-"),
            access.is_primary ? "Y" : "N",
            escapeHtml(asArray(access.work_scopes).join(", ") || "-"),
          ])), "No customer assignments.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Customer work scope master</h2><p>Customer-level target work scopes that limit user-level grants.</p></div>
          ${table(["Customer", "Business no.", "Target scopes"], data.customers.map((item) => row([
            escapeHtml(item.customer_name || item.customer_code || item.customer_id),
            escapeHtml(item.biz_reg_no || "-"),
            escapeHtml(asArray(item.work_scopes).join(", ") || "-"),
          ])), "No customers available.")}
        </article>
      </section>
      <article class="panel">
        <div class="panel-head">
          <div><h2>Effective access preview</h2><p>Distribution after direct assignment, group/rule expansion, delegation, and override precedence.</p></div>
          <button id="caccPreviewRefresh" class="secondary-btn compact" type="button">Recalculate preview</button>
        </div>
        ${table(["Access level", "Assignments"], accessLevelMetrics(assignments).map(([level, count]) => row([escapeHtml(level), money.format(count)])), "No effective access rows.")}
        <p class="empty" id="caccPreviewStatus">Delegations: ${escapeHtml(String(data.delegations.length + data.adminDelegations.length))}, overrides: ${escapeHtml(String(data.overrides.length))}</p>
      </article>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#caccPreviewRefresh")?.addEventListener("click", () => {
    const status = env.outlet.querySelector("#caccPreviewStatus");
    if (status) status.textContent = `Preview recalculated at ${new Date().toLocaleTimeString()}`;
  });
}

async function renderAdminCustomerGroups(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerAccessData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access-groups" data-leaf-key="admin/cacc:groups">
      ${adminCustomerAccessHeader(env, activeLeaf, "Access groups", "Manage customer groups used for bulk assignment and reusable access policies.")}
      ${metrics([
        ["Groups", money.format(data.customerGroups.length)],
        ["Customers", money.format(data.customers.length)],
        ["Grouped members", money.format(data.customerGroups.reduce((sum, item) => sum + Number(item.member_count || 0), 0))],
        ["Rules", money.format(data.customerRules.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Customer groups</h2><p>Reusable customer sets such as industry teams or VIP portfolios.</p></div>
          ${table(["Group", "Members", "Default access"], data.customerGroups.map((item) => row([
            escapeHtml(item.group_name || item.name || "-"),
            money.format(item.member_count || 0),
            escapeHtml(item.access_level || "CO_WORKER"),
          ])), "No customer groups.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create group</h2><p>Registers a reusable customer access group.</p></div>
          <form id="customerGroupForm" class="stack">
            <label>Group name <input id="customerGroupName" value="${escapeHtml(uiText(env.locale, "신규 고객사 그룹", "New customer group"))}" /></label>
            <label>Default access <select id="customerGroupAccess"><option>OWNER</option><option selected>CO_WORKER</option><option>REVIEWER</option><option>ASSISTANT</option><option>VIEWER</option></select></label>
            <label>Seed customer <select id="customerGroupSeed">${data.customers.map((item) => `<option value="${item.customer_id}">${escapeHtml(item.customer_name || item.customer_code || item.customer_id)}</option>`).join("")}</select></label>
            <button class="primary-btn" type="submit" ${data.customers.length ? "" : "disabled"}>Save group</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#customerGroupForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/customer-groups", {
      method: "POST",
      body: JSON.stringify({
        group_name: env.outlet.querySelector("#customerGroupName").value,
        access_level: env.outlet.querySelector("#customerGroupAccess").value,
        customer_ids: [Number(env.outlet.querySelector("#customerGroupSeed").value)],
      }),
    });
    await renderAdminCustomerGroups(env);
  });
}

async function renderAdminCustomerRules(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerAccessData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access-rules" data-leaf-key="admin/cacc:rules">
      ${adminCustomerAccessHeader(env, activeLeaf, "Access rules", "Define automatic assignment rules by customer attributes such as industry, region, and entity type.")}
      ${metrics([
        ["Rules", money.format(data.customerRules.length)],
        ["Customers", money.format(data.customers.length)],
        ["Assignments", money.format(customerAccessAssignments(data).length)],
        ["Overrides", money.format(data.overrides.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Automatic assignment rules</h2><p>Condition-based defaults before manual overrides are applied.</p></div>
          ${table(["Condition", "Access", "Priority"], data.customerRules.map((item) => row([
            escapeHtml(item.condition || "-"),
            escapeHtml(item.access_level || "-"),
            money.format(item.priority || item.rule_id || 0),
          ])), "No customer rules.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create rule</h2><p>Adds a condition-based default assignment rule.</p></div>
          <form id="customerRuleForm" class="stack">
            <label>Condition <input id="customerRuleCondition" value="${escapeHtml(uiText(env.locale, "업종코드가 62로 시작", "industry_code starts 62"))}" /></label>
            <label>Access <select id="customerRuleAccess"><option>OWNER</option><option>CO_WORKER</option><option selected>REVIEWER</option><option>ASSISTANT</option><option>VIEWER</option><option>BLOCKED</option></select></label>
            <label>Priority <input id="customerRulePriority" type="number" value="100" /></label>
            <button class="primary-btn" type="submit">Save rule</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#customerRuleForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/customer-rules", {
      method: "POST",
      body: JSON.stringify({
        condition: env.outlet.querySelector("#customerRuleCondition").value,
        access_level: env.outlet.querySelector("#customerRuleAccess").value,
        priority: Number(env.outlet.querySelector("#customerRulePriority").value || 100),
      }),
    });
    await renderAdminCustomerRules(env);
  });
}

async function renderAdminCustomerDelegation(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerAccessData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access-delegate" data-leaf-key="admin/cacc:delegate">
      ${adminCustomerAccessHeader(env, activeLeaf, "Delegation", "Create temporary handoff access for vacation coverage or reviewer substitution.")}
      ${metrics([
        ["Tenant delegations", money.format(data.delegations.length)],
        ["Admin delegations", money.format(data.adminDelegations.length)],
        ["Customers", money.format(data.customers.length)],
        ["Users", money.format(data.users.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Live tenant delegations</h2><p>Effective customer-level handoff windows.</p></div>
          ${table(["Grantor", "Delegatee", "Customer", "Scope", "Period"], data.delegations.map((item) => row([
            escapeHtml(item.grantor_login_id || item.grantor || "-"),
            escapeHtml(item.delegatee_login_id || item.delegatee || "-"),
            escapeHtml(customerName(data.customers, item.customer_id)),
            escapeHtml(item.work_scope || "-"),
            `${escapeHtml(item.valid_from || "-")} ~ ${escapeHtml(item.valid_to || "-")}`,
          ])), "No live delegations.")}
          ${table(["Grantor", "Delegatee", "Status"], data.adminDelegations.map((item) => row([
            escapeHtml(item.grantor || item.grantor_login_id || "-"),
            escapeHtml(item.delegatee || item.delegatee_login_id || "-"),
            escapeHtml(item.status || "-"),
          ])), "No admin delegation rules.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create delegation</h2><p>Delegates one customer work scope until the selected end date.</p></div>
          <form id="customerDelegationForm" class="stack">
            <label>Grantor <input id="delegationGrantor" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Delegatee <input id="delegationDelegatee" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Customer <select id="delegationCustomer">${data.customers.map((item) => `<option value="${item.customer_id}">${escapeHtml(item.customer_name || item.customer_code || item.customer_id)}</option>`).join("")}</select></label>
            <label>Scope <select id="delegationScope"><option>INFO</option><option>ADJUST</option><option>FORM</option><option>VALIDATE</option><option>APPROVE</option><option>PRINT</option><option>EFILE</option><option>POST</option></select></label>
            <label>Valid to <input id="delegationValidTo" type="date" value="${today()}" /></label>
            <label>Reason <input id="delegationReason" value="${escapeHtml(uiText(env.locale, "휴가 대체", "vacation coverage"))}" /></label>
            <button class="primary-btn" type="submit" ${data.customers.length ? "" : "disabled"}>Create delegation</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#customerDelegationForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${data.root}/access-delegations`, {
      method: "POST",
      body: JSON.stringify({
        grantor_login_id: env.outlet.querySelector("#delegationGrantor").value,
        delegatee_login_id: env.outlet.querySelector("#delegationDelegatee").value,
        customer_id: Number(env.outlet.querySelector("#delegationCustomer").value),
        work_scope: env.outlet.querySelector("#delegationScope").value,
        valid_from: today(),
        valid_to: env.outlet.querySelector("#delegationValidTo").value || null,
        reason: env.outlet.querySelector("#delegationReason").value || uiText(env.locale, "관리자 위임", "admin delegation"),
      }),
    });
    await renderAdminCustomerDelegation(env);
  });
}

async function renderAdminCustomerOverrides(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const data = await loadAdminCustomerAccessData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="customer-access-override" data-leaf-key="admin/cacc:override">
      ${adminCustomerAccessHeader(env, activeLeaf, "Access override", "Manage per-customer exceptions such as conflict blocking or elevated owner access.")}
      ${metrics([
        ["Overrides", money.format(data.overrides.length)],
        ["Blocked overrides", money.format(data.overrides.filter((item) => item.access_level === "BLOCKED").length)],
        ["Customers", money.format(data.customers.length)],
        ["Assignments", money.format(customerAccessAssignments(data).length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Individual access overrides</h2><p>Manual exceptions are applied after groups, rules, and delegations.</p></div>
          ${table(["Customer", "Access", "Reason"], data.overrides.map((item) => row([
            escapeHtml(item.customer_code || customerName(data.customers, item.customer_id) || "-"),
            escapeHtml(item.access_level || "-"),
            escapeHtml(item.reason || "-"),
          ])), "No access overrides.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create override</h2><p>Records a customer-level exception for the access evaluator.</p></div>
          <form id="customerOverrideForm" class="stack">
            <label>Customer <select id="overrideCustomer">${data.customers.map((item) => `<option value="${escapeHtml(item.customer_code || item.customer_id)}">${escapeHtml(item.customer_name || item.customer_code || item.customer_id)}</option>`).join("")}</select></label>
            <label>Access <select id="overrideAccess"><option>OWNER</option><option>CO_WORKER</option><option>REVIEWER</option><option>ASSISTANT</option><option>VIEWER</option><option selected>BLOCKED</option></select></label>
            <label>Reason <input id="overrideReason" value="${escapeHtml(uiText(env.locale, "이해상충 검토", "conflict review"))}" /></label>
            <button class="primary-btn" type="submit" ${data.customers.length ? "" : "disabled"}>Save override</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#customerOverrideForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/admin/customer-access/override", {
      method: "POST",
      body: JSON.stringify({
        customer_code: env.outlet.querySelector("#overrideCustomer").value,
        access_level: env.outlet.querySelector("#overrideAccess").value,
        reason: env.outlet.querySelector("#overrideReason").value || uiText(env.locale, "수동 예외", "manual override"),
      }),
    });
    await renderAdminCustomerOverrides(env);
  });
}

async function renderAdminCustomerAccessWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/cacc:groups") return renderAdminCustomerGroups(env);
  if (activeLeaf === "admin/cacc:rules") return renderAdminCustomerRules(env);
  if (activeLeaf === "admin/cacc:delegate") return renderAdminCustomerDelegation(env);
  if (activeLeaf === "admin/cacc:override") return renderAdminCustomerOverrides(env);
  return renderAdminCustomerAssignment(env);
}

const ADMIN_LAW_ROUTES = [
  "admin/law:master",
  "admin/law:rates",
  "admin/law:limits",
  "admin/law:credits",
  "admin/law:depr-lives",
  "admin/law:sme",
  "admin/law:loss-rule",
  "admin/law:snapshots",
  "admin/law:impact",
  "admin/law:history",
];

const LAW_LIMIT_SCREEN_CONFIG = {
  "admin/law:limits": {
    stage: "law-limits",
    category: "LIMIT",
    title: "Limit/rate table",
    description: "Maintain entertainment, donation, bad-debt, and business vehicle limit parameters.",
    itemCode: "ENTERTAINMENT_BASIC_LIMIT",
    amount: 36000000,
    unit: "KRW",
  },
  "admin/law:credits": {
    stage: "law-credits",
    category: "CREDIT",
    title: "Tax credit/reduction rates",
    description: "Maintain R&D, investment, foreign tax, and disaster credit rate parameters.",
    itemCode: "RND_CREDIT_BPS",
    amount: 2500,
    unit: "BPS",
  },
  "admin/law:depr-lives": {
    stage: "law-depr-lives",
    category: "DEPRECIATION_LIFE",
    title: "Depreciation lives",
    description: "Maintain standard useful lives and depreciation limit parameters used by B-4.",
    itemCode: "MACHINE_LIFE_YEARS",
    amount: 5,
    unit: "YEARS",
  },
  "admin/law:sme": {
    stage: "law-sme",
    category: "SME_CRITERIA",
    title: "SME criteria",
    description: "Maintain SME classification thresholds used by credits, minimum tax, and loss limits.",
    itemCode: "SME_REVENUE_LIMIT",
    amount: 150000000000,
    unit: "KRW",
  },
  "admin/law:loss-rule": {
    stage: "law-loss-rule",
    category: "LOSS_RULE",
    title: "Loss carryforward rules",
    description: "Maintain carryforward years and deduction ratio limits for B-11.",
    itemCode: "LOSS_CARRYFORWARD_YEARS",
    amount: 15,
    unit: "YEARS",
  },
};

function adminLawHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Law and rate version control</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_LAW_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminLawBase() {
  const [laws, summary, histories] = await Promise.all([
    request("/api/tax-laws").catch(() => []),
    request("/api/law-versioning/summary").catch(() => ({ laws: 0, rates: 0, limits: 0, amendments: 0 })),
    request("/api/law-amendments").catch(() => []),
  ]);
  const activeLaw = laws.find((item) => item.status === "ACTIVE") || laws.find((item) => item.status === "APPROVED") || laws[0] || null;
  return { laws, summary, histories, activeLaw };
}

function lawVersionOptions(laws, selectedId) {
  return laws.map((item) => `<option value="${item.law_version_id}" ${String(item.law_version_id) === String(selectedId) ? "selected" : ""}>${escapeHtml(item.version_code)} / ${escapeHtml(item.status)}</option>`).join("");
}

function lawEffectiveRange(item) {
  return `${escapeHtml(item.effective_from || "-")} ~ ${escapeHtml(item.effective_to || "")}`;
}

function limitCategory(item) {
  return item.metadata?.category || item.metadata?.group || "-";
}

function limitValue(item, unit) {
  const amount = Number(item.amount || 0);
  if (unit === "BPS" || String(item.item_code || "").endsWith("_BPS")) return `${(amount / 100).toFixed(2)}%`;
  if (unit === "YEARS" || String(item.item_code || "").includes("YEARS")) return `${money.format(amount)} years`;
  if (amount > 100000) return money.format(amount);
  return money.format(amount);
}

async function renderAdminLawMaster(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { laws, summary, activeLaw } = await loadAdminLawBase();
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law-master" data-leaf-key="admin/law:master">
      ${adminLawHeader(env, activeLeaf, "Law version master", "Register, review, approve, and retire temporal corporate tax law versions.")}
      ${metrics([
        ["Laws", money.format(summary.laws || laws.length)],
        ["Active version", activeLaw?.version_code || "-"],
        ["Rates", money.format(summary.rates || 0)],
        ["Limits", money.format(summary.limits || 0)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Law versions</h2><p>Temporal versions used by calculation snapshots.</p></div>
          ${table(["ID", "Version", "Name", "Status", "Effective"], laws.map((item) => row([
            escapeHtml(item.law_version_id),
            escapeHtml(item.version_code),
            escapeHtml(item.law_name),
            escapeHtml(item.status),
            lawEffectiveRange(item),
          ])), "No law versions.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create amendment draft</h2><p>Creates a DRAFT law version for later data entry and impact simulation.</p></div>
          <form id="lawMasterForm" class="stack">
            <label>Version code <input id="lawVersionCode" value="CIT-${new Date().getFullYear()}-${Date.now().toString(36).slice(-4).toUpperCase()}" /></label>
            <label>Law name <input id="lawNameInput" value="${escapeHtml(uiText(env.locale, "법인세법 개정안", "Corporate income tax amendment"))}" /></label>
            <label>Effective from <input id="lawEffectiveFrom" type="date" value="${new Date().getFullYear()}-01-01" /></label>
            <label>Effective to <input id="lawEffectiveTo" type="date" /></label>
            <button class="primary-btn" type="submit">Create version</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#lawMasterForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/tax-laws", {
      method: "POST",
      body: JSON.stringify({
        version_code: env.outlet.querySelector("#lawVersionCode").value,
        law_name: env.outlet.querySelector("#lawNameInput").value,
        effective_from: env.outlet.querySelector("#lawEffectiveFrom").value,
        effective_to: env.outlet.querySelector("#lawEffectiveTo").value || null,
        metadata: { source: "admin-law-master" },
      }),
    });
    await renderAdminLawMaster(env);
  });
}

async function renderAdminTaxRates(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { laws, summary, activeLaw } = await loadAdminLawBase();
  const rates = activeLaw ? await request(`/api/tax-rates?law_version_id=${activeLaw.law_version_id}`).catch(() => []) : [];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law-rates" data-leaf-key="admin/law:rates">
      ${adminLawHeader(env, activeLeaf, "Corporate tax rates", "Maintain taxable income brackets, statutory rates, and progressive deductions by effective date.")}
      ${metrics([
        ["Version", activeLaw?.version_code || "-"],
        ["Rate rows", money.format(rates.length)],
        ["All rates", money.format(summary.rates || rates.length)],
        ["Laws", money.format(laws.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Rate brackets</h2><p>Corporate tax, minimum tax, special rural tax, and penalty rate rows.</p></div>
          ${table(["Item", "Taxable from", "Taxable to", "Rate", "Deduction", "Effective"], rates.map((item) => row([
            escapeHtml(item.item_code),
            money.format(item.taxable_from),
            item.taxable_to ? money.format(item.taxable_to) : "-",
            `${(Number(item.rate_bps || 0) / 100).toFixed(2)}%`,
            money.format(item.progressive_deduction || 0),
            lawEffectiveRange(item),
          ])), "No tax rates for the selected version.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Add rate bracket</h2><p>Creates a temporal rate row for the selected law version.</p></div>
          <form id="taxRateForm" class="stack">
            <label>Law version <select id="taxRateLaw" ${laws.length ? "" : "disabled"}>${lawVersionOptions(laws, activeLaw?.law_version_id)}</select></label>
            <label>Item code <input id="taxRateItem" value="CORP_TAX_GENERAL" /></label>
            <label>Taxable from <input id="taxRateFrom" type="number" value="0" /></label>
            <label>Taxable to <input id="taxRateTo" type="number" value="200000000" /></label>
            <label>Rate bps <input id="taxRateBps" type="number" value="900" /></label>
            <label>Progressive deduction <input id="taxRateDeduction" type="number" value="0" /></label>
            <label>Effective from <input id="taxRateEffectiveFrom" type="date" value="${activeLaw?.effective_from || today()}" /></label>
            <button class="primary-btn" type="submit" ${laws.length ? "" : "disabled"}>Save rate</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#taxRateForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/tax-rates", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(env.outlet.querySelector("#taxRateLaw").value),
        item_code: env.outlet.querySelector("#taxRateItem").value,
        taxable_from: Number(env.outlet.querySelector("#taxRateFrom").value || 0),
        taxable_to: env.outlet.querySelector("#taxRateTo").value ? Number(env.outlet.querySelector("#taxRateTo").value) : null,
        base_tax: 0,
        rate_bps: Number(env.outlet.querySelector("#taxRateBps").value || 0),
        progressive_deduction: Number(env.outlet.querySelector("#taxRateDeduction").value || 0),
        effective_from: env.outlet.querySelector("#taxRateEffectiveFrom").value || today(),
        effective_to: null,
        metadata: { category: "RATE" },
      }),
    });
    await renderAdminTaxRates(env);
  });
}

async function renderAdminLawLimitScreen(env, config) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { laws, summary, activeLaw } = await loadAdminLawBase();
  const query = activeLaw ? `law_version_id=${activeLaw.law_version_id}&category=${encodeURIComponent(config.category)}` : `category=${encodeURIComponent(config.category)}`;
  const rows = await request(`/api/tax-limits?${query}`).catch(() => []);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="${escapeHtml(config.stage)}" data-leaf-key="${escapeHtml(activeLeaf)}">
      ${adminLawHeader(env, activeLeaf, config.title, config.description)}
      ${metrics([
        ["Version", activeLaw?.version_code || "-"],
        ["Rows", money.format(rows.length)],
        ["Category", config.category],
        ["All limits", money.format(summary.limits || rows.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>${escapeHtml(config.title)}</h2><p>Temporal parameter rows filtered by ${escapeHtml(config.category)}.</p></div>
          ${table(["Item", "Value", "Category", "Effective"], rows.map((item) => row([
            escapeHtml(item.item_code),
            escapeHtml(limitValue(item, config.unit)),
            escapeHtml(limitCategory(item)),
            lawEffectiveRange(item),
          ])), `No ${config.category} rows.`)}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Add parameter</h2><p>Stores a versioned parameter row with category metadata.</p></div>
          <form id="lawLimitForm" class="stack">
            <label>Law version <select id="lawLimitVersion" ${laws.length ? "" : "disabled"}>${lawVersionOptions(laws, activeLaw?.law_version_id)}</select></label>
            <label>Item code <input id="lawLimitItem" value="${escapeHtml(config.itemCode)}" /></label>
            <label>Amount <input id="lawLimitAmount" type="number" value="${escapeHtml(config.amount)}" /></label>
            <label>Effective from <input id="lawLimitEffectiveFrom" type="date" value="${activeLaw?.effective_from || today()}" /></label>
            <button class="primary-btn" type="submit" ${laws.length ? "" : "disabled"}>Save parameter</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#lawLimitForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/tax-limits", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(env.outlet.querySelector("#lawLimitVersion").value),
        item_code: env.outlet.querySelector("#lawLimitItem").value,
        amount: Number(env.outlet.querySelector("#lawLimitAmount").value || 0),
        effective_from: env.outlet.querySelector("#lawLimitEffectiveFrom").value || today(),
        effective_to: null,
        metadata: { category: config.category, unit: config.unit },
      }),
    });
    await renderAdminLawLimitScreen(env, config);
  });
}

async function renderAdminLawLimits(env) {
  return renderAdminLawLimitScreen(env, LAW_LIMIT_SCREEN_CONFIG["admin/law:limits"]);
}

async function renderAdminLawCredits(env) {
  return renderAdminLawLimitScreen(env, LAW_LIMIT_SCREEN_CONFIG["admin/law:credits"]);
}

async function renderAdminLawDepreciationLives(env) {
  return renderAdminLawLimitScreen(env, LAW_LIMIT_SCREEN_CONFIG["admin/law:depr-lives"]);
}

async function renderAdminLawSmeCriteria(env) {
  return renderAdminLawLimitScreen(env, LAW_LIMIT_SCREEN_CONFIG["admin/law:sme"]);
}

async function renderAdminLawLossRules(env) {
  return renderAdminLawLimitScreen(env, LAW_LIMIT_SCREEN_CONFIG["admin/law:loss-rule"]);
}

async function renderAdminLawSnapshots(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const root = routeRoot(env);
  const { laws, summary, activeLaw } = await loadAdminLawBase();
  const years = await request(`${root}/business-years`).catch(() => []);
  const selectedYear = years[0] || null;
  const snapshot = selectedYear ? await request(`${root}/business-years/${selectedYear.by_id}/snapshot`).catch(() => null) : null;
  const snapshotData = snapshot?.snapshot_data || {};
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law-snapshots" data-leaf-key="admin/law:snapshots">
      ${adminLawHeader(env, activeLeaf, "Business-year law snapshots", "Inspect immutable law, rate, form, and e-filing versions applied to a business year.")}
      ${metrics([
        ["Business years", money.format(years.length)],
        ["Snapshot", snapshot?.snapshot_id || "-"],
        ["Locked", snapshot?.locked ? "Y" : "N"],
        ["Latest law", activeLaw?.version_code || summary.latest_law?.version_code || "-"],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Business years</h2><p>Select a business year from this tenant to inspect its applied snapshot.</p></div>
          ${table(["ID", "Customer", "Year", "Status"], years.map((item) => row([
            escapeHtml(item.by_id),
            escapeHtml(item.customer_id),
            escapeHtml(item.year_label),
            escapeHtml(item.status),
          ])), "No business years.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Selected snapshot</h2><p>${escapeHtml(selectedYear ? `${selectedYear.year_label} / ${selectedYear.status}` : "No selected business year")}</p></div>
          ${table(["Item", "Value"], [
            row(["Snapshot ID", escapeHtml(snapshot?.snapshot_id || "-")]),
            row(["Law version", escapeHtml(snapshotData.law?.version_code || snapshot?.law_version_id || "-")]),
            row(["Rate rows", money.format(asArray(snapshot?.rate_version_ids).length)]),
            row(["Form versions", money.format(asArray(snapshotData.form_versions).length)]),
            row(["E-file masters", money.format(asArray(snapshotData.efile_masters).length)]),
          ])}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminLawImpact(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { laws, summary, activeLaw } = await loadAdminLawBase();
  const impact = activeLaw ? await request("/api/law-versioning/impact", {
    method: "POST",
    body: JSON.stringify({ law_version_id: activeLaw.law_version_id, include_locked: false }),
  }).catch(() => ({ summary: {}, tenant_impacts: [] })) : { summary: {}, tenant_impacts: [] };
  const impactSummary = impact.summary || impact;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law-impact" data-leaf-key="admin/law:impact">
      ${adminLawHeader(env, activeLeaf, "Impact simulation", "Dry-run law amendments against open business years before activation and notification.")}
      ${metrics([
        ["Target version", activeLaw?.version_code || "-"],
        ["Business years", money.format(impactSummary.business_years || impact.impacted_business_years || 0)],
        ["Locked skipped", money.format(impactSummary.locked_snapshots || 0)],
        ["Limit rows", money.format(impactSummary.limit_rows || summary.limits || 0)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Simulation target</h2><p>Choose a draft or active version and include locked snapshots only when auditing.</p></div>
          <form id="lawImpactForm" class="stack">
            <label>Law version <select id="lawImpactVersion" ${laws.length ? "" : "disabled"}>${lawVersionOptions(laws, activeLaw?.law_version_id)}</select></label>
            <label><input id="lawImpactLocked" type="checkbox" /> Include locked snapshots</label>
            <button class="primary-btn" type="submit" ${laws.length ? "" : "disabled"}>Run simulation</button>
          </form>
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Tenant impact</h2><p>Open business years potentially affected by the law version window.</p></div>
          ${table(["Tenant", "Business years", "Locked", "Schema"], asArray(impact.tenant_impacts).map((item) => row([
            escapeHtml(item.tenant_code),
            money.format(item.business_years || 0),
            money.format(item.locked_snapshots || 0),
            item.schema_ready ? "READY" : "MISSING",
          ])), "No impacted tenants.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#lawImpactForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/law-versioning/impact", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(env.outlet.querySelector("#lawImpactVersion").value),
        include_locked: env.outlet.querySelector("#lawImpactLocked").checked,
      }),
    });
    await renderAdminLawImpact(env);
  });
}

async function renderAdminLawHistory(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { laws, histories, activeLaw } = await loadAdminLawBase();
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="law-history" data-leaf-key="admin/law:history">
      ${adminLawHeader(env, activeLeaf, "Amendment history", "Review approval notes, legal change summaries, and amendment audit trail entries.")}
      ${metrics([
        ["History rows", money.format(histories.length)],
        ["Laws", money.format(laws.length)],
        ["Latest version", activeLaw?.version_code || "-"],
        ["Approvers", money.format(new Set(histories.map((item) => item.approved_by)).size)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Law amendment history</h2><p>Tracked legal changes and approval notes.</p></div>
          ${table(["Law", "Summary", "Approved by", "Approved at"], histories.map((item) => row([
            escapeHtml(item.law_version_id),
            escapeHtml(item.change_summary || "-"),
            escapeHtml(item.approved_by || "-"),
            escapeHtml(item.approved_at || "-"),
          ])), "No law amendment history.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Record amendment note</h2><p>Adds a history row to the selected law version.</p></div>
          <form id="lawHistoryForm" class="stack">
            <label>Law version <select id="lawHistoryVersion" ${laws.length ? "" : "disabled"}>${lawVersionOptions(laws, activeLaw?.law_version_id)}</select></label>
            <label>Summary <input id="lawHistorySummary" value="${escapeHtml(uiText(env.locale, "법령 개정 검토", "Legal amendment reviewed"))}" /></label>
            <label>Approved by <input id="lawHistoryApprover" value="${escapeHtml(env.auth?.user?.login_id || "admin")}" /></label>
            <button class="primary-btn" type="submit" ${laws.length ? "" : "disabled"}>Save history</button>
          </form>
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#lawHistoryForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/law-amendments", {
      method: "POST",
      body: JSON.stringify({
        law_version_id: Number(env.outlet.querySelector("#lawHistoryVersion").value),
        change_summary: env.outlet.querySelector("#lawHistorySummary").value,
        approved_by: env.outlet.querySelector("#lawHistoryApprover").value || env.auth?.user?.login_id || "admin",
      }),
    });
    await renderAdminLawHistory(env);
  });
}

async function renderAdminLawWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/law:rates") return renderAdminTaxRates(env);
  if (activeLeaf === "admin/law:limits") return renderAdminLawLimits(env);
  if (activeLeaf === "admin/law:credits") return renderAdminLawCredits(env);
  if (activeLeaf === "admin/law:depr-lives") return renderAdminLawDepreciationLives(env);
  if (activeLeaf === "admin/law:sme") return renderAdminLawSmeCriteria(env);
  if (activeLeaf === "admin/law:loss-rule") return renderAdminLawLossRules(env);
  if (activeLeaf === "admin/law:snapshots") return renderAdminLawSnapshots(env);
  if (activeLeaf === "admin/law:impact") return renderAdminLawImpact(env);
  if (activeLeaf === "admin/law:history") return renderAdminLawHistory(env);
  return renderAdminLawMaster(env);
}

const ADMIN_FORM_ROUTES = [
  "admin/form:master",
  "admin/form:versions",
  "admin/form:fields",
  "admin/form:validations",
  "admin/form:linkage-rule",
  "admin/form:migration",
  "admin/form:efile-map",
  "admin/form:by-set",
  "admin/form:impact",
];

function adminFormHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Form version administration</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_FORM_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminFormBase() {
  const [forms, versions] = await Promise.all([
    request("/api/form-versioning/forms").catch(() => []),
    request("/api/form-versioning/versions").catch(() => []),
  ]);
  const activeVersion = versions.find((item) => ["ACTIVE", "APPROVED"].includes(String(item.status || "").toUpperCase())) || versions[0] || null;
  return { forms, versions, activeVersion };
}

function formOptions(forms, selectedCode) {
  return forms.map((item) => `<option value="${escapeHtml(item.form_code)}" ${String(item.form_code) === String(selectedCode) ? "selected" : ""} data-form-name="${escapeHtml(item.form_name)}">${escapeHtml(item.form_code)} / ${escapeHtml(item.form_name)}</option>`).join("");
}

function formVersionOptions(versions, selectedId) {
  return versions.map((item) => `<option value="${escapeHtml(item.form_version_id)}" ${String(item.form_version_id) === String(selectedId) ? "selected" : ""}>${escapeHtml(item.form_code)} v${escapeHtml(item.version_no)} / ${escapeHtml(item.status)}</option>`).join("");
}

function selectedAdminFormVersion(versions, state) {
  const selectedId = state.versionId;
  return versions.find((item) => String(item.form_version_id) === String(selectedId)) || versions[0] || null;
}

function formVersionSummary(item) {
  if (!item) return "No selected form version";
  return `${item.form_code} v${item.version_no} / ${item.status}`;
}

function stringifyRule(value) {
  return escapeHtml(JSON.stringify(value || {}));
}

async function renderAdminFormMaster(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { forms, versions, activeVersion } = await loadAdminFormBase();
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-master" data-leaf-key="admin/form:master">
      ${adminFormHeader(env, activeLeaf, "Form master", "Register master tax forms and keep their active status separate from version metadata.")}
      ${metrics([
        ["Forms", money.format(forms.length)],
        ["Active", money.format(forms.filter((item) => item.active).length)],
        ["Versions", money.format(versions.length)],
        ["Current version", activeVersion ? `${activeVersion.form_code} ${activeVersion.version_no}` : "-"],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Master forms</h2><p>Reusable form codes used by business-year snapshots and e-filing mappings.</p></div>
          ${table(["Code", "Name", "Group", "Active"], forms.map((item) => row([
            escapeHtml(item.form_code),
            escapeHtml(item.form_name),
            escapeHtml(item.form_group || "-"),
            item.active ? "Y" : "N",
          ])), "No forms.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Register form</h2><p>Creates or updates a master form without creating a version row.</p></div>
          <form id="formMasterForm" class="stack">
            <label>Form code <input id="formMasterCode" value="FORM${Date.now().toString(36).slice(-3).toUpperCase()}" /></label>
            <label>Form name <input id="formMasterName" value="${escapeHtml(uiText(env.locale, "법인세 서식", "Corporate tax form"))}" /></label>
            <label>Group <input id="formMasterGroup" value="CIT" /></label>
            <label>Description <input id="formMasterDescription" value="${escapeHtml(uiText(env.locale, "관리자 서식 마스터에서 관리", "Managed from admin form master"))}" /></label>
            <label><input id="formMasterActive" type="checkbox" checked /> Active</label>
            <button class="primary-btn" type="submit">Save form</button>
          </form>
          ${renderLeafActionResult()}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formMasterForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/form-versioning/forms", {
      method: "POST",
      body: JSON.stringify({
        form_code: env.outlet.querySelector("#formMasterCode").value,
        form_name: env.outlet.querySelector("#formMasterName").value,
        form_group: env.outlet.querySelector("#formMasterGroup").value || null,
        description: env.outlet.querySelector("#formMasterDescription").value || null,
        active: env.outlet.querySelector("#formMasterActive").checked,
      }),
    });
    setLeafActionMessage("Form master saved.", false, env.locale);
    await renderAdminFormMaster(env);
  });
}

async function renderAdminFormVersions(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { forms, versions, activeVersion } = await loadAdminFormBase();
  const selectedForm = forms.find((item) => item.form_code === activeVersion?.form_code) || forms[0] || null;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-versions" data-leaf-key="admin/form:versions">
      ${adminFormHeader(env, activeLeaf, "Form versions", "Create temporal form versions, review effective windows, and manage status changes.")}
      ${metrics([
        ["Versions", money.format(versions.length)],
        ["Active", money.format(versions.filter((item) => ["ACTIVE", "APPROVED"].includes(String(item.status || "").toUpperCase())).length)],
        ["Forms", money.format(forms.length)],
        ["Latest", activeVersion ? `${activeVersion.form_code} ${activeVersion.version_no}` : "-"],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Version registry</h2><p>Effective form templates used by snapshots and migration checks.</p></div>
          ${table(["ID", "Form", "Version", "Status", "Effective"], versions.map((item) => row([
            escapeHtml(item.form_version_id),
            escapeHtml(item.form_code),
            escapeHtml(item.version_no),
            escapeHtml(item.status),
            `${escapeHtml(item.effective_from || "-")} ~ ${escapeHtml(item.effective_to || "")}`,
          ])), "No versions.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create version</h2><p>Creates a draft template version and keeps the master form synchronized.</p></div>
          <form id="formVersionForm" class="stack">
            <label>Form <select id="formVersionCode" ${forms.length ? "" : "disabled"}>${formOptions(forms, selectedForm?.form_code)}</select></label>
            <label>Form name <input id="formVersionName" value="${escapeHtml(selectedForm?.form_name || uiText(env.locale, "법인세 서식", "Corporate tax form"))}" /></label>
            <label>Version no <input id="formVersionNo" value="${new Date().getFullYear()}.1" /></label>
            <label>Effective from <input id="formVersionFrom" type="date" value="${today()}" /></label>
            <label>Effective to <input id="formVersionTo" type="date" /></label>
            <label>Status <select id="formVersionStatus"><option>DRAFT</option><option>REVIEWED</option><option>APPROVED</option><option>ACTIVE</option><option>RETIRED</option></select></label>
            <button class="primary-btn" type="submit" ${forms.length ? "" : "disabled"}>Create version</button>
          </form>
          ${renderLeafActionResult()}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formVersionCode")?.addEventListener("change", (event) => {
    const form = forms.find((item) => item.form_code === event.target.value);
    const nameInput = env.outlet.querySelector("#formVersionName");
    if (form && nameInput) nameInput.value = form.form_name;
  });
  env.outlet.querySelector("#formVersionForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/form-versioning/versions", {
      method: "POST",
      body: JSON.stringify({
        form_code: env.outlet.querySelector("#formVersionCode").value,
        form_name: env.outlet.querySelector("#formVersionName").value,
        version_no: env.outlet.querySelector("#formVersionNo").value,
        effective_from: env.outlet.querySelector("#formVersionFrom").value || today(),
        effective_to: env.outlet.querySelector("#formVersionTo").value || null,
        template_json: { fields: [] },
        status: env.outlet.querySelector("#formVersionStatus").value,
      }),
    });
    setLeafActionMessage("Form version created.", false, env.locale);
    await renderAdminFormVersions(env);
  });
}

async function renderAdminFormFields(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const state = leafViewState.get("admin/form:fields") || {};
  const { forms, versions } = await loadAdminFormBase();
  const selectedVersion = selectedAdminFormVersion(versions, state);
  const fields = selectedVersion ? await request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/fields`).catch(() => []) : [];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-fields" data-leaf-key="admin/form:fields">
      ${adminFormHeader(env, activeLeaf, "Field definitions", "Inspect and update template fields for the selected form version.")}
      ${metrics([
        ["Fields", money.format(fields.length)],
        ["Selected", formVersionSummary(selectedVersion)],
        ["Versions", money.format(versions.length)],
        ["Forms", money.format(forms.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Version fields</h2><p>${escapeHtml(formVersionSummary(selectedVersion))}</p></div>
          <label class="inline-control">Version <select id="formFieldVersion" ${versions.length ? "" : "disabled"}>${formVersionOptions(versions, selectedVersion?.form_version_id)}</select></label>
          ${table(["Field", "Label"], fields.map((item) => row([
            escapeHtml(item.field_path || item.field_name || item.path || "-"),
            escapeHtml(item.label || item.field_label || "-"),
          ])), "No fields for the selected version.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Add field</h2><p>Updates the selected version field list through the versioning API.</p></div>
          <form id="formFieldForm" class="stack">
            <label>Field path <input id="formFieldPath" value="new_field" /></label>
            <label>Label <input id="formFieldLabel" value="${escapeHtml(uiText(env.locale, "신규 필드", "New field"))}" /></label>
            <button class="primary-btn" type="submit" ${selectedVersion ? "" : "disabled"}>Save fields</button>
          </form>
          ${renderLeafActionResult()}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formFieldVersion")?.addEventListener("change", async (event) => {
    leafViewState.set("admin/form:fields", { versionId: event.target.value });
    await renderAdminFormFields(env);
  });
  env.outlet.querySelector("#formFieldForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const nextFields = [...fields, {
      field_path: env.outlet.querySelector("#formFieldPath").value,
      label: env.outlet.querySelector("#formFieldLabel").value,
    }];
    await request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/fields`, {
      method: "PUT",
      body: JSON.stringify({ fields: nextFields }),
    });
    setLeafActionMessage("Field definitions saved.", false, env.locale);
    await renderAdminFormFields(env);
  });
}

async function renderAdminFormValidations(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const state = leafViewState.get("admin/form:validations") || {};
  const { forms, versions } = await loadAdminFormBase();
  const selectedVersion = selectedAdminFormVersion(versions, state);
  const validations = selectedVersion ? await request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/validations`).catch(() => []) : [];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-validations" data-leaf-key="admin/form:validations">
      ${adminFormHeader(env, activeLeaf, "Validation rules", "Maintain field-level validation rules attached to each form version.")}
      ${metrics([
        ["Rules", money.format(validations.length)],
        ["Errors", money.format(validations.filter((item) => String(item.severity).toUpperCase() === "ERROR").length)],
        ["Selected", formVersionSummary(selectedVersion)],
        ["Forms", money.format(forms.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Version validation rules</h2><p>${escapeHtml(formVersionSummary(selectedVersion))}</p></div>
          <label class="inline-control">Version <select id="formValidationVersion" ${versions.length ? "" : "disabled"}>${formVersionOptions(versions, selectedVersion?.form_version_id)}</select></label>
          ${table(["Field", "Rule", "Severity", "Message"], validations.map((item) => row([
            escapeHtml(item.field_path || "-"),
            escapeHtml(item.rule_code || "-"),
            escapeHtml(item.severity || "-"),
            escapeHtml(item.message || "-"),
          ])), "No validations for the selected version.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Add validation</h2><p>Saves rule metadata used by form validation and filing readiness checks.</p></div>
          <form id="formValidationForm" class="stack">
            <label>Field path <input id="formValidationField" value="taxable_income" /></label>
            <label>Rule code <input id="formValidationRule" value="REQUIRED" /></label>
            <label>Severity <select id="formValidationSeverity"><option>ERROR</option><option>WARN</option><option>INFO</option></select></label>
            <label>Message <input id="formValidationMessage" value="${escapeHtml(uiText(env.locale, "필수 입력 항목입니다.", "Field is required"))}" /></label>
            <button class="primary-btn" type="submit" ${selectedVersion ? "" : "disabled"}>Save validations</button>
          </form>
          ${renderLeafActionResult()}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formValidationVersion")?.addEventListener("change", async (event) => {
    leafViewState.set("admin/form:validations", { versionId: event.target.value });
    await renderAdminFormValidations(env);
  });
  env.outlet.querySelector("#formValidationForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const nextValidations = [...validations, {
      field_path: env.outlet.querySelector("#formValidationField").value,
      rule_code: env.outlet.querySelector("#formValidationRule").value,
      severity: env.outlet.querySelector("#formValidationSeverity").value,
      message: env.outlet.querySelector("#formValidationMessage").value,
      rule_json: { required: true },
    }];
    await request(`/api/form-versioning/versions/${selectedVersion.form_version_id}/validations`, {
      method: "PUT",
      body: JSON.stringify({ validations: nextValidations }),
    });
    setLeafActionMessage("Validation rules saved.", false, env.locale);
    await renderAdminFormValidations(env);
  });
}

async function renderAdminFormLinkageRules(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { forms, versions } = await loadAdminFormBase();
  const [relationships, cycleCheck, references] = await Promise.all([
    request("/api/form-versioning/relationships").catch(() => []),
    request("/api/form-versioning/cycle-check").catch(() => ({ valid: false, cycles: [] })),
    request("/api/form-versioning/field-references").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-linkage-rule" data-leaf-key="admin/form:linkage-rule">
      ${adminFormHeader(env, activeLeaf, "Linkage rules", "Maintain cross-form field dependencies and check the relationship graph for cycles.")}
      ${metrics([
        ["Relationships", money.format(relationships.length)],
        ["Cycle check", cycleCheck.valid ? "ACYCLIC" : "CYCLE"],
        ["References", money.format(asArray(references).length)],
        ["Versions", money.format(versions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Relationship graph</h2><span class="badge ${cycleCheck.valid ? "ok" : "error"}">${cycleCheck.valid ? "ACYCLIC" : "CYCLE"}</span></div>
          ${table(["Source", "Target", "Rule"], relationships.map((item) => row([
            `${escapeHtml(item.source_form)}.${escapeHtml(item.source_field)}`,
            `${escapeHtml(item.target_form)}.${escapeHtml(item.target_field)}`,
            stringifyRule(item.rule_json),
          ])), "No relationships.")}
          ${table(["Reference", "Target", "Cycle"], asArray(references).map((item) => row([
            escapeHtml(item.source || "-"),
            escapeHtml(item.target || "-"),
            item.cycle ? "Y" : "N",
          ])), "No field references.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Add linkage</h2><p>Creates a relationship used by automatic form propagation.</p></div>
          <form id="formRelationshipForm" class="stack">
            <label>Source form <input id="formRelSourceForm" value="${escapeHtml(forms[0]?.form_code || "FORM15")}" /></label>
            <label>Source field <input id="formRelSourceField" value="taxable_income" /></label>
            <label>Target form <input id="formRelTargetForm" value="${escapeHtml(forms[1]?.form_code || forms[0]?.form_code || "FORM3")}" /></label>
            <label>Target field <input id="formRelTargetField" value="taxable_income" /></label>
            <label>Effective from <input id="formRelFrom" type="date" value="${today()}" /></label>
            <button class="primary-btn" type="submit">Save relationship</button>
          </form>
          ${renderLeafActionResult()}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formRelationshipForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    await request("/api/form-versioning/relationships", {
      method: "POST",
      body: JSON.stringify({
        source_form: env.outlet.querySelector("#formRelSourceForm").value,
        source_field: env.outlet.querySelector("#formRelSourceField").value,
        target_form: env.outlet.querySelector("#formRelTargetForm").value,
        target_field: env.outlet.querySelector("#formRelTargetField").value,
        rule_json: { operation: "COPY" },
        effective_from: env.outlet.querySelector("#formRelFrom").value || today(),
        effective_to: null,
      }),
    });
    setLeafActionMessage("Relationship saved.", false, env.locale);
    await renderAdminFormLinkageRules(env);
  });
}

async function renderAdminFormMigration(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const root = routeRoot(env);
  const { forms, versions, activeVersion } = await loadAdminFormBase();
  const years = await request(`${root}/business-years`).catch(() => []);
  const selectedYear = years[0] || null;
  const selectedFormCode = activeVersion?.form_code || forms[0]?.form_code || "FORM3";
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-migration" data-leaf-key="admin/form:migration">
      ${adminFormHeader(env, activeLeaf, "Form migration", "Dry-run, execute, or roll back business-year form data migrations to a target version.")}
      ${metrics([
        ["Business years", money.format(years.length)],
        ["Target versions", money.format(versions.length)],
        ["Default form", selectedFormCode],
        ["Tenant", tenantCode(env)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Migration target</h2><p>Select a business year and target version before running migration.</p></div>
          <form id="formMigrationForm" class="stack">
            <label>Business year <select id="formMigrationBy" ${years.length ? "" : "disabled"}>${years.map((item) => `<option value="${escapeHtml(item.by_id)}">${escapeHtml(item.year_label || item.fiscal_year || item.by_id)} / ${escapeHtml(item.status || "-")}</option>`).join("")}</select></label>
            <label>Form code <input id="formMigrationCode" value="${escapeHtml(selectedFormCode)}" /></label>
            <label>Target version <select id="formMigrationVersion" ${versions.length ? "" : "disabled"}>${formVersionOptions(versions, activeVersion?.form_version_id)}</select></label>
            <label>Mode <select id="formMigrationMode"><option value="dry-run">Dry run</option><option value="execute">Execute</option><option value="rollback">Rollback</option></select></label>
            <button class="primary-btn" type="submit" ${years.length && versions.length ? "" : "disabled"}>Run migration</button>
          </form>
          ${renderLeafActionResult()}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Available form versions</h2><p>${escapeHtml(selectedYear ? `Default business year ${selectedYear.year_label || selectedYear.by_id}` : "No business year available")}</p></div>
          ${table(["ID", "Form", "Version", "Status"], versions.map((item) => row([
            escapeHtml(item.form_version_id),
            escapeHtml(item.form_code),
            escapeHtml(item.version_no),
            escapeHtml(item.status),
          ])), "No versions.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formMigrationForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const mode = env.outlet.querySelector("#formMigrationMode").value;
    const result = await request(`/api/form-versioning/migrations/${mode}`, {
      method: "POST",
      body: JSON.stringify({
        tenant_code: tenantCode(env),
        by_id: Number(env.outlet.querySelector("#formMigrationBy").value),
        form_code: env.outlet.querySelector("#formMigrationCode").value,
        to_version_id: Number(env.outlet.querySelector("#formMigrationVersion").value),
      }),
    });
    setLeafActionMessage(`${result.mode || mode}: ${result.message || "migration completed"}`, false, env.locale);
  });
}

async function renderAdminFormEfileMap(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { forms, versions } = await loadAdminFormBase();
  const efileMap = await request("/api/form-versioning/efile-map").catch(() => []);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-efile-map" data-leaf-key="admin/form:efile-map">
      ${adminFormHeader(env, activeLeaf, "E-filing map", "Inspect outbound record mappings between form fields and electronic filing records.")}
      ${metrics([
        ["Mapping rows", money.format(efileMap.length)],
        ["Forms", money.format(forms.length)],
        ["Versions", money.format(versions.length)],
        ["Mapped records", money.format(new Set(efileMap.map((item) => item.record || item.record_type || item.target)).size)],
      ])}
      <article class="panel">
        <div class="panel-head"><h2>Outbound file mapping</h2><p>Record type, field path, target field, and fixed-length metadata used by filing generation.</p></div>
        ${table(["Record", "Form", "Field path", "Length"], efileMap.map((item) => row([
          escapeHtml(item.record || item.record_type || item.target || "-"),
          escapeHtml(item.form_code || item.source_form || "-"),
          escapeHtml(item.field_path || item.field_name || item.target_field || "-"),
          escapeHtml(item.length || item.field_length || "-"),
        ])), "No e-file map rows.")}
      </article>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminFormBySet(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const root = routeRoot(env);
  const { forms, versions } = await loadAdminFormBase();
  const [bySet, years] = await Promise.all([
    request("/api/form-versioning/by-set").catch(() => []),
    request(`${root}/business-years`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-by-set" data-leaf-key="admin/form:by-set">
      ${adminFormHeader(env, activeLeaf, "Business-year form sets", "Review which form version set is applied to each business-year context.")}
      ${metrics([
        ["Form sets", money.format(bySet.length)],
        ["Business years", money.format(years.length)],
        ["Forms", money.format(forms.length)],
        ["Versions", money.format(versions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Version set mapping</h2><p>Configured or resolved form sets by business year.</p></div>
          ${table(["Set", "Business year", "Version", "Status"], bySet.map((item) => row([
            escapeHtml(item.by_set_id || item.form_set_code || item.set_code || "-"),
            escapeHtml(item.year_label || item.by_id || "-"),
            escapeHtml(item.form_version || item.version_no || "-"),
            escapeHtml(item.status || "-"),
          ])), "No business-year form sets.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Tenant business years</h2><p>Available tenant years that can receive locked form snapshots.</p></div>
          ${table(["ID", "Customer", "Year", "Status"], years.map((item) => row([
            escapeHtml(item.by_id),
            escapeHtml(item.customer_id),
            escapeHtml(item.year_label || item.fiscal_year || "-"),
            escapeHtml(item.status || "-"),
          ])), "No business years.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminFormImpact(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { forms, versions, activeVersion } = await loadAdminFormBase();
  const impact = await request("/api/form-versioning/impact").catch(() => ({ affected_business_years: 0, affected_forms: 0, risk: "UNKNOWN" }));
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="form-impact" data-leaf-key="admin/form:impact">
      ${adminFormHeader(env, activeLeaf, "Impact simulation", "Estimate affected forms and open business years before activating a form version change.")}
      ${metrics([
        ["Affected years", money.format(impact.affected_business_years ?? impact.impacted_business_years ?? 0)],
        ["Affected forms", money.format(impact.affected_forms ?? impact.impacted_forms ?? 0)],
        ["Risk", impact.risk || "UNKNOWN"],
        ["Versions", money.format(versions.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Simulation target</h2><p>Runs a server-side form-versioning impact check for the selected target version.</p></div>
          <form id="formImpactForm" class="stack">
            <label>Target version <select id="formImpactVersion" ${versions.length ? "" : "disabled"}>${formVersionOptions(versions, activeVersion?.form_version_id)}</select></label>
            <label>Include locked years <input id="formImpactLocked" type="checkbox" /></label>
            <button class="primary-btn" type="submit" ${versions.length ? "" : "disabled"}>Run simulation</button>
          </form>
          ${renderLeafActionResult()}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Current impact snapshot</h2><p>Latest backend impact summary for form version administration.</p></div>
          ${table(["Item", "Value"], [
            row(["Affected business years", escapeHtml(impact.affected_business_years ?? impact.impacted_business_years ?? 0)]),
            row(["Affected forms", escapeHtml(impact.affected_forms ?? impact.impacted_forms ?? 0)]),
            row(["Risk", escapeHtml(impact.risk || "-")]),
            row(["Known forms", money.format(forms.length)]),
          ])}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
  env.outlet.querySelector("#formImpactForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const result = await request("/api/form-versioning/impact", {
      method: "POST",
      body: JSON.stringify({
        form_version_id: Number(env.outlet.querySelector("#formImpactVersion").value),
        include_locked: env.outlet.querySelector("#formImpactLocked").checked,
      }),
    });
    setLeafActionMessage(`Simulation completed for ${result.affected_forms ?? result.impacted_forms ?? 0} forms.`, false, env.locale);
  });
}

async function renderAdminFormsWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/form:versions") return renderAdminFormVersions(env);
  if (activeLeaf === "admin/form:fields") return renderAdminFormFields(env);
  if (activeLeaf === "admin/form:validations") return renderAdminFormValidations(env);
  if (activeLeaf === "admin/form:linkage-rule") return renderAdminFormLinkageRules(env);
  if (activeLeaf === "admin/form:migration") return renderAdminFormMigration(env);
  if (activeLeaf === "admin/form:efile-map") return renderAdminFormEfileMap(env);
  if (activeLeaf === "admin/form:by-set") return renderAdminFormBySet(env);
  if (activeLeaf === "admin/form:impact") return renderAdminFormImpact(env);
  return renderAdminFormMaster(env);
}

const ADMIN_AUDIT_ROUTES = ["admin/audit:events", "admin/audit:login", "admin/audit:perm", "admin/audit:settings"];

function adminAuditHeader(env, activeLeaf, title, description) {
  return `
    <article class="panel">
      <div class="panel-head">
        <div>
          <span class="badge info">Audit and change review</span>
          <h2>${escapeHtml(title)}</h2>
          <p>${escapeHtml(description)}</p>
        </div>
        <div class="button-row">${renderAdminRouteButtons(activeLeaf, ADMIN_AUDIT_ROUTES, env.locale)}</div>
      </div>
    </article>`;
}

async function loadAdminAuditData(env) {
  const root = routeRoot(env);
  const [logs, verify, loginHistory, permissionHistory, systemSettings] = await Promise.all([
    request(`${root}/audit-logs`).catch(() => []),
    request(`${root}/audit-logs/verify`).catch(() => ({ valid: false, checked: 0, broken: [] })),
    request("/api/login-history").catch(() => []),
    request("/api/permission-change-history").catch(() => []),
    request("/api/system-settings").catch(() => []),
  ]);
  return { root, logs, verify, loginHistory, permissionHistory, systemSettings };
}

async function renderAdminAuditEvents(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { logs, verify, loginHistory, permissionHistory } = await loadAdminAuditData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="audit-events" data-leaf-key="admin/audit:events">
      ${adminAuditHeader(env, activeLeaf, "Audit events", "Review tenant audit records and verify the hash chain for tamper evidence.")}
      ${metrics([
        ["Audit logs", money.format(logs.length)],
        ["Checked", money.format(verify.checked || logs.length)],
        ["Chain", verify.valid ? "HASH OK" : "CHECK"],
        ["Other events", money.format(loginHistory.length + permissionHistory.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Audit log chain</h2><span class="badge ${verify.valid ? "ok" : "error"}">${verify.valid ? "HASH OK" : "HASH CHECK"}</span></div>
          ${table(["ID", "Table", "Record", "Action", "Actor", "Changed"], logs.map((item) => row([
            escapeHtml(item.audit_id),
            escapeHtml(item.table_name),
            escapeHtml(item.record_id),
            escapeHtml(item.action),
            escapeHtml(item.changed_by),
            escapeHtml(item.changed_at || item.event_date || "-"),
          ])), "No audit logs.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Hash verification</h2><p>Broken rows require database-level investigation before filing submission.</p></div>
          ${table(["Audit ID", "Previous hash", "Expected", "Current"], asArray(verify.broken).map((item) => row([
            escapeHtml(item.audit_id),
            escapeHtml(item.prev_hash || "-"),
            escapeHtml(item.expected_prev_hash || "-"),
            escapeHtml(item.hash_current || "-"),
          ])), verify.valid ? "No hash-chain breaks." : "No detailed break rows.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminLoginHistory(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { loginHistory, logs } = await loadAdminAuditData(env);
  const failed = loginHistory.filter((item) => item.success === false).length;
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="audit-login" data-leaf-key="admin/audit:login">
      ${adminAuditHeader(env, activeLeaf, "Login history", "Trace sign-in attempts, source addresses, and authentication outcomes.")}
      ${metrics([
        ["Login events", money.format(loginHistory.length)],
        ["Success", money.format(loginHistory.filter((item) => item.success !== false).length)],
        ["Failed", money.format(failed)],
        ["Audit logs", money.format(logs.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Authentication events</h2><p>User login outcomes and source IP addresses.</p></div>
          ${table(["Login", "Success", "IP", "Reason"], loginHistory.map((item) => row([
            escapeHtml(item.login_id || "-"),
            item.success === false ? "N" : "Y",
            escapeHtml(item.ip_address || "-"),
            escapeHtml(item.fail_reason || item.reason || "-"),
          ])), "No login history.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Review focus</h2><p>Failed attempts and unusual addresses should be reviewed before high-risk filing operations.</p></div>
          ${table(["Metric", "Value"], [
            row(["Failed attempts", money.format(failed)]),
            row(["Unique users", money.format(new Set(loginHistory.map((item) => item.login_id)).size)]),
            row(["Unique IPs", money.format(new Set(loginHistory.map((item) => item.ip_address)).size)]),
          ])}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminPermissionChangeHistory(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { permissionHistory, logs } = await loadAdminAuditData(env);
  const roles = await request("/api/admin/roles").catch(() => []);
  const functionCodes = await request("/api/admin/function-codes").catch(() => []);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="audit-permission" data-leaf-key="admin/audit:perm">
      ${adminAuditHeader(env, activeLeaf, "Permission change history", "Review role and function changes that affect menu access and filing authority.")}
      ${metrics([
        ["Permission events", money.format(permissionHistory.length)],
        ["Roles", money.format(roles.length)],
        ["Functions", money.format(functionCodes.length)],
        ["Audit logs", money.format(logs.length)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Permission events</h2><p>Role/function changes captured for compliance review.</p></div>
          ${table(["Event", "Role", "Function", "Changed by"], permissionHistory.map((item) => row([
            escapeHtml(item.event_id || "-"),
            escapeHtml(item.role_code || "-"),
            escapeHtml(item.function || item.function_code || "-"),
            escapeHtml(item.changed_by || "-"),
          ])), "No permission change history.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Current permission catalog</h2><p>Context for interpreting historical changes.</p></div>
          ${table(["Role", "Name", "System"], roles.map((item) => row([
            escapeHtml(item.role_code),
            escapeHtml(item.role_name),
            item.system_role ? "Y" : "N",
          ])), "No roles.")}
          ${table(["Function", "Name"], functionCodes.slice(0, 8).map((item) => row([
            escapeHtml(item.function_code || item.code || "-"),
            escapeHtml(item.function_name || item.name || "-"),
          ])), "No function codes.")}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminSystemSettingsAudit(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  const { systemSettings, verify, logs } = await loadAdminAuditData(env);
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="audit-settings" data-leaf-key="admin/audit:settings">
      ${adminAuditHeader(env, activeLeaf, "System settings audit", "Inspect global configuration that influences sessions, step-up authentication, and audit review.")}
      ${metrics([
        ["Settings", money.format(systemSettings.length)],
        ["Audit logs", money.format(logs.length)],
        ["Chain", verify.valid ? "HASH OK" : "CHECK"],
        ["Tenant", tenantCode(env)],
      ])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>System settings snapshot</h2><p>Current global configuration visible to administrators.</p></div>
          ${table(["Key", "Value"], systemSettings.map((item) => row([
            escapeHtml(item.setting_key),
            escapeHtml(item.setting_value),
          ])), "No system settings rows.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Audit posture</h2><p>Settings are reviewed alongside the tenant audit hash-chain status.</p></div>
          ${table(["Item", "Value"], [
            row(["Hash chain", verify.valid ? "HASH OK" : "HASH CHECK"]),
            row(["Checked rows", money.format(verify.checked || logs.length)]),
            row(["Broken rows", money.format(asArray(verify.broken).length)]),
            row(["Tenant", escapeHtml(tenantCode(env))]),
          ])}
        </article>
      </section>
    </section>`;
  bindAdminRouteButtons(env);
}

async function renderAdminAuditWorkbench(env) {
  const activeLeaf = env.routeKey || env.leafKey || env.key;
  if (activeLeaf === "admin/audit:login") return renderAdminLoginHistory(env);
  if (activeLeaf === "admin/audit:perm") return renderAdminPermissionChangeHistory(env);
  if (activeLeaf === "admin/audit:settings") return renderAdminSystemSettingsAudit(env);
  return renderAdminAuditEvents(env);
}

async function renderAdminCodes(env) {
  const locale = env.locale || currentDocumentLocale();
  const root = routeRoot(env);
  const state = leafViewState.get("admin/code:manage") || { group: "ALL" };
  leafViewState.set("admin/code:manage", state);
  const [tenantCodesResponse, customCodesResponse, functionCodes] = await Promise.all([
    request(`${root}/codes?group=${encodeURIComponent(state.group)}`),
    request(`${root}/leaf-records?leaf_key=admin/code:manage`).catch(() => ({ rows: [] })),
    request("/api/admin/function-codes").catch(() => []),
  ]);
  const tenantCodes = asArray(tenantCodesResponse).map((item) => ({ ...item, source: "MASTER" }));
  const customCodes = asArray(customCodesResponse.rows)
    .filter((item) => state.group === "ALL" || item.group === state.group)
    .map((item) => ({ ...item, source: "TENANT" }));
  const codeRows = [...tenantCodes, ...customCodes];
  const groups = ["ALL", "INDUSTRY", "ACCOUNT", "TAX", "FORM", "WORK_SCOPE", "STATUS"];
  env.outlet.innerHTML = `
    <section class="leaf-workbench admin-settings-workbench" data-admin-stage="codes" data-leaf-key="admin/code:manage">
      <section class="panel leaf-summary" data-leaf-block="summary">
        <div class="panel-head">
          <div>
            <span class="badge info">${escapeHtml(t(locale, "route.admin.code.manage"))}</span>
            <h2>코드 관리</h2>
            <p>테넌트 업무 코드와 공통 기능 코드를 분리해서 조회하고, 테넌트 전용 코드를 추가합니다.</p>
          </div>
        </div>
        ${metrics([
          ["Tenant codes", money.format(codeRows.length)],
          ["Function codes", money.format(functionCodes.length)],
          ["Group", state.group],
          ["Custom", money.format(customCodes.length)],
        ])}
      </section>
      <section class="grid two">
        <article class="panel leaf-table" data-leaf-block="table">
          <div class="panel-head">
            <div><h2>Tenant code registry</h2><p>Codes used by customer, workflow, form, and adjustment screens.</p></div>
            <div class="panel-head-actions" data-leaf-block="toolbar">
              <label class="inline-control">Group
                <select id="adminCodeGroup">
                  ${groups.map((group) => `<option value="${group}" ${state.group === group ? "selected" : ""}>${group}</option>`).join("")}
                </select>
              </label>
            </div>
          </div>
          ${table(["Group", "Code", "Label", "Tenant", "Source"], codeRows.map((item) => row([
            escapeHtml(item.group || state.group),
            escapeHtml(item.code || item.row_id || "-"),
            escapeHtml(item.label || item.name || item.value || "-"),
            escapeHtml(item.tenant_code || tenantCode(env)),
            escapeHtml(item.source || "MASTER"),
          ])), "등록된 코드가 없습니다.")}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Function codes</h2><p>Menu action catalog used by permission screens.</p></div>
          ${table(["Code", "Name", "Sort"], functionCodes.map((item) => row([
            escapeHtml(item.function_code || item.code || "-"),
            escapeHtml(item.function_name || item.name || "-"),
            money.format(item.sort_order || 0),
          ])), "기능 코드가 없습니다.")}
        </article>
      </section>
      <form class="panel inline-form" id="adminCodeForm">
        <div class="panel-head"><h2>Tenant code add</h2><p>Stores tenant-specific codes without changing the global master.</p></div>
        <label>Group <input name="group" value="${escapeHtml(state.group === "ALL" ? "CUSTOM" : state.group)}" /></label>
        <label>Code <input name="code" required placeholder="CODE_001" /></label>
        <label>Label <input name="label" required placeholder="Display label" /></label>
        <button class="primary-btn compact" type="submit">${escapeHtml(t(locale, "common.save"))}</button>
        ${renderLeafActionResult()}
      </form>
    </section>`;
  env.outlet.querySelector("#adminCodeGroup")?.addEventListener("change", async (event) => {
    state.group = event.target.value || "ALL";
    leafViewState.set("admin/code:manage", state);
    await renderAdminCodes(env);
  });
  env.outlet.querySelector("#adminCodeForm")?.addEventListener("submit", async (event) => {
    event.preventDefault();
    const form = event.currentTarget;
    const values = Object.fromEntries(new FormData(form).entries());
    await request(`${root}/leaf-records`, {
      method: "POST",
      body: JSON.stringify({
        leaf_key: "admin/code:manage",
        data: {
          group: String(values.group || "CUSTOM").trim(),
          code: String(values.code || "").trim(),
          label: String(values.label || "").trim(),
          status: "ACTIVE",
        },
      }),
    });
    await renderAdminCodes(env);
  });
}

async function renderAdminRoles(env) {
  return renderAdminRolesWorkbench(env);
  const [roles, permissions, functionCodes, roleMenuFunctions] = await Promise.all([
    request("/api/admin/roles"),
    request("/api/admin/role-permissions"),
    request("/api/admin/function-codes").catch(() => []),
    request("/api/admin/role-menu-functions").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      <section class="grid two">
      <article class="panel">${table(["역할", "이름", "시스템"], roles.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.role_name), item.system_role ? "Y" : "N"])))}</article>
      <article class="panel">
        <div class="panel-head"><h2>권한 매트릭스</h2><button id="saveExpertPerm" class="primary-btn compact" type="button">TAX_EXPERT 저장</button></div>
        ${table(["역할", "모듈", "기능", "효과"], permissions.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.module_code), escapeHtml(item.function_code), escapeHtml(item.effect)])))}
      </article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Function Codes</h2></div>
          ${table(["Code", "Name", "Sort"], functionCodes.map((item) => row([escapeHtml(item.function_code), escapeHtml(item.function_name), escapeHtml(item.sort_order)])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Role Menu Functions</h2></div>
          ${table(["Role", "Menu", "Function", "Effect"], roleMenuFunctions.map((item) => row([escapeHtml(item.role_code), escapeHtml(item.menu_key), escapeHtml(item.function_code), escapeHtml(item.effect)])))}
        </article>
      </section>
    </section>`;
  document.getElementById("saveExpertPerm").addEventListener("click", async () => {
    await request("/api/admin/roles/TAX_EXPERT/permissions", {
      method: "PUT",
      body: JSON.stringify({ permissions: [
        { module_code: "tax-data", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "adjustment", function_code: "WRITE", effect: "ALLOW" },
        { module_code: "efiling", function_code: "EFILE", effect: "ALLOW" },
      ] }),
    });
    await renderAdminRoles(env);
  });
}

async function renderAdminMenus(env) {
  return renderAdminMenusWorkbench(env);
  const locale = env.locale || currentDocumentLocale();
  const [menus, menuFunctions] = await Promise.all([
    request("/api/admin/menus"),
    request("/api/admin/menu-functions").catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid two">
      <article class="panel">
        <div class="panel-head"><h2>메뉴/기능 관리</h2></div>
        ${table(["키", "상위", "라벨", "권한", "플래그", "사용", ""], menus.map((item) => row([
          escapeHtml(item.menu_key),
          escapeHtml(item.parent_key || "-"),
          escapeHtml(item.label),
          escapeHtml([item.required_perm_module, item.required_perm_function].filter(Boolean).join(":") || "-"),
          `<input value="${escapeHtml(item.feature_flag || "")}" data-menu-flag="${escapeHtml(item.menu_key)}" />`,
          item.enabled ? "Y" : "N",
          `<button class="secondary-btn compact" data-save-menu="${escapeHtml(item.menu_key)}" type="button">${escapeHtml(t(locale, "common.save"))}</button>`,
        ])))}
      </article>
      <article class="panel">
        <div class="panel-head"><h2>Menu Functions</h2></div>
        ${table(["Menu", "Function", "Label", "Enabled"], menuFunctions.map((item) => row([
          escapeHtml(item.menu_key),
          escapeHtml(item.function_code),
          escapeHtml(item.function_name || item.label || "-"),
          item.enabled ? "Y" : "N",
        ])))}
      </article>
    </section>`;
  document.querySelectorAll("[data-save-menu]").forEach((button) => {
    button.addEventListener("click", async () => {
      const input = document.querySelector(`[data-menu-flag="${CSS.escape(button.dataset.saveMenu)}"]`);
      await request(`/api/admin/menus/${button.dataset.saveMenu}`, {
        method: "PUT",
        body: JSON.stringify({ feature_flag: input.value || null, enabled: true }),
      });
      await renderAdminMenus(env);
    });
  });
}

async function renderAdminCustomerAccess(env) {
  return renderAdminCustomerAccessWorkbench(env);
  const root = routeRoot(env);
  const [users, customers, delegations] = await Promise.all([
    request(`${root.replace("/api/tenants", "/api/admin/tenants")}/users`),
    request(`${root}/customers`),
    request(`${root}/access-delegations`).catch(() => []),
  ]);
  env.outlet.innerHTML = `
    <section class="grid">
      <section class="grid two">
      <article class="panel">${table(["사용자", "고객사", "권한", "업무범위"], users.flatMap((user) => asArray(user.customer_access).map((access) => {
        const customer = customers.find((item) => item.customer_id === access.customer_id);
        return row([escapeHtml(user.login_id), escapeHtml(customer?.customer_name || access.customer_id), escapeHtml(access.access_level), escapeHtml(asArray(access.work_scopes).join(", "))]);
      })))}</article>
      <article class="panel">${table(["고객사", "업무범위"], customers.map((item) => row([
        escapeHtml(item.customer_name || item.customer_id),
        escapeHtml(asArray(item.work_scopes).join(", ") || "-"),
      ])))}</article>
      </section>
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>Delegations</h2></div>
          ${table(["Grantor", "Delegatee", "Customer", "Scope", "Period"], delegations.map((item) => row([
            escapeHtml(item.grantor_login_id),
            escapeHtml(item.delegatee_login_id),
            escapeHtml(item.customer_id),
            escapeHtml(item.work_scope),
            `${escapeHtml(item.valid_from || "-")} ~ ${escapeHtml(item.valid_to || "-")}`,
          ])))}
        </article>
        <article class="panel">
          <div class="panel-head"><h2>Create Delegation</h2></div>
          <form id="delegationForm" class="stack">
            <label>Grantor <input id="delegationGrantor" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Delegatee <input id="delegationDelegatee" value="${escapeHtml(env.auth.user.login_id)}" /></label>
            <label>Customer <select id="delegationCustomer">${customers.map((item) => `<option value="${item.customer_id}">${escapeHtml(item.customer_name)}</option>`).join("")}</select></label>
            <label>Scope <select id="delegationScope"><option>INFO</option><option>ADJUST</option><option>FORM</option><option>VALIDATE</option><option>APPROVE</option><option>PRINT</option><option>EFILE</option><option>POST</option></select></label>
            <label>Valid to <input id="delegationValidTo" type="date" value="${today()}" /></label>
            <button class="primary-btn" type="submit" ${customers.length ? "" : "disabled"}>Create</button>
          </form>
        </article>
      </section>
    </section>`;
  document.getElementById("delegationForm").addEventListener("submit", async (event) => {
    event.preventDefault();
    await request(`${root}/access-delegations`, {
      method: "POST",
      body: JSON.stringify({
        grantor_login_id: document.getElementById("delegationGrantor").value,
        delegatee_login_id: document.getElementById("delegationDelegatee").value,
        customer_id: Number(document.getElementById("delegationCustomer").value),
        work_scope: document.getElementById("delegationScope").value,
        valid_from: today(),
        valid_to: document.getElementById("delegationValidTo").value || null,
        reason: uiText(env.locale, "관리자 위임", "admin delegation"),
      }),
    });
    await renderAdminCustomerAccess(env);
  });
}

async function renderAdminLaw(env) {
  return renderAdminLawWorkbench(env);
  const [laws, summary] = await Promise.all([
    request("/api/tax-laws"),
    request("/api/law-versioning/summary"),
  ]);
  const activeLaw = laws[0];
  const rates = activeLaw ? await request(`/api/tax-rates?law_version_id=${activeLaw.law_version_id}`) : [];
  const limits = activeLaw ? await request(`/api/tax-limits?law_version_id=${activeLaw.law_version_id}`) : [];
  env.outlet.innerHTML = `
    <section class="grid">
      ${metrics([["법령", summary.laws || laws.length], ["세율", summary.rates || rates.length], ["한도", summary.limits || limits.length], ["활성 버전", activeLaw?.version_code || "-"]])}
      <section class="grid two">
        <article class="panel">
          <div class="panel-head"><h2>법령 버전</h2><button id="createLaw" class="primary-btn compact" type="button">등록</button></div>
          ${table(["ID", "버전", "상태", "기간"], laws.map((item) => row([escapeHtml(item.law_version_id), escapeHtml(item.version_code), escapeHtml(item.status), `${item.effective_from} ~ ${item.effective_to || ""}`])))}
        </article>
        <article class="panel">${table(["항목", "구간", "율/금액"], rates.slice(0, 10).map((item) => row([escapeHtml(item.item_code), `${money.format(item.taxable_from)} ~ ${item.taxable_to ? money.format(item.taxable_to) : ""}`, `${(item.rate_bps / 100).toFixed(2)}%`])).concat(limits.slice(0, 10).map((item) => row([escapeHtml(item.item_code), "한도", money.format(item.amount)]))))}</article>
      </section>
    </section>`;
  document.getElementById("createLaw").addEventListener("click", async () => {
    const suffix = Date.now().toString(36).slice(-4).toUpperCase();
    await request("/api/tax-laws", { method: "POST", body: JSON.stringify({ version_code: `CIT-${new Date().getFullYear()}-${suffix}`, law_name: "법인세법 개정", effective_from: `${new Date().getFullYear()}-01-01`, effective_to: null, metadata: { source: "admin-ui" } }) });
    await renderAdminLaw(env);
  });
}

async function renderAdminForms(env) {
  return renderAdminFormsWorkbench(env);
  const [forms, versions, relationships, cycleCheck] = await Promise.all([
    request("/api/form-versioning/forms"),
    request("/api/form-versioning/versions"),
    request("/api/form-versioning/relationships"),
    request("/api/form-versioning/cycle-check").catch(() => ({ valid: false })),
  ]);
  env.outlet.innerHTML = `
    <section class="grid three">
      <article class="panel"><div class="panel-head"><h2>서식</h2></div>${table(["코드", "이름", "활성"], forms.map((item) => row([escapeHtml(item.form_code), escapeHtml(item.form_name), item.active ? "Y" : "N"])))}</article>
      <article class="panel"><div class="panel-head"><h2>버전</h2></div>${table(["ID", "서식", "버전", "상태"], versions.map((item) => row([escapeHtml(item.form_version_id), escapeHtml(item.form_code), escapeHtml(item.version_no), escapeHtml(item.status)])))}</article>
      <article class="panel"><div class="panel-head"><h2>연동</h2><span class="badge ${cycleCheck.valid ? "ok" : "error"}">${cycleCheck.valid ? "ACYCLIC" : "CYCLE"}</span></div>${table(["원천", "대상", "규칙"], relationships.map((item) => row([`${escapeHtml(item.source_form)}.${escapeHtml(item.source_field)}`, `${escapeHtml(item.target_form)}.${escapeHtml(item.target_field)}`, escapeHtml(JSON.stringify(item.rule_json))])))}</article>
    </section>`;
}

async function renderAdminAudit(env) {
  return renderAdminAuditWorkbench(env);
  const [logs, verify] = await Promise.all([
    request(`${routeRoot(env)}/audit-logs`),
    request(`${routeRoot(env)}/audit-logs/verify`).catch(() => ({ valid: false })),
  ]);
  env.outlet.innerHTML = `
    <section class="panel">
      <div class="panel-head"><h2>감사/로그</h2><span class="badge ${verify.valid ? "ok" : "error"}">${verify.valid ? "HASH OK" : "HASH CHECK"}</span></div>
      ${table(["ID", "테이블", "작업", "사용자", "해시"], logs.map((item) => row([escapeHtml(item.audit_id), escapeHtml(item.table_name), escapeHtml(item.action), escapeHtml(item.changed_by), escapeHtml(item.hash_current || "-")])))}
    </section>`;
}
