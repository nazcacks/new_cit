use std::collections::BTreeSet;

use cit_system::modules::prototype_menu_tree;
use serde_json::Value;

#[test]
fn v12_full_menu_tree_exposes_all_active_leaves() {
    let tree = prototype_menu_tree();
    let roots = tree["children"].as_array().expect("root children");
    assert_eq!(
        roots
            .iter()
            .map(|node| node["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["dashboard", "workspace", "post", "reports", "admin"]
    );

    let leaves = collect_leaves(&tree);
    let leaf_keys = leaves
        .iter()
        .map(|node| node["code"].as_str().unwrap().to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(leaves.len(), 99, "v1.2 exposes 99 active leaf screens");
    assert_eq!(leaf_keys, expected_leaf_keys());

    for leaf in leaves {
        assert_eq!(leaf["leaf"], true, "leaf marker for {}", leaf["code"]);
        assert!(
            leaf["path"].as_str().unwrap().starts_with("#/"),
            "hash route for {}",
            leaf["code"]
        );
        assert!(
            leaf["layout"]
                .as_str()
                .is_some_and(|layout| !layout.is_empty()),
            "layout for {}",
            leaf["code"]
        );
        assert_eq!(
            leaf["required_permissions"].as_array().unwrap().len(),
            1,
            "one permission function per leaf for {}",
            leaf["code"]
        );
        if leaf["code"].as_str().unwrap().starts_with("ws/")
            && !leaf["code"].as_str().unwrap().starts_with("ws/start:")
        {
            assert_eq!(
                leaf["requires_context"],
                serde_json::json!(["customer_id", "business_year_id"]),
                "workspace context guard for {}",
                leaf["code"]
            );
        }
    }
}

#[test]
fn frontend_registry_and_router_cover_representative_deep_links() {
    let screens = include_str!("../frontend/app/screens.js");
    let router = include_str!("../frontend/app/router.js");
    let menu = include_str!("../frontend/app/menu.js");
    let app = include_str!("../frontend/app.js");
    let i18n = include_str!("../frontend/app/i18n.js");
    let index = include_str!("../frontend/index.html");
    for key in [
        "dashboard:overview",
        "ws/info:fs",
        "ws/file:generate",
        "post/amend:diff",
        "report:loss-expiry",
        "admin/sec:menus",
        "admin/law:master",
        "admin/form:linkage-rule",
        "admin/audit:perm",
    ] {
        assert!(
            screens.contains(key),
            "frontend leaf registry contains {key}"
        );
    }
    assert!(
        screens.contains("ws/adj:${code}") && screens.contains("\"B12\""),
        "frontend leaf registry generates adjustment deep links including ws/adj:B12"
    );
    for snippet in [
        "export const leafRoutes",
        "leafFocusText",
        "keyToHash",
        "#/workspace/",
        "normalizeParts",
        "renderNode",
        "nodeActive",
        "languageSelect",
        "labelForNode",
        "data-leaf-key",
        "screenByLeaf",
    ] {
        assert!(
            screens.contains(snippet)
                || router.contains(snippet)
                || menu.contains(snippet)
                || app.contains(snippet)
                || i18n.contains(snippet)
                || index.contains(snippet),
            "frontend route/menu implementation contains {snippet}"
        );
    }

    for key in expected_leaf_keys() {
        if let Some(code) = key.strip_prefix("ws/adj:") {
            assert!(
                screens.contains("ws/adj:${code}") && screens.contains(&format!("\"{code}\"")),
                "frontend route registry generates {key}"
            );
        } else {
            assert!(
                screens.contains(&format!("\"{key}\"")),
                "frontend route registry contains {key}"
            );
        }
    }
}

#[test]
fn menu_labels_default_to_korean_with_english_alternate_labels() {
    let tree = prototype_menu_tree();
    let dashboard = find_node(&tree, "dashboard").expect("dashboard node");
    assert_eq!(dashboard["label"], "대시보드");
    assert_eq!(dashboard["labels"]["en"], "Dashboard");

    let b12 = find_node(&tree, "ws/adj:B12").expect("B12 node");
    assert_eq!(b12["label"], "B12 세액공제/감면");
    assert_eq!(b12["labels"]["en"], "B12 Tax credits");

    let menu = include_str!("../frontend/app/menu.js");
    let i18n = include_str!("../frontend/app/i18n.js");
    assert!(
        menu.contains("labelForNode(node, locale)") && i18n.contains("normalizeLocale(locale)"),
        "frontend menu renders labels by selected locale"
    );
}

fn collect_leaves(node: &Value) -> Vec<&Value> {
    match node["children"].as_array() {
        Some(children) if !children.is_empty() => {
            children.iter().flat_map(collect_leaves).collect()
        }
        _ => vec![node],
    }
}

fn find_node<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    if node["code"] == key || node["key"] == key {
        return Some(node);
    }
    node["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|child| find_node(child, key))
}

fn expected_leaf_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    keys.extend(
        [
            "dashboard:overview",
            "dashboard:duesoon",
            "dashboard:inbox",
            "dashboard:recent",
            "dashboard:kpi-tax",
            "ws/start:customer-pick",
            "ws/start:by-pick",
            "ws/start:snapshot",
            "ws/info:fs",
            "ws/info:mapping",
            "ws/info:assets",
            "ws/info:transactions",
            "ws/info:vehicle",
            "ws/info:consistency",
            "ws/form:form3",
            "ws/form:attachments",
            "ws/form:preview",
            "ws/form:linkage",
            "ws/val:run",
            "ws/val:issues",
            "ws/val:rules",
            "ws/appr:request",
            "ws/appr:inbox",
            "ws/appr:rejected",
            "ws/print:preview",
            "ws/print:bulk",
            "ws/print:history",
            "ws/file:precheck",
            "ws/file:generate",
            "ws/file:submit",
            "ws/file:done",
            "post/hist:list",
            "post/amend:unlock",
            "post/amend:version",
            "post/amend:diff",
            "post/amend:resubmit",
            "post/correction",
            "report:year-compare",
            "report:tax-burden",
            "report:reserve-trend",
            "report:loss-expiry",
            "report:industry-stats",
            "report:custom",
            "admin/cust:list",
            "admin/cust:by-master",
            "admin/cust:agent",
            "admin/sec:users",
            "admin/sec:roles",
            "admin/sec:matrix",
            "admin/sec:menus",
            "admin/sec:functions",
            "admin/sec:mask",
            "admin/sec:scope",
            "admin/cacc:assign",
            "admin/cacc:groups",
            "admin/cacc:rules",
            "admin/cacc:delegate",
            "admin/cacc:override",
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
            "admin/form:master",
            "admin/form:versions",
            "admin/form:fields",
            "admin/form:validations",
            "admin/form:linkage-rule",
            "admin/form:migration",
            "admin/form:efile-map",
            "admin/form:by-set",
            "admin/form:impact",
            "admin/code:manage",
            "admin/audit:events",
            "admin/audit:login",
            "admin/audit:perm",
            "admin/audit:settings",
        ]
        .into_iter()
        .map(String::from),
    );
    for index in 1..=17 {
        keys.insert(format!("ws/adj:B{index}"));
    }
    keys
}
