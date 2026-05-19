CREATE TABLE IF NOT EXISTS menu_nodes (
    menu_key               VARCHAR(80) PRIMARY KEY,
    parent_key             VARCHAR(80) REFERENCES menu_nodes(menu_key),
    label                  VARCHAR(120) NOT NULL,
    path                   VARCHAR(240) NOT NULL,
    requires_context       TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    feature_flag           VARCHAR(80),
    required_perm_module   VARCHAR(80),
    required_perm_function VARCHAR(80),
    sort_order             INT NOT NULL DEFAULT 0,
    enabled                BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_menu_nodes_parent
    ON menu_nodes(parent_key, sort_order);

CREATE TABLE IF NOT EXISTS validation_rules (
    rule_code        VARCHAR(80) PRIMARY KEY,
    severity         VARCHAR(20) NOT NULL CHECK (severity IN ('ERROR', 'WARN', 'INFO')),
    area             VARCHAR(40) NOT NULL,
    message_template TEXT NOT NULL,
    applies_to       VARCHAR(160) NOT NULL,
    active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO validation_rules (
    rule_code, severity, area, message_template, applies_to
) VALUES
    ('TD_FS_REQUIRED', 'ERROR', 'tax-data', '재무제표 라인이 없습니다. 1-1 세무정보 입력을 완료하세요.', '#/workspace/ws-info'),
    ('TD_FS_BALANCED', 'ERROR', 'tax-data', '차변/대변 합계가 일치하지 않습니다.', '#/workspace/ws-info'),
    ('TD_MAPPING_RESOLVED', 'WARN', 'tax-data', '미매핑 계정 {unresolved_mapping_count}건이 남아 있습니다.', '#/workspace/ws-info'),
    ('TD_ASSET_REGISTER', 'INFO', 'tax-data', '등록된 자산이 없습니다.', '#/workspace/ws-info'),
    ('TD_VEHICLE_USAGE', 'WARN', 'tax-data', '업무용 차량 운행기록이 필요합니다.', '#/workspace/ws-info'),
    ('TD_TRANSACTIONS', 'INFO', 'tax-data', '거래명세 데이터가 없습니다.', '#/workspace/ws-info'),
    ('ADJ_B01', 'WARN', 'adjustment', 'B-1 소득금액 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B02', 'WARN', 'adjustment', 'B-2 기부금 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B03', 'WARN', 'adjustment', 'B-3 접대비 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B04', 'WARN', 'adjustment', 'B-4 감가상각 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B05', 'WARN', 'adjustment', 'B-5 가지급금 인정이자 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B06', 'WARN', 'adjustment', 'B-6 퇴직급여충당금 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B07', 'WARN', 'adjustment', 'B-7 대손충당금 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B08', 'WARN', 'adjustment', 'B-8 외화평가 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B09', 'WARN', 'adjustment', 'B-9 재고/유가증권 평가 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B10', 'WARN', 'adjustment', 'B-10 업무용승용차 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B11', 'WARN', 'adjustment', 'B-11 이월결손금 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B12', 'WARN', 'adjustment', 'B-12 세액공제 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B13', 'WARN', 'adjustment', 'B-13 최저한세 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B14', 'WARN', 'adjustment', 'B-14 가산세 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B15', 'WARN', 'adjustment', 'B-15 자본금과 적립금 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B16', 'WARN', 'adjustment', 'B-16 외국법인 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('ADJ_B17', 'WARN', 'adjustment', 'B-17 연결납세 조정 결과가 없습니다.', '#/workspace/ws-adj'),
    ('FORM_FORM3', 'ERROR', 'forms', '별지 3호 서식이 생성되지 않았습니다.', '#/workspace/ws-form'),
    ('FORM_FORM15', 'WARN', 'forms', '별지 15호 부속서류가 생성되지 않았습니다.', '#/workspace/ws-form'),
    ('FORM_FORM22', 'WARN', 'forms', '별지 22호 부속서류가 생성되지 않았습니다.', '#/workspace/ws-form'),
    ('FORM_FORM32', 'INFO', 'forms', '별지 32호 부속서류 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_FORM50', 'INFO', 'forms', '별지 50호 부속서류 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT01', 'INFO', 'forms', '부속서류 ATT01 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT02', 'INFO', 'forms', '부속서류 ATT02 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT03', 'INFO', 'forms', '부속서류 ATT03 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT04', 'INFO', 'forms', '부속서류 ATT04 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT05', 'INFO', 'forms', '부속서류 ATT05 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT06', 'INFO', 'forms', '부속서류 ATT06 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT07', 'INFO', 'forms', '부속서류 ATT07 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT08', 'INFO', 'forms', '부속서류 ATT08 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT09', 'INFO', 'forms', '부속서류 ATT09 검토가 필요합니다.', '#/workspace/ws-form'),
    ('FORM_ATT10', 'INFO', 'forms', '부속서류 ATT10 검토가 필요합니다.', '#/workspace/ws-form'),
    ('EF_FORMAT_SPEC', 'INFO', 'efiling', '전자신고 포맷 스펙을 확인했습니다.', '#/workspace/ws-file'),
    ('EF_PRECHECK', 'INFO', 'efiling', '전자신고 사전검증 결과를 확인하세요.', '#/workspace/ws-file'),
    ('EF_FILE_GENERATED', 'ERROR', 'efiling', 'FILED 상태이지만 전자신고 파일 생성 이력이 없습니다.', '#/workspace/ws-file'),
    ('EF_CHECKSUM', 'INFO', 'efiling', '전자신고 체크섬 검토가 필요합니다.', '#/workspace/ws-file'),
    ('EF_DOWNLOAD', 'INFO', 'efiling', '전자신고 파일 다운로드 가능 여부를 확인하세요.', '#/workspace/ws-file'),
    ('WF_READY_FOR_APPROVAL', 'ERROR', 'workflow', '현재 상태가 DRAFT입니다. 결재요청이 필요합니다.', '#/workspace/ws-appr'),
    ('WF_APPROVAL_LINE', 'INFO', 'workflow', '결재선 검토가 필요합니다.', '#/workspace/ws-appr'),
    ('WF_COMMENT', 'INFO', 'workflow', '결재 의견 기록을 확인하세요.', '#/workspace/ws-appr'),
    ('WF_FILE_LOCKED', 'ERROR', 'workflow', '신고 완료 상태의 잠금/파일 이력이 일치하지 않습니다.', '#/workspace/ws-file'),
    ('POST_AMENDMENT_READY', 'INFO', 'post', '수정신고 진입 전 잠금해제 사유를 확인하세요.', '#/post/post-amend'),
    ('POST_UNREAD_NOTIFICATIONS', 'INFO', 'post', '읽지 않은 알림이 남아 있습니다.', '#/reports/rp-alerts'),
    ('RP_BURDEN_READY', 'INFO', 'reports', '세부담 분석 지표가 갱신되었습니다.', '#/reports/rp-burden'),
    ('RP_COMPARE_READY', 'INFO', 'reports', '사업연도 비교 지표가 갱신되었습니다.', '#/reports/rp-compare'),
    ('RP_RESERVE_READY', 'INFO', 'reports', '유보 잔액 추이 지표가 갱신되었습니다.', '#/reports/rp-reserve'),
    ('RESERVE_REGISTERED', 'INFO', 'reports', '등록된 유보 잔액이 없습니다.', '#/reports/rp-reserve')
ON CONFLICT (rule_code) DO UPDATE
SET severity = EXCLUDED.severity,
    area = EXCLUDED.area,
    message_template = EXCLUDED.message_template,
    applies_to = EXCLUDED.applies_to,
    active = TRUE;

DO $$
DECLARE
    schema_name TEXT;
BEGIN
    FOR schema_name IN
        SELECT t.schema_name
        FROM tenants t
        WHERE to_regclass(format('%I.business_years', t.schema_name)) IS NOT NULL
    LOOP
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I.validation_issues (
                issue_id      BIGSERIAL PRIMARY KEY,
                by_id         BIGINT NOT NULL REFERENCES %I.business_years(by_id),
                rule_code     VARCHAR(80) NOT NULL REFERENCES public.validation_rules(rule_code),
                severity      VARCHAR(20) NOT NULL,
                area          VARCHAR(40) NOT NULL,
                message       TEXT NOT NULL,
                target_path   VARCHAR(200),
                status        VARCHAR(20) NOT NULL DEFAULT ''OPEN'',
                metadata      JSONB NOT NULL DEFAULT ''{}''::jsonb,
                created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                dismissed_at  TIMESTAMPTZ
            )',
            schema_name,
            schema_name
        );
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_validation_issues_by
                ON %I.validation_issues(by_id, status, severity, created_at DESC)',
            schema_name
        );
    END LOOP;
END $$;
