DO $$
DECLARE
    schema_name TEXT;
BEGIN
    FOR schema_name IN
        SELECT t.schema_name
        FROM tenants t
        WHERE to_regclass(format('%I.form_data', t.schema_name)) IS NOT NULL
    LOOP
        EXECUTE format(
            'DELETE FROM %I.form_data_history
             WHERE form_data_id IN (
                 SELECT form_data_id
                 FROM %I.form_data
                 WHERE form_version_id IN (
                     SELECT form_version_id
                     FROM public.form_versions
                     WHERE version_no = ''MENU-SMOKE-2026.1''
                 )
             )',
            schema_name,
            schema_name
        );
        EXECUTE format(
            'DELETE FROM %I.form_data
             WHERE form_version_id IN (
                 SELECT form_version_id
                 FROM public.form_versions
                 WHERE version_no = ''MENU-SMOKE-2026.1''
             )',
            schema_name
        );
    END LOOP;
END $$;

DELETE FROM form_validations
WHERE form_version_id IN (
    SELECT form_version_id
    FROM form_versions
    WHERE version_no = 'MENU-SMOKE-2026.1'
);

DELETE FROM form_templates
WHERE form_version_id IN (
    SELECT form_version_id
    FROM form_versions
    WHERE version_no = 'MENU-SMOKE-2026.1'
);

DELETE FROM form_versions
WHERE version_no = 'MENU-SMOKE-2026.1';
