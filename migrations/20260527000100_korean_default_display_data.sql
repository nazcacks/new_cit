UPDATE tenants
SET tenant_name = CASE tenant_name
    WHEN 'Demo Corporate Tax Workspace' THEN '데모 법인세 신고 작업장'
    WHEN 'Demo Tax Firm' THEN '샘플 세무법인'
    WHEN 'Dashboard Work Status' THEN '대시보드 작업 상태'
    WHEN 'Dashboard Deadlines' THEN '대시보드 마감 현황'
    WHEN 'Dashboard Notifications' THEN '대시보드 알림'
    WHEN 'Dashboard Approval Actions' THEN '대시보드 결재 작업'
    WHEN 'Dashboard Recent Activity' THEN '대시보드 최근 활동'
    WHEN 'Dashboard KPI Tax Burden' THEN '대시보드 KPI 세부담'
    WHEN 'Dashboard KPI Industry Loss' THEN '대시보드 KPI 업종 결손'
    ELSE tenant_name
END,
updated_at = NOW()
WHERE tenant_name IN (
    'Demo Corporate Tax Workspace',
    'Demo Tax Firm',
    'Dashboard Work Status',
    'Dashboard Deadlines',
    'Dashboard Notifications',
    'Dashboard Approval Actions',
    'Dashboard Recent Activity',
    'Dashboard KPI Tax Burden',
    'Dashboard KPI Industry Loss'
);

UPDATE roles
SET role_name = CASE role_code
    WHEN 'TAX_WRITER' THEN '작성 담당자'
    WHEN 'TAX_REVIEWER' THEN '검토 담당자'
    WHEN 'TAX_EXPERT' THEN '세무조정 전문가'
    WHEN 'TENANT_ADMIN' THEN '테넌트 관리자'
    WHEN 'SUPER_ADMIN' THEN '슈퍼 관리자'
    ELSE role_name
END,
description = CASE role_code
    WHEN 'TAX_WRITER' THEN '데모 자료 입력 및 서식 작성 담당자'
    WHEN 'TAX_REVIEWER' THEN '데모 검토 및 승인 담당자'
    WHEN 'TAX_EXPERT' THEN '데모 세무조정 전문가'
    WHEN 'TENANT_ADMIN' THEN '데모 테넌트 관리자'
    WHEN 'SUPER_ADMIN' THEN '데모 전체 관리자'
    ELSE description
END
WHERE role_code IN ('TAX_WRITER', 'TAX_REVIEWER', 'TAX_EXPERT', 'TENANT_ADMIN', 'SUPER_ADMIN');

UPDATE tax_law_versions
SET law_name = regexp_replace(law_name, '^Corporate Income Tax Act ([0-9]{4})$', '법인세법 \1')
WHERE law_name ~ '^Corporate Income Tax Act [0-9]{4}$';

UPDATE tax_limits
SET metadata = jsonb_set(metadata, '{description}', to_jsonb('접대비 기본한도'::TEXT), TRUE)
WHERE item_code = 'ENTERTAINMENT_BASE_LIMIT'
  AND metadata->>'description' = 'base entertainment expense limit';

UPDATE tax_limits
SET metadata = jsonb_set(metadata, '{description}', to_jsonb('기부금 차감 전 소득의 10%'::TEXT), TRUE)
WHERE item_code = 'DONATION_LIMIT_BPS'
  AND metadata->>'description' = '10 percent of pre-donation income';

UPDATE tax_limits
SET metadata = jsonb_set(
    metadata,
    '{description}',
    to_jsonb((CASE metadata->>'description'
        WHEN 'R&D tax credit rate in bps' THEN '연구개발 세액공제율 bps'
        WHEN 'SME revenue threshold' THEN '중소기업 매출 기준'
        WHEN 'loss carryforward years' THEN '이월결손금 공제 가능 연수'
        WHEN 'special donation limit rate in bps' THEN '특례기부금 한도율 bps'
        WHEN 'general donation limit rate in bps' THEN '일반기부금 한도율 bps'
        WHEN 'donation carryforward years' THEN '기부금 이월공제 가능 연수'
        WHEN 'revenue based entertainment limit rate in bps' THEN '수입금액 기준 접대비 한도율 bps'
        WHEN 'non-card entertainment disallowance rate in bps' THEN '비카드 접대비 손금불산입률 bps'
        WHEN 'default weighted average interest rate in bps' THEN '기본 가중평균차입이자율 bps'
        WHEN 'SME loss deduction limit rate in bps' THEN '중소기업 이월결손금 공제한도율 bps'
        WHEN 'general company loss deduction limit rate in bps' THEN '일반법인 이월결손금 공제한도율 bps'
        WHEN 'integrated investment credit rate in bps' THEN '통합투자세액공제율 bps'
        WHEN 'foreign tax credit maximum bps' THEN '외국납부세액공제 한도 bps'
        WHEN 'disaster tax credit rate in bps' THEN '재해손실 세액공제율 bps'
        WHEN 'SME special reduction rate in bps' THEN '중소기업 특별세액감면율 bps'
        WHEN 'startup reduction rate in bps' THEN '창업감면율 bps'
        WHEN 'SME minimum tax rate in bps' THEN '중소기업 최저한세율 bps'
        WHEN 'general minimum tax rate in bps' THEN '일반법인 최저한세율 bps'
        ELSE metadata->>'description'
    END)::TEXT),
    TRUE
)
WHERE metadata->>'description' IN (
    'R&D tax credit rate in bps',
    'SME revenue threshold',
    'loss carryforward years',
    'special donation limit rate in bps',
    'general donation limit rate in bps',
    'donation carryforward years',
    'revenue based entertainment limit rate in bps',
    'non-card entertainment disallowance rate in bps',
    'default weighted average interest rate in bps',
    'SME loss deduction limit rate in bps',
    'general company loss deduction limit rate in bps',
    'integrated investment credit rate in bps',
    'foreign tax credit maximum bps',
    'disaster tax credit rate in bps',
    'SME special reduction rate in bps',
    'startup reduction rate in bps',
    'SME minimum tax rate in bps',
    'general minimum tax rate in bps'
);

UPDATE form_versions
SET form_name = CASE form_code
    WHEN 'FORM3' THEN '법인세 과세표준 및 세액조정계산서'
    WHEN 'FORM15' THEN '소득금액조정명세서'
    WHEN 'FORM22' THEN '기부금 조정명세서'
    WHEN 'FORM32' THEN '유보 변동 명세서'
    WHEN 'FORM50' THEN '전자신고 요약 명세서'
    WHEN 'ATT01' THEN '재무제표 첨부서식'
    WHEN 'ATT02' THEN '자산대장 첨부서식'
    WHEN 'ATT03' THEN '거래명세 첨부서식'
    WHEN 'ATT04' THEN '업무용승용차 첨부서식'
    WHEN 'ATT05' THEN '결재 승인 첨부서식'
    WHEN 'ATT06' THEN '검증 결과 첨부서식'
    WHEN 'ATT07' THEN '세액공제 첨부서식'
    WHEN 'ATT08' THEN '이월결손금 첨부서식'
    WHEN 'ATT09' THEN '국외소득 첨부서식'
    WHEN 'ATT10' THEN '연결납세 첨부서식'
    ELSE form_name
END
WHERE form_code IN (
    'FORM3', 'FORM15', 'FORM22', 'FORM32', 'FORM50',
    'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
    'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
);

UPDATE tax_forms
SET form_name = CASE form_code
    WHEN 'FORM3' THEN '법인세 과세표준 및 세액조정계산서'
    WHEN 'FORM15' THEN '소득금액조정명세서'
    WHEN 'FORM22' THEN '기부금 조정명세서'
    WHEN 'FORM32' THEN '유보 변동 명세서'
    WHEN 'FORM50' THEN '전자신고 요약 명세서'
    WHEN 'ATT01' THEN '재무제표 첨부서식'
    WHEN 'ATT02' THEN '자산대장 첨부서식'
    WHEN 'ATT03' THEN '거래명세 첨부서식'
    WHEN 'ATT04' THEN '업무용승용차 첨부서식'
    WHEN 'ATT05' THEN '결재 승인 첨부서식'
    WHEN 'ATT06' THEN '검증 결과 첨부서식'
    WHEN 'ATT07' THEN '세액공제 첨부서식'
    WHEN 'ATT08' THEN '이월결손금 첨부서식'
    WHEN 'ATT09' THEN '국외소득 첨부서식'
    WHEN 'ATT10' THEN '연결납세 첨부서식'
    ELSE form_name
END,
description = CASE
    WHEN form_code IN (
        'FORM3', 'FORM15', 'FORM22', 'FORM32', 'FORM50',
        'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
        'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
    ) THEN '기본 법인세 서식'
    ELSE description
END,
updated_at = NOW()
WHERE form_code IN (
    'FORM3', 'FORM15', 'FORM22', 'FORM32', 'FORM50',
    'ATT01', 'ATT02', 'ATT03', 'ATT04', 'ATT05',
    'ATT06', 'ATT07', 'ATT08', 'ATT09', 'ATT10'
);

UPDATE efile_record_layouts
SET record_name = CASE record_name
    WHEN 'Header' THEN '헤더'
    WHEN 'Detail' THEN '상세'
    WHEN 'Trailer' THEN '합계'
    ELSE record_name
END
WHERE record_name IN ('Header', 'Detail', 'Trailer');

UPDATE form_validations
SET message = '필수 입력 항목입니다.'
WHERE message LIKE '% is required';

UPDATE user_report_definitions
SET report_name = regexp_replace(report_name, '^Custom report', '사용자 리포트'),
    updated_at = NOW()
WHERE report_name LIKE 'Custom report%';

UPDATE user_report_definitions
SET report_name = regexp_replace(report_name, '^Loss expiry', '결손금 만료'),
    updated_at = NOW()
WHERE report_name LIKE 'Loss expiry%';

DO $$
DECLARE
    tenant_schema TEXT;
BEGIN
    FOR tenant_schema IN
        SELECT schema_name
        FROM tenants
        WHERE to_regnamespace(schema_name) IS NOT NULL
          AND to_regclass(format('%I.customers', schema_name)) IS NOT NULL
    LOOP
        EXECUTE format($sql$
            UPDATE %I.customers
            SET customer_name = CASE customer_name
                WHEN 'Alpha Manufacturing Co.' THEN '알파 제조 주식회사'
                WHEN 'Beta Platform Services' THEN '베타 플랫폼 서비스'
                WHEN 'Gamma Bio Research' THEN '감마 바이오 연구소'
                WHEN 'Dashboard Customer' THEN '대시보드 고객사'
                WHEN 'Deadline Customer' THEN '마감 관리 고객사'
                WHEN 'Notification Customer' THEN '알림 고객사'
                WHEN 'Approval Action Customer' THEN '결재 작업 고객사'
                WHEN 'Activity Customer' THEN '활동 이력 고객사'
                WHEN 'KPI Customer' THEN 'KPI 고객사'
                ELSE customer_name
            END,
            updated_at = NOW()
            WHERE customer_name IN (
                'Alpha Manufacturing Co.',
                'Beta Platform Services',
                'Gamma Bio Research',
                'Dashboard Customer',
                'Deadline Customer',
                'Notification Customer',
                'Approval Action Customer',
                'Activity Customer',
                'KPI Customer'
            );
        $sql$, tenant_schema);

        IF to_regclass(format('%I.notifications', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.notifications
                SET title = CASE title
                        WHEN 'Approval requested' THEN '결재 요청'
                        WHEN 'Approval completed' THEN '결재 완료'
                        WHEN 'Approval returned' THEN '결재 반려'
                        WHEN 'Filing completed' THEN '신고 완료'
                        WHEN 'Amendment opened' THEN '수정신고 시작'
                        ELSE title
                    END,
                    message = CASE message
                        WHEN 'A business year is waiting for approval' THEN '사업연도가 결재 대기 중입니다.'
                        WHEN 'All approval lines are approved' THEN '모든 결재선이 승인되었습니다.'
                        WHEN 'Approval was returned to draft' THEN '결재가 반려되어 작성 단계로 돌아갔습니다.'
                        WHEN 'The business year has been filed and locked' THEN '사업연도 신고가 완료되어 잠겼습니다.'
                        WHEN 'The filed business year was unlocked for amendment' THEN '신고 완료된 사업연도가 수정신고용으로 열렸습니다.'
                        ELSE message
                    END
                WHERE title IN (
                    'Approval requested',
                    'Approval completed',
                    'Approval returned',
                    'Filing completed',
                    'Amendment opened'
                )
                OR message IN (
                    'A business year is waiting for approval',
                    'All approval lines are approved',
                    'Approval was returned to draft',
                    'The business year has been filed and locked',
                    'The filed business year was unlocked for amendment'
                );
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.form_data_history', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.form_data_history
                SET reason = CASE reason
                    WHEN 'form engine generation' THEN '서식 엔진 생성'
                    WHEN 'user override' THEN '사용자 수동 수정'
                    WHEN 'tax adjustment summary' THEN '세무조정 요약'
                    ELSE reason
                END
                WHERE reason IN ('form engine generation', 'user override', 'tax adjustment summary');
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.form_data', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.form_data
                SET data_json = replace(
                    replace(
                        replace(
                            replace(
                                replace(
                                    replace(data_json::TEXT,
                                        '"source_ref": "tax adjustment summary"', '"source_ref": "세무조정 요약"'),
                                    '"source_ref":"tax adjustment summary"', '"source_ref":"세무조정 요약"'),
                                '"source_ref": "user override"', '"source_ref": "사용자 수동 수정"'),
                            '"source_ref":"user override"', '"source_ref":"사용자 수동 수정"'),
                        '"source_ref": "form engine generation"', '"source_ref": "서식 엔진 생성"'),
                    '"source_ref":"form engine generation"', '"source_ref":"서식 엔진 생성"')::JSONB
                WHERE data_json::TEXT LIKE '%%tax adjustment summary%%'
                   OR data_json::TEXT LIKE '%%user override%%'
                   OR data_json::TEXT LIKE '%%form engine generation%%';
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.fs_lines', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.fs_lines
                SET account_name = CASE account_name
                        WHEN 'Cash' THEN '현금'
                        WHEN 'Accounts payable' THEN '미지급금'
                        ELSE account_name
                    END,
                    standard_account_name = CASE standard_account_name
                        WHEN 'Cash' THEN '현금'
                        WHEN 'Accounts payable' THEN '미지급금'
                        ELSE standard_account_name
                    END
                WHERE account_name IN ('Cash', 'Accounts payable')
                   OR standard_account_name IN ('Cash', 'Accounts payable');
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.account_mappings', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.account_mappings
                SET source_account_name = CASE source_account_name
                        WHEN 'Cash' THEN '현금'
                        WHEN 'Accounts payable' THEN '미지급금'
                        ELSE source_account_name
                    END,
                    standard_account_name = CASE standard_account_name
                        WHEN 'Cash' THEN '현금'
                        WHEN 'Accounts payable' THEN '미지급금'
                        ELSE standard_account_name
                    END,
                    updated_at = NOW()
                WHERE source_account_name IN ('Cash', 'Accounts payable')
                   OR standard_account_name IN ('Cash', 'Accounts payable');
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.assets', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.assets
                SET asset_name = CASE asset_name
                    WHEN 'Company sedan' THEN '업무용 승용차'
                    WHEN 'CNC machine' THEN 'CNC 장비'
                    ELSE asset_name
                END
                WHERE asset_name IN ('Company sedan', 'CNC machine');
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.transactions', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.transactions
                SET partner_name = CASE partner_name
                        WHEN 'Good Charity' THEN '좋은나눔재단'
                        WHEN 'Client Dinner' THEN '거래처 만찬'
                        ELSE partner_name
                    END,
                    description = CASE description
                        WHEN 'Donation receipt' THEN '기부금 영수증'
                        WHEN 'Dinner meeting' THEN '저녁 회의'
                        ELSE description
                    END
                WHERE partner_name IN ('Good Charity', 'Client Dinner')
                   OR description IN ('Donation receipt', 'Dinner meeting');
            $sql$, tenant_schema);
        END IF;

        IF to_regclass(format('%I.adjustment_items', tenant_schema)) IS NOT NULL THEN
            EXECUTE format($sql$
                UPDATE %I.adjustment_items
                SET item_name = CASE
                        WHEN item_name LIKE '%% depreciation limit excess' THEN regexp_replace(item_name, ' depreciation limit excess$', ' 감가상각 한도초과')
                        WHEN item_name LIKE '%% business vehicle limit excess' THEN regexp_replace(item_name, ' business vehicle limit excess$', ' 업무용승용차 한도초과')
                        WHEN item_name = 'Retirement reserve limit adjustment' THEN '퇴직급여충당금 한도 조정'
                        WHEN item_name = 'Bad debt reserve limit excess' THEN '대손충당금 한도초과'
                        WHEN item_name = 'Special donation limit excess' THEN '특례기부금 한도초과'
                        WHEN item_name = 'General donation limit excess' THEN '일반기부금 한도초과'
                        WHEN item_name = 'Prior donation carryforward used' THEN '전기 이월기부금 사용'
                        WHEN item_name = 'Entertainment expense without qualified evidence' THEN '적격증빙 없는 접대비'
                        WHEN item_name = 'Entertainment expense limit excess' THEN '접대비 한도초과'
                        WHEN item_name = 'Interest paid to unidentified creditor' THEN '채권자 불분명 지급이자'
                        WHEN item_name = 'Interest paid to unidentified recipient' THEN '수령자 불분명 지급이자'
                        WHEN item_name = 'Construction financing interest' THEN '건설자금이자'
                        WHEN item_name = 'Non-business asset related interest' THEN '업무무관자산 관련 지급이자'
                        WHEN item_name = 'Deemed interest from weighted loan balance' THEN '가중평균 차입금 인정이자'
                        WHEN item_name = 'Manual interest disallowance' THEN '수동 지급이자 손금불산입'
                        WHEN item_name = 'Loss carryforward deduction' THEN '이월결손금 공제'
                        WHEN item_name = 'Minimum tax additional amount' THEN '최저한세 추가세액'
                        WHEN item_name = 'Domestic source income allocated to PE' THEN '국내사업장 귀속 국내원천소득'
                        WHEN item_name = 'Consolidated tax base after eliminations' THEN '내부거래 제거 후 연결 과세표준'
                        ELSE item_name
                    END,
                    law_ref = CASE law_ref
                        WHEN 'CIT donation limit' THEN '법인세법 기부금 한도'
                        WHEN 'CIT donation carryforward' THEN '법인세법 기부금 이월공제'
                        WHEN 'CIT entertainment evidence rule' THEN '법인세법 접대비 증빙 규정'
                        WHEN 'CIT entertainment limit' THEN '법인세법 접대비 한도'
                        WHEN 'CIT interest expense disallowance' THEN '법인세법 지급이자 손금불산입'
                        WHEN 'CIT valuation rule' THEN '법인세법 평가 규정'
                        WHEN 'CIT loss carryforward rule' THEN '법인세법 이월결손금 공제 규정'
                        WHEN 'Capital and reserve schedule' THEN '자본금과 적립금 조정명세'
                        WHEN 'Capital change' THEN '자본 변동'
                        WHEN 'CIT tax credit rule' THEN '법인세법 세액공제 규정'
                        WHEN 'CIT minimum tax rule' THEN '법인세법 최저한세 규정'
                        WHEN 'Penalty tax rule' THEN '가산세 규정'
                        WHEN 'Foreign corporation domestic source income' THEN '외국법인 국내원천소득 규정'
                        WHEN 'Consolidated tax rule' THEN '연결납세 규정'
                        ELSE law_ref
                    END
                WHERE item_name LIKE '%% depreciation limit excess'
                   OR item_name LIKE '%% business vehicle limit excess'
                   OR item_name IN (
                        'Retirement reserve limit adjustment',
                        'Bad debt reserve limit excess',
                        'Special donation limit excess',
                        'General donation limit excess',
                        'Prior donation carryforward used',
                        'Entertainment expense without qualified evidence',
                        'Entertainment expense limit excess',
                        'Interest paid to unidentified creditor',
                        'Interest paid to unidentified recipient',
                        'Construction financing interest',
                        'Non-business asset related interest',
                        'Deemed interest from weighted loan balance',
                        'Manual interest disallowance',
                        'Loss carryforward deduction',
                        'Minimum tax additional amount',
                        'Domestic source income allocated to PE',
                        'Consolidated tax base after eliminations'
                   )
                   OR law_ref IN (
                        'CIT donation limit',
                        'CIT donation carryforward',
                        'CIT entertainment evidence rule',
                        'CIT entertainment limit',
                        'CIT interest expense disallowance',
                        'CIT valuation rule',
                        'CIT loss carryforward rule',
                        'Capital and reserve schedule',
                        'Capital change',
                        'CIT tax credit rule',
                        'CIT minimum tax rule',
                        'Penalty tax rule',
                        'Foreign corporation domestic source income',
                        'Consolidated tax rule'
                   );
            $sql$, tenant_schema);
        END IF;
    END LOOP;
END $$;
