ALTER TABLE efile_record_fields
    ADD COLUMN IF NOT EXISTS data_type VARCHAR(10) NOT NULL DEFAULT 'AN',
    ADD COLUMN IF NOT EXISTS description TEXT;

CREATE TABLE IF NOT EXISTS efile_validation_rules (
    rule_id         BIGSERIAL PRIMARY KEY,
    efile_master_id BIGINT NOT NULL REFERENCES efile_masters(efile_master_id),
    rule_code       VARCHAR(80) NOT NULL,
    severity        VARCHAR(20) NOT NULL DEFAULT 'ERROR',
    field_path      VARCHAR(200),
    message         TEXT NOT NULL,
    rule_json       JSONB NOT NULL DEFAULT '{}'::jsonb,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (efile_master_id, rule_code)
);

UPDATE efile_record_layouts l
SET fixed_length = spec.fixed_length
FROM efile_masters m
JOIN (
    VALUES
        ('H', 80),
        ('D', 80),
        ('T', 80)
) AS spec(record_type, fixed_length) ON TRUE
WHERE l.efile_master_id = m.efile_master_id
  AND m.master_code = 'CIT-EFILE-2026'
  AND l.record_type = spec.record_type;

DELETE FROM efile_record_fields f
USING efile_record_layouts l, efile_masters m
WHERE f.layout_id = l.layout_id
  AND l.efile_master_id = m.efile_master_id
  AND m.master_code = 'CIT-EFILE-2026';

INSERT INTO efile_record_fields (
    layout_id, field_name, start_pos, byte_length, align, pad_char,
    required, source_path, data_type, description
)
SELECT l.layout_id, spec.field_name, spec.start_pos, spec.byte_length,
       spec.align, spec.pad_char, spec.required, spec.source_path,
       spec.data_type, spec.description
FROM efile_masters m
JOIN efile_record_layouts l ON l.efile_master_id = m.efile_master_id
JOIN (
    VALUES
        ('H', 'record_type', 1, 1, 'LEFT', ' ', TRUE, 'literal:H', 'AN', '헤더 레코드 식별자'),
        ('H', 'biz_reg_no', 2, 10, 'LEFT', ' ', TRUE, 'customer.biz_reg_no', 'AN', '사업자등록번호'),
        ('H', 'customer_name', 12, 30, 'LEFT', ' ', TRUE, 'customer.customer_name', 'AN', '법인명'),
        ('H', 'year_label', 42, 4, 'RIGHT', '0', TRUE, 'business_year.year_label', 'N', '귀속연도'),
        ('H', 'snapshot_id', 46, 12, 'RIGHT', '0', TRUE, 'law_snapshot.snapshot_id', 'N', '적용 스냅샷'),
        ('H', 'total_tax_due', 58, 20, 'RIGHT', '0', TRUE, 'FORM3.total_tax_due', 'N', '총 납부세액'),
        ('D', 'record_type', 1, 1, 'LEFT', ' ', TRUE, 'literal:D', 'AN', '상세 레코드 식별자'),
        ('D', 'form_code', 2, 10, 'LEFT', ' ', TRUE, 'literal:FORM3', 'AN', '서식 코드'),
        ('D', 'taxable_income', 12, 15, 'RIGHT', '0', TRUE, 'FORM3.taxable_income', 'N', '과세표준'),
        ('D', 'corporate_tax', 27, 15, 'RIGHT', '0', TRUE, 'FORM3.corporate_tax', 'N', '산출세액'),
        ('D', 'local_income_tax', 42, 15, 'RIGHT', '0', TRUE, 'FORM3.local_income_tax', 'N', '지방소득세'),
        ('D', 'tax_credits', 57, 15, 'RIGHT', '0', TRUE, 'FORM3.tax_credits', 'N', '세액공제'),
        ('T', 'record_type', 1, 1, 'LEFT', ' ', TRUE, 'literal:T', 'AN', '합계 레코드 식별자'),
        ('T', 'record_count', 2, 6, 'RIGHT', '0', TRUE, 'system.record_count', 'N', '총 레코드 수'),
        ('T', 'checksum', 8, 20, 'RIGHT', '0', TRUE, 'system.checksum', 'AN', '바이트 체크섬'),
        ('T', 'total_tax_due', 28, 20, 'RIGHT', '0', TRUE, 'FORM3.total_tax_due', 'N', '총 납부세액 합계')
) AS spec(record_type, field_name, start_pos, byte_length, align, pad_char, required, source_path, data_type, description)
    ON spec.record_type = l.record_type
WHERE m.master_code = 'CIT-EFILE-2026'
ORDER BY l.sort_order, spec.start_pos;

INSERT INTO efile_validation_rules (
    efile_master_id, rule_code, severity, field_path, message, rule_json
)
SELECT efile_master_id, rule_code, severity, field_path, message, rule_json
FROM efile_masters
CROSS JOIN (
    VALUES
        ('BIZ_REG_NO_FORMAT', 'ERROR', 'customer.biz_reg_no', '사업자등록번호는 숫자 10자리여야 합니다.', '{"type":"digits_length","length":10}'::jsonb),
        ('BIZ_REG_NO_CHECKSUM', 'WARN', 'customer.biz_reg_no', '사업자등록번호 체크섬 확인이 필요합니다.', '{"type":"biz_reg_no_checksum"}'::jsonb),
        ('FORM3_TOTAL_TAX_DUE_REQUIRED', 'ERROR', 'FORM3.total_tax_due', 'FORM3 총 납부세액은 0보다 커야 합니다.', '{"type":"min","min":1}'::jsonb),
        ('RECORD_LENGTH_MATCH', 'ERROR', 'system.records', '전자신고 레코드 길이가 포맷 메타와 일치해야 합니다.', '{"type":"record_length"}'::jsonb)
) AS rule(rule_code, severity, field_path, message, rule_json)
WHERE master_code = 'CIT-EFILE-2026'
ON CONFLICT (efile_master_id, rule_code) DO NOTHING;
