use cit_system::modules::{legacy_module_tree, prototype_menu_seeds, prototype_menu_tree};
use serde_json::json;

#[test]
fn prototype_menu_tree_matches_phase2_information_architecture() {
    let tree = prototype_menu_tree();
    assert_eq!(tree["code"], "cit-system");
    let top = tree["children"].as_array().expect("top menu");
    assert_eq!(
        top.iter()
            .map(|node| node["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["dashboard", "workspace", "post", "reports", "admin"]
    );

    let workspace = top.iter().find(|node| node["code"] == "workspace").unwrap();
    assert_eq!(workspace["children"].as_array().unwrap().len(), 8);
    assert_eq!(leaf_count(&tree), 99);
    let info = workspace["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["code"] == "ws/info")
        .expect("info group");
    assert_eq!(info["children"][0]["code"], "ws/info:fs");
    assert_eq!(
        info["children"][0]["requires_context"],
        json!(["customer_id", "business_year_id"])
    );
    let adjustments = workspace["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["code"] == "ws/adj")
        .expect("adjustment group");
    assert_eq!(adjustments["children"].as_array().unwrap().len(), 17);
    assert_eq!(
        adjustments["children"][11]["path"],
        "#/workspace/ws/adj/B12"
    );

    let admin = top.iter().find(|node| node["code"] == "admin").unwrap();
    assert_eq!(admin["children"].as_array().unwrap().len(), 7);
    assert!(prototype_menu_seeds()
        .iter()
        .any(|node| node.key == "admin/sec:menus"
            && node.required_perm_module.as_deref() == Some("admin")
            && node.layout == "admin"));
}

#[test]
fn legacy_module_tree_is_kept_as_audit_route_payload() {
    let legacy = legacy_module_tree();
    assert_eq!(legacy["code"], "cit-system-legacy");
    assert_eq!(legacy["children"].as_array().unwrap().len(), 10);
}

fn leaf_count(node: &serde_json::Value) -> usize {
    let children = node["children"].as_array().cloned().unwrap_or_default();
    if children.is_empty() {
        return 1;
    }
    children.iter().map(leaf_count).sum()
}
