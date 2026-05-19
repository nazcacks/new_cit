# Menu Smoke Findings

## Before

The prototype IA and routes existed, but a clean demo database could still render empty menu surfaces: no tenant-scoped customers, no business years, no tax data, no adjustment records, no generated forms, no e-filing history, and no workflow queue items.

The old demo login also used `admin123!`, while the menu DoD requires `ChangeMe123!`.

## After

`seed-demo --reset` now provisions a deterministic `demo` tenant dataset:

- 4 users, 3 customers, 5 business years, and the v1.2 menu seed. The active tree follows the 99-leaf completion gate in `추가구현_v1.2.md`.
- Main Alpha 2026 filing context with balanced financial statements, assets, transactions, vehicle logs, import batches, B1~B17 adjustment outputs, reserves, generated forms, validation issues, workflow queue, printable bundle, and e-filing history.
- Filed Alpha 2024 context for amendment preview.
- Notifications, audit logs, tax burden, year comparison, and reserve trend report data.
- Demo login is `demo / admin / ChangeMe123!`.

The 2026-05-19 pack completion adds security and workflow coverage on top of the seed data: FILED lock enforcement, TOTP/IP allowlist login, five-failure lockout/admin unlock, B1/B4/B15 item grids, adjustment item history and evidence attachments, role-menu-function RBAC, data-scope filtering, field masking, access delegation, audit-chain verification, D-30/D-7 scheduler alerts, loss-expiry and industry reports, user-defined reports, PDF watermark/seal, and print history.

The v1.2 menu completion adds the 5-group recursive sidebar, `#/<group>/<key>` deep links, workspace/plain/admin layout selection, leaf watermarks with `data-leaf-key`, one-click work-context guidance, and `tests/menu_full_tree.rs` coverage for the 99 active leaves.

P3 external integrations are not required for menu smoke completion: direct e-Tax submission, ERP connectors, and Redis pub/sub cache invalidation remain roadmap items pending external credentials/infrastructure.

## Verification Targets

- `cargo run --bin seed-demo -- --reset`
- `cargo test --test menu_smoke`
- `cargo test --test menu_full_tree`
- `cargo test --test integration_flow`
- `cargo test --test lock_and_2fa`
- `cargo test --test permissions_evaluator`
- `cargo test --test adjustment_modules_all`
- `cargo test --test workflow_multi_step`
- `cargo test --test scheduler`
- `cargo clippy --all-targets -- -D warnings`
