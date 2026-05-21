use cit_system::modules;
use serde_json::Value;

#[test]
fn v13_leaf_permissions_context_and_empty_state_contract_are_consistent() {
    let tree = modules::module_tree();
    let leaves = collect_leaves(&tree);
    assert_eq!(leaves.len(), 100);

    for leaf in &leaves {
        let code = leaf["code"].as_str().unwrap();
        let permissions = leaf["required_permissions"].as_array().unwrap();
        assert_eq!(
            permissions.len(),
            1,
            "{code} must expose one required permission"
        );
        assert!(
            leaf["required_perm_module"].as_str().is_some(),
            "{code} permission module missing"
        );
        assert!(
            leaf["required_perm_function"].as_str().is_some(),
            "{code} permission function missing"
        );
    }

    let context_required = leaves
        .iter()
        .filter(|leaf| {
            leaf["requires_context"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
        .map(|leaf| leaf["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(
        context_required.iter().all(|code| {
            code.starts_with("ws/info:")
                || code.starts_with("ws/adj:")
                || code.starts_with("ws/form:")
                || code.starts_with("ws/val:")
                || code.starts_with("ws/appr:")
                || code.starts_with("ws/print:")
                || code.starts_with("ws/file:")
                || code.starts_with("post/amend:")
        }),
        "unexpected context-gated leaves: {context_required:?}"
    );

    let screens = std::fs::read_to_string("frontend/app/screens.js").expect("screens.js");
    for required in [
        "function leafGate",
        "empty-state",
        "work-context",
        "canAccessLeaf",
        "isFeatureEnabled",
        "data-leaf-key",
    ] {
        assert!(
            screens.contains(required),
            "frontend gate contract missing {required}"
        );
    }
}

fn collect_leaves(node: &Value) -> Vec<&Value> {
    match node["children"].as_array() {
        Some(children) if !children.is_empty() => {
            children.iter().flat_map(collect_leaves).collect()
        }
        _ => vec![node],
    }
}
