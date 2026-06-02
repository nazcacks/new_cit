UPDATE tax_forms
SET description = '서식 버전에서 생성됨',
    updated_at = NOW()
WHERE description = 'created from form version';
