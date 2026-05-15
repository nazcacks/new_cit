use serde_json::{json, Value};

pub fn module_tree() -> Value {
    json!({
        "code": "cit-system",
        "name": "CIT System",
        "display_name": "CIT System",
        "description": "법인세 세무조정계산서 시스템 모듈 구조",
        "path": "/",
        "implemented": true,
        "children": [
            module(
                "law-versioning",
                "0",
                "법령·세율 버전 관리 모듈",
                "Tax Law Versioning",
                Some("★"),
                "/modules/law-versioning",
                vec![
                    submodule("law-versioning.laws", "0.1", "법령 버전 마스터", "/modules/law-versioning/laws"),
                    submodule("law-versioning.rates", "0.2", "법인세율표", "/modules/law-versioning/rates"),
                    submodule("law-versioning.limits", "0.3", "한도·율표", "/modules/law-versioning/limits"),
                    submodule("law-versioning.credits", "0.4", "세액공제·감면 율표", "/modules/law-versioning/credits"),
                    submodule("law-versioning.depreciation-lives", "0.5", "기준내용연수표", "/modules/law-versioning/depreciation-lives"),
                    submodule("law-versioning.sme-criteria", "0.6", "중소기업 판정기준", "/modules/law-versioning/sme-criteria"),
                    submodule("law-versioning.loss-rules", "0.7", "결손금 공제규정", "/modules/law-versioning/loss-rules"),
                    submodule("law-versioning.snapshots", "0.8", "사업연도별 적용 스냅샷", "/modules/law-versioning/snapshots"),
                    submodule("law-versioning.impact", "0.9", "영향 시뮬레이션", "/modules/law-versioning/impact"),
                    submodule("law-versioning.history", "0.10", "개정 공지/이력", "/modules/law-versioning/history"),
                ],
            ),
            module(
                "auth",
                "1",
                "인증/계정 모듈",
                "Auth Module",
                None,
                "/modules/auth",
                vec![],
            ),
            module(
                "admin",
                "2",
                "시스템 관리 모듈",
                "Admin Module",
                None,
                "/modules/admin",
                vec![
                    submodule("admin.users", "2.1", "사용자 관리", "/modules/admin/users"),
                    submodule("admin.roles", "2.2", "권한/역할 관리", "/modules/admin/roles"),
                    submodule("admin.menus", "2.3", "메뉴 관리", "/modules/admin/menus"),
                    submodule("admin.tenants", "2.4", "테넌트 관리", "/modules/admin/tenants"),
                    submodule("admin.audit-logs", "2.5", "감사 로그", "/modules/admin/audit-logs"),
                ],
            ),
            module(
                "customer",
                "3",
                "고객사 관리 모듈",
                "Customer Module",
                None,
                "/modules/customer",
                vec![
                    submodule("customer.profile", "3.1", "법인 기본정보", "/modules/customer/profile"),
                    submodule("customer.business-years", "3.2", "사업연도 관리", "/modules/customer/business-years"),
                    submodule("customer.contracts", "3.3", "세무대리 계약", "/modules/customer/contracts"),
                ],
            ),
            module(
                "tax-data",
                "4",
                "세무정보 입력 모듈",
                "Tax Data Input",
                None,
                "/modules/tax-data",
                vec![
                    submodule("tax-data.financial-statements", "4.1", "재무제표 입력/임포트", "/modules/tax-data/financial-statements"),
                    submodule("tax-data.account-mapping", "4.2", "계정과목 매핑", "/modules/tax-data/account-mapping"),
                    submodule("tax-data.partners", "4.3", "거래 명세", "/modules/tax-data/partners"),
                    submodule("tax-data.assets", "4.4", "자산/감가상각 정보", "/modules/tax-data/assets"),
                ],
            ),
            module(
                "adjustment",
                "5",
                "세무조정 모듈",
                "Tax Adjustment",
                None,
                "/modules/adjustment",
                vec![
                    submodule("adjustment.income", "5.1", "소득금액조정", "/modules/adjustment/income"),
                    submodule("adjustment.donations-entertainment", "5.2", "기부금/접대비", "/modules/adjustment/donations-entertainment"),
                    submodule("adjustment.depreciation", "5.3", "감가상각", "/modules/adjustment/depreciation"),
                    submodule("adjustment.retirement-reserve", "5.4", "퇴직급여충당금", "/modules/adjustment/retirement-reserve"),
                    submodule("adjustment.bad-debt-reserve", "5.5", "대손충당금", "/modules/adjustment/bad-debt-reserve"),
                    submodule("adjustment.fx-valuation", "5.6", "외화평가", "/modules/adjustment/fx-valuation"),
                    submodule("adjustment.inventory-valuation", "5.7", "재고·유가증권 평가", "/modules/adjustment/inventory-valuation"),
                    submodule("adjustment.carryforward-loss", "5.8", "이월결손금", "/modules/adjustment/carryforward-loss"),
                    submodule("adjustment.tax-credits", "5.9", "세액공제/감면", "/modules/adjustment/tax-credits"),
                    submodule("adjustment.minimum-tax", "5.10", "최저한세", "/modules/adjustment/minimum-tax"),
                    submodule("adjustment.penalty-tax", "5.11", "가산세", "/modules/adjustment/penalty-tax"),
                    submodule("adjustment.capital-reserves", "5.12", "자본금과 적립금", "/modules/adjustment/capital-reserves"),
                ],
            ),
            module(
                "forms",
                "6",
                "서식 생성 모듈",
                "Form Generation",
                None,
                "/modules/forms",
                vec![
                    submodule("forms.versions", "6.0", "서식 버전 관리", "/modules/forms/versions"),
                    submodule("forms.relationships", "6.0.1", "서식 항목 매핑", "/modules/forms/relationships"),
                    submodule("forms.migrations", "6.0.2", "서식 데이터 마이그레이션", "/modules/forms/migrations"),
                    submodule("forms.resolver", "6.0.3", "사업연도 적용 서식", "/modules/forms/resolver"),
                    submodule("forms.form3", "6.1", "과세표준 및 세액조정계산서 (별지 제3호)", "/modules/forms/form3"),
                    submodule("forms.attachments", "6.2", "100여 종 부속서식", "/modules/forms/attachments"),
                    submodule("forms.linkage", "6.3", "서식 간 데이터 연동", "/modules/forms/linkage"),
                    submodule("forms.preview", "6.4", "미리보기", "/modules/forms/preview"),
                ],
            ),
            module(
                "print",
                "7",
                "출력 모듈",
                "Print Module",
                None,
                "/modules/print",
                vec![
                    submodule("print.pdf", "7.1", "PDF 생성 (JasperReports)", "/modules/print/pdf"),
                    submodule("print.batch", "7.2", "일괄 인쇄", "/modules/print/batch"),
                    submodule("print.watermark", "7.3", "워터마크/봉인", "/modules/print/watermark"),
                ],
            ),
            module(
                "efiling",
                "8",
                "전자신고 모듈",
                "e-Filing",
                None,
                "/modules/efiling",
                vec![
                    submodule("efiling.hometax-record", "8.1", "홈택스 전자신고 레코드 파일 생성", "/modules/efiling/hometax-record"),
                    submodule("efiling.validation", "8.2", "검증 및 오류 점검", "/modules/efiling/validation"),
                    submodule("efiling.history", "8.3", "신고 이력 관리", "/modules/efiling/history"),
                ],
            ),
        ]
    })
}

fn module(
    code: &str,
    number: &str,
    name: &str,
    name_en: &str,
    marker: Option<&str>,
    path: &str,
    children: Vec<Value>,
) -> Value {
    let suffix = marker.map(|value| format!(" {value}")).unwrap_or_default();

    json!({
        "code": code,
        "number": number,
        "name": name,
        "name_en": name_en,
        "display_name": format!("{number}. {name} ({name_en}){suffix}"),
        "path": path,
        "implemented": true,
        "children": children,
    })
}

fn submodule(code: &str, number: &str, name: &str, path: &str) -> Value {
    json!({
        "code": code,
        "number": number,
        "name": name,
        "display_name": format!("{number} {name}"),
        "path": path,
        "implemented": true,
        "children": [],
    })
}
