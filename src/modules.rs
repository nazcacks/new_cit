use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct MenuNodeSeed {
    pub key: String,
    pub parent_key: Option<String>,
    pub label: String,
    pub path: String,
    pub layout: String,
    pub requires_context: Vec<String>,
    pub required_perm_module: Option<String>,
    pub required_perm_function: Option<String>,
    pub feature_flag: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
struct MenuDef {
    key: String,
    parent_key: Option<String>,
    label: String,
    path: String,
    icon: String,
    layout: String,
    requires_context: Vec<String>,
    required_perm_module: Option<String>,
    required_perm_function: Option<String>,
    feature_flag: Option<String>,
    sort_order: i32,
}

pub fn module_tree() -> Value {
    prototype_menu_tree()
}

pub fn prototype_menu_tree() -> Value {
    let defs = prototype_menu_defs();
    json!({
        "code": "cit-system",
        "name": "CIT System",
        "display_name": "CIT System",
        "description": "Corporate income tax workspace menu",
        "path": "#/dashboard/overview",
        "implemented": true,
        "version": "v1.2",
        "children": children_for(&defs, None),
    })
}

pub fn legacy_module_tree() -> Value {
    json!({
        "code": "cit-system-legacy",
        "name": "CIT System Legacy Module Tree",
        "display_name": "CIT System Legacy Module Tree",
        "description": "Phase 1 module-number menu retained for audit and development",
        "path": "/modules",
        "implemented": true,
        "children": [
            legacy_node("law-versioning", "0", "Law/rate versioning", "/modules/law-versioning"),
            legacy_node("auth", "1", "Authentication/accounts", "/modules/auth"),
            legacy_node("admin", "2", "System administration", "/modules/admin"),
            legacy_node("customer", "3", "Customer management", "/modules/customer"),
            legacy_node("tax-data", "4", "Tax data input", "/modules/tax-data"),
            legacy_node("adjustment", "5", "Tax adjustments", "/modules/adjustment"),
            legacy_node("forms", "6", "Tax forms", "/modules/forms"),
            legacy_node("print", "7", "Print/output", "/modules/print"),
            legacy_node("efiling", "8", "Electronic filing", "/modules/efiling"),
            legacy_node("reports", "9", "Analytics/reports", "/modules/reports"),
        ]
    })
}

pub fn prototype_menu_seeds() -> Vec<MenuNodeSeed> {
    prototype_menu_defs()
        .into_iter()
        .map(|def| {
            let label = korean_label(&def.key, &def.label).to_string();
            MenuNodeSeed {
                key: def.key,
                parent_key: def.parent_key,
                label,
                path: def.path,
                layout: def.layout,
                requires_context: def.requires_context,
                required_perm_module: def.required_perm_module,
                required_perm_function: def.required_perm_function,
                feature_flag: def.feature_flag,
                sort_order: def.sort_order,
            }
        })
        .collect()
}

fn prototype_menu_defs() -> Vec<MenuDef> {
    let mut defs = vec![
        group(
            "dashboard",
            None,
            "Dashboard",
            "#/dashboard/overview",
            "layout-dashboard",
            "plain",
            10,
        ),
        group(
            "workspace",
            None,
            "Tax workspace",
            "#/workspace/ws/start/customer-pick",
            "workflow",
            "workspace",
            20,
        ),
        group(
            "post",
            None,
            "Post filing",
            "#/post/hist/list",
            "history",
            "plain",
            30,
        ),
        group(
            "reports",
            None,
            "Analytics/reports",
            "#/report/year-compare",
            "bar-chart-3",
            "plain",
            40,
        ),
        group(
            "admin",
            None,
            "Administration",
            "#/admin/cust/list",
            "settings",
            "admin",
            50,
        ),
        group(
            "ws/start",
            Some("workspace"),
            "0. Start",
            "#/workspace/ws/start/customer-pick",
            "play",
            "workspace",
            210,
        ),
        group(
            "ws/info",
            Some("workspace"),
            "1. Tax data input",
            "#/workspace/ws/info/fs",
            "database",
            "workspace",
            220,
        ),
        group(
            "ws/adj",
            Some("workspace"),
            "2. Tax adjustments",
            "#/workspace/ws/adj/B1",
            "calculator",
            "workspace",
            230,
        ),
        group(
            "ws/form",
            Some("workspace"),
            "3. Forms",
            "#/workspace/ws/form/form3",
            "file-text",
            "workspace",
            240,
        ),
        group(
            "ws/val",
            Some("workspace"),
            "4. Validation",
            "#/workspace/ws/val/run",
            "shield-check",
            "workspace",
            250,
        ),
        group(
            "ws/appr",
            Some("workspace"),
            "5. Approval",
            "#/workspace/ws/appr/request",
            "badge-check",
            "workspace",
            260,
        ),
        group(
            "ws/print",
            Some("workspace"),
            "6. Print",
            "#/workspace/ws/print/preview",
            "printer",
            "workspace",
            270,
        ),
        group(
            "ws/file",
            Some("workspace"),
            "7. E-file",
            "#/workspace/ws/file/precheck",
            "send",
            "workspace",
            280,
        ),
        group(
            "post/hist",
            Some("post"),
            "1. Filing history",
            "#/post/hist/list",
            "list-checks",
            "plain",
            310,
        ),
        group(
            "post/amend",
            Some("post"),
            "2. Amend/correct",
            "#/post/amend/unlock",
            "unlock-keyhole",
            "plain",
            320,
        ),
        group(
            "admin/cust",
            Some("admin"),
            "5-A. Customer",
            "#/admin/cust/list",
            "briefcase-business",
            "admin",
            510,
        ),
        group(
            "admin/sec",
            Some("admin"),
            "5-B. Security",
            "#/admin/sec/users",
            "key-round",
            "admin",
            520,
        ),
        group(
            "admin/cacc",
            Some("admin"),
            "5-C. Customer access",
            "#/admin/cacc/assign",
            "user-check",
            "admin",
            530,
        ),
        group(
            "admin/law",
            Some("admin"),
            "5-D. Law/rate",
            "#/admin/law/master",
            "scale",
            "admin",
            540,
        ),
        group(
            "admin/form",
            Some("admin"),
            "5-E. Form versioning",
            "#/admin/form/master",
            "files",
            "admin",
            550,
        ),
        group(
            "admin/code",
            Some("admin"),
            "5-F. Codes",
            "#/admin/code/manage",
            "list-tree",
            "admin",
            560,
        ),
        group(
            "admin/audit",
            Some("admin"),
            "5-G. Audit",
            "#/admin/audit/events",
            "scroll-text",
            "admin",
            570,
        ),
    ];

    defs.extend(dashboard_defs());
    defs.extend(workspace_start_defs());
    defs.extend(workspace_info_defs());
    defs.extend(workspace_adjustment_defs());
    defs.extend(workspace_form_defs());
    defs.extend(workspace_validation_defs());
    defs.extend(workspace_approval_defs());
    defs.extend(workspace_print_defs());
    defs.extend(workspace_efile_defs());
    defs.extend(post_defs());
    defs.extend(report_defs());
    defs.extend(admin_customer_defs());
    defs.extend(admin_security_defs());
    defs.extend(admin_customer_access_defs());
    defs.extend(admin_law_defs());
    defs.extend(admin_form_defs());
    defs.extend(admin_code_defs());
    defs.extend(admin_audit_defs());
    defs
}

fn dashboard_defs() -> Vec<MenuDef> {
    [
        ("overview", "Overview"),
        ("duesoon", "Due soon"),
        ("inbox", "Inbox"),
        ("recent", "Recent activity"),
        ("kpi-tax", "KPI"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label))| {
        leaf(
            &format!("dashboard:{suffix}"),
            Some("dashboard"),
            label,
            "layout-dashboard",
            "plain",
            &[],
            "dashboard",
            "READ",
            100 + index as i32,
        )
    })
    .collect()
}

fn workspace_start_defs() -> Vec<MenuDef> {
    [
        ("customer-pick", "Customer selection", "READ"),
        ("by-pick", "Business year selection/new", "READ"),
        ("snapshot", "Law snapshot", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/start:{suffix}"),
            Some("ws/start"),
            label,
            "play",
            "workspace",
            &[],
            "customer",
            function,
            211 + index as i32,
        )
    })
    .collect()
}

fn workspace_info_defs() -> Vec<MenuDef> {
    [
        ("fs", "Financial statements", "READ"),
        ("mapping", "Account mapping", "UPDATE"),
        ("assets", "Asset register", "READ"),
        ("transactions", "Transactions", "READ"),
        ("vehicle", "Vehicle usage", "READ"),
        ("consistency", "Consistency check", "CALCULATE"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/info:{suffix}"),
            Some("ws/info"),
            label,
            "database",
            "workspace",
            work_context(),
            "tax-data",
            function,
            221 + index as i32,
        )
    })
    .collect()
}

fn workspace_adjustment_defs() -> Vec<MenuDef> {
    [
        ("B1", "B1 Income add/deduct"),
        ("B2", "B2 Donations"),
        ("B3", "B3 Entertainment expense"),
        ("B4", "B4 Depreciation"),
        ("B5", "B5 Deemed interest"),
        ("B6", "B6 Retirement allowance reserve"),
        ("B7", "B7 Bad debt reserve"),
        ("B8", "B8 Currency valuation"),
        ("B9", "B9 Inventory/securities valuation"),
        ("B10", "B10 Business transfer difference"),
        ("B11", "B11 Loss carryforward"),
        ("B12", "B12 Tax credits"),
        ("B13", "B13 Minimum tax"),
        ("B14", "B14 Additional tax"),
        ("B15", "B15 Capital/equity"),
        ("B16", "B16 Foreign corporation"),
        ("B17", "B17 Consolidated tax"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (code, label))| {
        leaf(
            &format!("ws/adj:{code}"),
            Some("ws/adj"),
            label,
            "calculator",
            "workspace",
            work_context(),
            "adjustment",
            "CALCULATE",
            231 + index as i32,
        )
    })
    .collect()
}

fn workspace_form_defs() -> Vec<MenuDef> {
    [
        ("form3", "Form 3 main statement", "CREATE"),
        ("attachments", "Attachments", "CREATE"),
        ("preview", "Preview", "READ"),
        ("linkage", "Form linkage", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/form:{suffix}"),
            Some("ws/form"),
            label,
            "file-text",
            "workspace",
            work_context(),
            "forms",
            function,
            241 + index as i32,
        )
    })
    .collect()
}

fn workspace_validation_defs() -> Vec<MenuDef> {
    [
        ("run", "Run validation", "CALCULATE"),
        ("issues", "Validation issues", "READ"),
        ("rules", "Validation rules", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/val:{suffix}"),
            Some("ws/val"),
            label,
            "shield-check",
            "workspace",
            work_context(),
            "validation",
            function,
            251 + index as i32,
        )
    })
    .collect()
}

fn workspace_approval_defs() -> Vec<MenuDef> {
    [
        ("request", "Request approval", "APPROVE"),
        ("inbox", "Approval inbox", "READ"),
        ("rejected", "Rejected items", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/appr:{suffix}"),
            Some("ws/appr"),
            label,
            "badge-check",
            "workspace",
            work_context(),
            "workflow",
            function,
            261 + index as i32,
        )
    })
    .collect()
}

fn workspace_print_defs() -> Vec<MenuDef> {
    [
        ("preview", "Print preview", "READ"),
        ("bulk", "Bulk print", "PRINT"),
        ("history", "Print history", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/print:{suffix}"),
            Some("ws/print"),
            label,
            "printer",
            "workspace",
            work_context(),
            "forms",
            function,
            271 + index as i32,
        )
    })
    .collect()
}

fn workspace_efile_defs() -> Vec<MenuDef> {
    [
        ("precheck", "E-file precheck", "READ"),
        ("generate", "Generate e-file", "EFILE"),
        ("submit", "Submit e-file", "EFILE"),
        ("done", "Submission result", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        leaf(
            &format!("ws/file:{suffix}"),
            Some("ws/file"),
            label,
            "send",
            "workspace",
            work_context(),
            "efiling",
            function,
            281 + index as i32,
        )
    })
    .collect()
}

fn post_defs() -> Vec<MenuDef> {
    let mut defs = [
        (
            "post/hist:list",
            Some("post/hist"),
            "Filing history list",
            "efiling",
            "READ",
            311,
        ),
        (
            "post/amend:unlock",
            Some("post/amend"),
            "Unlock for amendment",
            "post",
            "UPDATE",
            321,
        ),
        (
            "post/amend:version",
            Some("post/amend"),
            "Amendment version",
            "post",
            "CREATE",
            322,
        ),
        (
            "post/amend:diff",
            Some("post/amend"),
            "Amendment diff",
            "post",
            "READ",
            323,
        ),
        (
            "post/amend:resubmit",
            Some("post/amend"),
            "Resubmit amendment",
            "post",
            "EFILE",
            324,
        ),
        (
            "post/correction",
            Some("post"),
            "Correction request",
            "post",
            "CREATE",
            330,
        ),
    ]
    .into_iter()
    .map(|(key, parent, label, module, function, order)| {
        leaf(
            key,
            parent,
            label,
            "history",
            "plain",
            if key.starts_with("post/amend") {
                work_context()
            } else {
                &[]
            },
            module,
            function,
            order,
        )
    })
    .collect::<Vec<_>>();
    defs.sort_by_key(|def| def.sort_order);
    defs
}

fn report_defs() -> Vec<MenuDef> {
    [
        ("year-compare", "Year comparison"),
        ("tax-burden", "Tax burden"),
        ("reserve-trend", "Reserve trend"),
        ("loss-expiry", "Loss expiry"),
        ("industry-stats", "Industry stats"),
        ("custom", "Custom report"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label))| {
        leaf(
            &format!("report:{suffix}"),
            Some("reports"),
            label,
            "bar-chart-3",
            "plain",
            &[],
            "reports",
            "READ",
            401 + index as i32,
        )
    })
    .collect()
}

fn admin_customer_defs() -> Vec<MenuDef> {
    [
        ("list", "Customer list", "READ"),
        ("by-master", "Business year master", "UPDATE"),
        ("agent", "Tax agent", "UPDATE"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        admin_leaf(
            "cust",
            suffix,
            label,
            "customer",
            function,
            511 + index as i32,
        )
    })
    .collect()
}

fn admin_security_defs() -> Vec<MenuDef> {
    [
        ("users", "Users", "READ"),
        ("roles", "Roles", "READ"),
        ("matrix", "Permission matrix", "READ"),
        ("menus", "Menus", "UPDATE"),
        ("functions", "Functions", "UPDATE"),
        ("mask", "Masking policies", "MASK_OFF"),
        ("scope", "Data scopes", "UPDATE"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        admin_leaf("sec", suffix, label, "admin", function, 521 + index as i32)
    })
    .collect()
}

fn admin_customer_access_defs() -> Vec<MenuDef> {
    [
        ("assign", "Customer assignment", "UPDATE"),
        ("groups", "Access groups", "UPDATE"),
        ("rules", "Access rules", "UPDATE"),
        ("delegate", "Delegation", "DELEGATE"),
        ("override", "Access override", "UPDATE"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        admin_leaf(
            "cacc",
            suffix,
            label,
            "permissions",
            function,
            531 + index as i32,
        )
    })
    .collect()
}

fn admin_law_defs() -> Vec<MenuDef> {
    [
        ("master", "Law version master", "READ"),
        ("rates", "Tax rates", "UPDATE"),
        ("limits", "Limits", "UPDATE"),
        ("credits", "Credits", "UPDATE"),
        ("depr-lives", "Depreciation lives", "UPDATE"),
        ("sme", "SME rules", "UPDATE"),
        ("loss-rule", "Loss carryforward rules", "UPDATE"),
        ("snapshots", "Law snapshots", "READ"),
        ("impact", "Impact simulation", "CALCULATE"),
        ("history", "Law history", "READ"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        admin_leaf("law", suffix, label, "law", function, 541 + index as i32)
    })
    .collect()
}

fn admin_form_defs() -> Vec<MenuDef> {
    [
        ("master", "Form master", "READ"),
        ("versions", "Form versions", "READ"),
        ("fields", "Fields", "UPDATE"),
        ("validations", "Validation rules", "UPDATE"),
        ("linkage-rule", "Linkage rules", "UPDATE"),
        ("migration", "Form migration", "CREATE"),
        ("efile-map", "E-file mapping", "UPDATE"),
        ("by-set", "Business-year form set", "UPDATE"),
        ("impact", "Form impact", "CALCULATE"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label, function))| {
        admin_leaf("form", suffix, label, "forms", function, 551 + index as i32)
    })
    .collect()
}

fn admin_code_defs() -> Vec<MenuDef> {
    [("manage", "Code management")]
        .into_iter()
        .enumerate()
        .map(|(index, (suffix, label))| {
            admin_leaf("code", suffix, label, "admin", "UPDATE", 561 + index as i32)
        })
        .collect()
}

fn admin_audit_defs() -> Vec<MenuDef> {
    [
        ("events", "Audit events"),
        ("login", "Login history"),
        ("perm", "Permission audit"),
        ("settings", "Settings audit"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (suffix, label))| {
        admin_leaf("audit", suffix, label, "audit", "READ", 571 + index as i32)
    })
    .collect()
}

fn admin_leaf(
    section: &str,
    suffix: &str,
    label: &str,
    module: &str,
    function: &str,
    sort_order: i32,
) -> MenuDef {
    let parent = format!("admin/{section}");
    leaf(
        &format!("admin/{section}:{suffix}"),
        Some(parent.as_str()),
        label,
        "settings",
        "admin",
        &[],
        module,
        function,
        sort_order,
    )
}

fn children_for(defs: &[MenuDef], parent_key: Option<&str>) -> Vec<Value> {
    defs.iter()
        .filter(|def| def.parent_key.as_deref() == parent_key)
        .map(|def| {
            let children = children_for(defs, Some(&def.key));
            node(def, children)
        })
        .collect()
}

fn node(def: &MenuDef, children: Vec<Value>) -> Value {
    let required_permissions = match (
        def.required_perm_module.as_deref(),
        def.required_perm_function.as_deref(),
    ) {
        (Some(module), Some(function)) => vec![format!("{module}:{function}")],
        _ => Vec::new(),
    };
    let label_ko = korean_label(&def.key, &def.label);
    let label_en = def.label.as_str();
    json!({
        "code": def.key,
        "key": def.key,
        "name": label_ko,
        "label": label_ko,
        "display_name": label_ko,
        "label_ko": label_ko,
        "label_en": label_en,
        "labels": {
            "ko": label_ko,
            "en": label_en,
        },
        "path": def.path,
        "icon": def.icon,
        "layout": def.layout,
        "requires_context": def.requires_context,
        "required_perm_module": def.required_perm_module,
        "required_perm_function": def.required_perm_function,
        "required_permissions": required_permissions,
        "feature_flag": def.feature_flag,
        "implemented": true,
        "leaf": children.is_empty(),
        "children": children,
    })
}

fn legacy_node(code: &str, number: &str, label: &str, path: &str) -> Value {
    json!({
        "code": code,
        "number": number,
        "name": label,
        "label": label,
        "display_name": format!("{number}. {label}"),
        "path": path,
        "implemented": true,
        "children": [],
    })
}

fn group(
    key: &str,
    parent_key: Option<&str>,
    label: &str,
    path: &str,
    icon: &str,
    layout: &str,
    sort_order: i32,
) -> MenuDef {
    MenuDef {
        key: key.to_string(),
        parent_key: parent_key.map(ToString::to_string),
        label: label.to_string(),
        path: path.to_string(),
        icon: icon.to_string(),
        layout: layout.to_string(),
        requires_context: Vec::new(),
        required_perm_module: None,
        required_perm_function: None,
        feature_flag: None,
        sort_order,
    }
}

#[allow(clippy::too_many_arguments)]
fn leaf(
    key: &str,
    parent_key: Option<&str>,
    label: &str,
    icon: &str,
    layout: &str,
    requires_context: &[&str],
    required_perm_module: &str,
    required_perm_function: &str,
    sort_order: i32,
) -> MenuDef {
    MenuDef {
        key: key.to_string(),
        parent_key: parent_key.map(ToString::to_string),
        label: label.to_string(),
        path: route_path(key),
        icon: icon.to_string(),
        layout: layout.to_string(),
        requires_context: requires_context
            .iter()
            .map(|value| value.to_string())
            .collect(),
        required_perm_module: Some(required_perm_module.to_string()),
        required_perm_function: Some(required_perm_function.to_string()),
        feature_flag: None,
        sort_order,
    }
}

fn route_path(key: &str) -> String {
    if let Some(suffix) = key.strip_prefix("dashboard:") {
        return format!("#/dashboard/{suffix}");
    }
    if let Some(rest) = key.strip_prefix("ws/") {
        let normalized = rest.replace(':', "/");
        return format!("#/workspace/ws/{normalized}");
    }
    if let Some(rest) = key.strip_prefix("post/") {
        let normalized = rest.replace(':', "/");
        return format!("#/post/{normalized}");
    }
    if let Some(suffix) = key.strip_prefix("report:") {
        return format!("#/report/{suffix}");
    }
    if let Some(rest) = key.strip_prefix("admin/") {
        let normalized = rest.replace(':', "/");
        return format!("#/admin/{normalized}");
    }
    format!("#/{key}")
}

fn work_context() -> &'static [&'static str] {
    &["customer_id", "business_year_id"]
}

fn korean_label<'a>(key: &str, fallback: &'a str) -> &'a str {
    match key {
        "dashboard" => "대시보드",
        "workspace" => "신고 작업",
        "post" => "사후 관리",
        "reports" => "분석/보고서",
        "admin" => "관리",
        "ws/start" => "0. 작업 시작",
        "ws/info" => "1. 세무정보 입력",
        "ws/adj" => "2. 세무조정",
        "ws/form" => "3. 서식 작성",
        "ws/val" => "4. 검증",
        "ws/appr" => "5. 결재",
        "ws/print" => "6. 출력",
        "ws/file" => "7. 전자신고",
        "post/hist" => "1. 신고 이력",
        "post/amend" => "2. 수정신고/경정청구",
        "admin/cust" => "5-A. 고객사 관리",
        "admin/sec" => "5-B. 사용자/권한 관리",
        "admin/cacc" => "5-C. 담당 법인 권한",
        "admin/law" => "5-D. 법령/세율 버전 관리",
        "admin/form" => "5-E. 서식 버전 관리",
        "admin/code" => "5-F. 코드 관리",
        "admin/audit" => "5-G. 감사/로그",
        "dashboard:overview" => "업무 현황",
        "dashboard:duesoon" => "신고 마감 임박",
        "dashboard:inbox" => "알림/결재 대기함",
        "dashboard:recent" => "최근 활동",
        "dashboard:kpi-tax" => "KPI (당기 세부담 / 업종별 분포)",
        "ws/start:customer-pick" => "고객사 선택",
        "ws/start:by-pick" => "사업연도 선택/신규 사업연도 생성",
        "ws/start:snapshot" => "적용 법령/서식 스냅샷",
        "ws/info:fs" => "재무제표 입력/임포트",
        "ws/info:mapping" => "계정과목 매핑",
        "ws/info:assets" => "자산대장",
        "ws/info:transactions" => "거래명세",
        "ws/info:vehicle" => "업무용 차량 운행기록",
        "ws/info:consistency" => "입력 데이터 일관성 검증",
        "ws/adj:B1" => "B1 소득금액조정명세",
        "ws/adj:B2" => "B2 기부금 조정",
        "ws/adj:B3" => "B3 접대비 조정",
        "ws/adj:B4" => "B4 감가상각비 조정",
        "ws/adj:B5" => "B5 인정이자 조정",
        "ws/adj:B6" => "B6 퇴직급여충당금 조정",
        "ws/adj:B7" => "B7 대손충당금 조정",
        "ws/adj:B8" => "B8 외화평가 조정",
        "ws/adj:B9" => "B9 재고/유가증권 평가",
        "ws/adj:B10" => "B10 업무승용차 조정",
        "ws/adj:B11" => "B11 이월결손금",
        "ws/adj:B12" => "B12 세액공제/감면",
        "ws/adj:B13" => "B13 최저한세",
        "ws/adj:B14" => "B14 가산세",
        "ws/adj:B15" => "B15 자본금과 적립금",
        "ws/adj:B16" => "B16 외국법인",
        "ws/adj:B17" => "B17 연결납세",
        "ws/form:form3" => "별지 3호",
        "ws/form:attachments" => "부속서류 일람",
        "ws/form:preview" => "서식 미리보기",
        "ws/form:linkage" => "서식 간 연동 검증",
        "ws/val:run" => "일괄 검증 실행",
        "ws/val:issues" => "오류/경고 목록",
        "ws/val:rules" => "검증 규칙 적용 결과",
        "ws/appr:request" => "결재 요청",
        "ws/appr:inbox" => "결재함",
        "ws/appr:rejected" => "반려 이력/재작업",
        "ws/print:preview" => "PDF 미리보기",
        "ws/print:bulk" => "단일/일괄 출력",
        "ws/print:history" => "출력 이력",
        "ws/file:precheck" => "전자신고 사전 검증",
        "ws/file:generate" => "전자신고 파일 생성",
        "ws/file:submit" => "제출/접수 확인",
        "ws/file:done" => "신고 완료",
        "post/hist:list" => "신고 이력 목록",
        "post/amend:unlock" => "수정신고 잠금 해제 요청",
        "post/amend:version" => "수정신고 버전 선택",
        "post/amend:diff" => "수정신고 차이 보고서",
        "post/amend:resubmit" => "수정신고 재제출",
        "post/correction" => "경정청구",
        "report:year-compare" => "사업연도별 비교",
        "report:tax-burden" => "세부담 분석",
        "report:reserve-trend" => "유보 금액 추이",
        "report:loss-expiry" => "이월결손금 만료 예측",
        "report:industry-stats" => "업종별 통계",
        "report:custom" => "사용자 정의 리포트",
        "admin/cust:list" => "고객사 등록/조회",
        "admin/cust:by-master" => "사업연도 마스터",
        "admin/cust:agent" => "세무대리 계약",
        "admin/sec:users" => "사용자 등록/조회",
        "admin/sec:roles" => "역할 마스터",
        "admin/sec:matrix" => "권한 매트릭스",
        "admin/sec:menus" => "메뉴 관리",
        "admin/sec:functions" => "기능 코드 관리",
        "admin/sec:mask" => "필드 마스킹 정책",
        "admin/sec:scope" => "데이터 권한",
        "admin/cacc:assign" => "사용자별 담당 법인 배정",
        "admin/cacc:groups" => "고객사 그룹 관리",
        "admin/cacc:rules" => "자동 할당 규칙",
        "admin/cacc:delegate" => "권한 위임/대체",
        "admin/cacc:override" => "개별 권한 예외 설정",
        "admin/law:master" => "법령 버전 마스터",
        "admin/law:rates" => "법인세율표",
        "admin/law:limits" => "한도/공통표",
        "admin/law:credits" => "세액공제/감면 표",
        "admin/law:depr-lives" => "기준내용연수표",
        "admin/law:sme" => "중소기업 판정기준",
        "admin/law:loss-rule" => "결손금 공제규정",
        "admin/law:snapshots" => "사업연도별 적용 스냅샷",
        "admin/law:impact" => "영향 시뮬레이션",
        "admin/law:history" => "개정 공지/이력",
        "admin/form:master" => "서식 마스터",
        "admin/form:versions" => "서식 버전",
        "admin/form:fields" => "서식 항목 편집",
        "admin/form:validations" => "검증 규칙",
        "admin/form:linkage-rule" => "서식 간 연동 규칙",
        "admin/form:migration" => "데이터 마이그레이션",
        "admin/form:efile-map" => "전자신고 레코드 매핑",
        "admin/form:by-set" => "사업연도별 적용 서식 세트",
        "admin/form:impact" => "서식 영향 시뮬레이션",
        "admin/code:manage" => "코드 관리",
        "admin/audit:events" => "감사 로그",
        "admin/audit:login" => "로그인 이력",
        "admin/audit:perm" => "권한 변경 이력",
        "admin/audit:settings" => "시스템 설정 이력",
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prototype_tree_shape() {
        let tree = prototype_menu_tree();
        let top = tree["children"].as_array().expect("top-level menu");
        assert_eq!(
            top.iter()
                .map(|node| node["code"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["dashboard", "workspace", "post", "reports", "admin"]
        );
        assert_eq!(leaf_count(&tree), 99);
        let workspace = top.iter().find(|node| node["code"] == "workspace").unwrap();
        assert_eq!(workspace["children"].as_array().unwrap().len(), 8);
        let info = workspace["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["code"] == "ws/info")
            .unwrap();
        assert_eq!(info["children"][0]["code"], "ws/info:fs");
        assert_eq!(
            info["children"][0]["requires_context"],
            json!(["customer_id", "business_year_id"])
        );
        let admin = top.iter().find(|node| node["code"] == "admin").unwrap();
        assert_eq!(admin["children"].as_array().unwrap().len(), 7);
        assert!(prototype_menu_seeds()
            .iter()
            .any(|node| node.key == "admin/sec:menus" && node.layout == "admin"));
    }

    fn leaf_count(node: &Value) -> usize {
        let children = node["children"].as_array().cloned().unwrap_or_default();
        if children.is_empty() {
            return 1;
        }
        children.iter().map(leaf_count).sum()
    }
}
