DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.business_years ALTER COLUMN status SET DEFAULT ''DRAFT''',
            tenant_schema
        );
        EXECUTE format(
            'UPDATE %I.business_years SET status = ''DRAFT'' WHERE status = ''OPEN''',
            tenant_schema
        );
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.tax_agents (
                tax_agent_id BIGSERIAL PRIMARY KEY,
                customer_id BIGINT NOT NULL REFERENCES %I.customers(customer_id),
                agent_name VARCHAR(100) NOT NULL,
                agent_type VARCHAR(30) NOT NULL DEFAULT ''TAX_ACCOUNTANT'',
                email VARCHAR(200),
                phone VARCHAR(30),
                active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )',
            tenant_schema,
            tenant_schema
        );
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.customer_users (
                customer_user_id BIGSERIAL PRIMARY KEY,
                customer_id BIGINT NOT NULL REFERENCES %I.customers(customer_id),
                user_id BIGINT NOT NULL REFERENCES public.users(user_id),
                relationship_type VARCHAR(30) NOT NULL DEFAULT ''STAFF'',
                active BOOLEAN NOT NULL DEFAULT TRUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(customer_id, user_id)
            )',
            tenant_schema,
            tenant_schema
        );
    END LOOP;
END $$;
