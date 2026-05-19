ALTER TABLE menu_nodes
    ADD COLUMN IF NOT EXISTS layout VARCHAR(40) NOT NULL DEFAULT 'plain';

CREATE INDEX IF NOT EXISTS idx_menu_nodes_layout
    ON menu_nodes(layout, sort_order);
