use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::{
    domain::{MenuNodeRecord, UpdateMenuNodeRequest},
    modules,
};

pub async fn list_menu_nodes(pool: &PgPool) -> Result<Vec<MenuNodeRecord>> {
    ensure_menu_nodes(pool).await?;
    sqlx::query_as::<_, MenuNodeRecord>(
        r#"
        SELECT menu_key, parent_key, label, path, layout, requires_context,
               feature_flag, required_perm_module, required_perm_function,
               sort_order, enabled, updated_at
        FROM menu_nodes
        ORDER BY sort_order, menu_key
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to list menu nodes")
}

pub async fn update_menu_node(
    pool: &PgPool,
    menu_key: &str,
    request: UpdateMenuNodeRequest,
) -> Result<MenuNodeRecord> {
    ensure_menu_nodes(pool).await?;
    sqlx::query_as::<_, MenuNodeRecord>(
        r#"
        UPDATE menu_nodes
        SET feature_flag = COALESCE($2, feature_flag),
            required_perm_module = COALESCE($3, required_perm_module),
            required_perm_function = COALESCE($4, required_perm_function),
            enabled = COALESCE($5, enabled),
            updated_at = NOW()
        WHERE menu_key = $1
        RETURNING menu_key, parent_key, label, path, layout, requires_context,
                  feature_flag, required_perm_module, required_perm_function,
                  sort_order, enabled, updated_at
        "#,
    )
    .bind(menu_key)
    .bind(request.feature_flag)
    .bind(request.required_perm_module)
    .bind(request.required_perm_function)
    .bind(request.enabled)
    .fetch_one(pool)
    .await
    .context("menu node not found")
}

async fn ensure_menu_nodes(pool: &PgPool) -> Result<()> {
    for seed in modules::prototype_menu_seeds() {
        sqlx::query(
            r#"
            INSERT INTO menu_nodes (
                menu_key, parent_key, label, path, layout, requires_context,
                feature_flag, required_perm_module, required_perm_function, sort_order
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (menu_key) DO UPDATE
            SET label = EXCLUDED.label,
                path = EXCLUDED.path,
                layout = EXCLUDED.layout,
                requires_context = EXCLUDED.requires_context,
                required_perm_module = COALESCE(menu_nodes.required_perm_module, EXCLUDED.required_perm_module),
                required_perm_function = COALESCE(menu_nodes.required_perm_function, EXCLUDED.required_perm_function),
                sort_order = EXCLUDED.sort_order,
                updated_at = NOW()
            "#,
        )
        .bind(seed.key)
        .bind(seed.parent_key)
        .bind(seed.label)
        .bind(seed.path)
        .bind(seed.layout)
        .bind(seed.requires_context)
        .bind(seed.feature_flag)
        .bind(seed.required_perm_module)
        .bind(seed.required_perm_function)
        .bind(seed.sort_order)
        .execute(pool)
        .await
        .context("failed to seed menu nodes")?;
    }
    sqlx::query(
        r#"
        INSERT INTO menu_functions (menu_key, function_code, enabled)
        SELECT DISTINCT menu_key, function_code, TRUE
        FROM (
            SELECT menu_key, COALESCE(required_perm_function, 'READ') AS function_code
            FROM menu_nodes
            UNION ALL
            SELECT menu_key, 'READ'
            FROM menu_nodes
        ) seed
        WHERE EXISTS (
            SELECT 1 FROM function_codes fc WHERE fc.function_code = seed.function_code
        )
        ON CONFLICT (menu_key, function_code) DO UPDATE
        SET enabled = TRUE,
            updated_at = NOW()
        "#,
    )
    .execute(pool)
    .await
    .context("failed to seed menu functions")?;
    Ok(())
}
