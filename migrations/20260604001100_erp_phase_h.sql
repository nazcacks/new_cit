DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM information_schema.schemata
        WHERE schema_name LIKE 'tenant\_%' ESCAPE '\'
    LOOP
        IF to_regclass(format('%I.business_years', tenant_schema)) IS NOT NULL
           AND to_regclass(format('%I.import_batches', tenant_schema)) IS NOT NULL THEN
            EXECUTE format(
                'CREATE TABLE IF NOT EXISTS %I.erp_import_runs (
                    run_id          BIGSERIAL PRIMARY KEY,
                    by_id           BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                    vendor          VARCHAR(40) NOT NULL,
                    source_system   VARCHAR(120) NOT NULL,
                    adapter_kind    VARCHAR(30) NOT NULL DEFAULT ''MOCK'',
                    mock_profile    VARCHAR(40),
                    status          VARCHAR(30) NOT NULL DEFAULT ''QUEUED'',
                    attempt_count   INT NOT NULL DEFAULT 0,
                    last_error      TEXT,
                    job_id          UUID,
                    import_batch_id BIGINT REFERENCES %I.import_batches(batch_id),
                    row_count       INT NOT NULL DEFAULT 0,
                    valid_count     INT NOT NULL DEFAULT 0,
                    error_count     INT NOT NULL DEFAULT 0,
                    metadata        JSONB NOT NULL DEFAULT ''{}''::jsonb,
                    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    completed_at    TIMESTAMPTZ
                )',
                tenant_schema,
                tenant_schema,
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_erp_import_runs_by
                 ON %I.erp_import_runs(by_id, created_at DESC)',
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_erp_import_runs_status
                 ON %I.erp_import_runs(status, updated_at DESC)',
                tenant_schema
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_erp_import_runs_job
                 ON %I.erp_import_runs(job_id)',
                tenant_schema
            );
        END IF;
    END LOOP;
END $$;
