UPDATE function_codes
SET function_name = CASE function_code
        WHEN 'READ' THEN '조회'
        WHEN 'CREATE' THEN '생성'
        WHEN 'UPDATE' THEN '수정'
        WHEN 'DELETE' THEN '삭제'
        WHEN 'IMPORT' THEN '가져오기'
        WHEN 'EXPORT' THEN '내보내기'
        WHEN 'CALCULATE' THEN '계산'
        WHEN 'APPROVE' THEN '승인'
        WHEN 'EFILE' THEN '전자신고'
        WHEN 'PRINT' THEN '출력'
        WHEN 'MASK_OFF' THEN '마스킹 해제'
        WHEN 'DELEGATE' THEN '위임'
        ELSE function_name
    END,
    description = CASE function_code
        WHEN 'READ' THEN '레코드와 화면 조회'
        WHEN 'CREATE' THEN '레코드 생성'
        WHEN 'UPDATE' THEN '레코드 수정'
        WHEN 'DELETE' THEN '레코드 삭제'
        WHEN 'IMPORT' THEN '파일 또는 행 가져오기'
        WHEN 'EXPORT' THEN '파일 또는 행 내보내기'
        WHEN 'CALCULATE' THEN '세무 계산 실행'
        WHEN 'APPROVE' THEN '워크플로 항목 승인'
        WHEN 'EFILE' THEN '전자신고 파일 생성'
        WHEN 'PRINT' THEN 'PDF/출력물 생성'
        WHEN 'MASK_OFF' THEN '마스킹 해제된 민감 필드 조회'
        WHEN 'DELEGATE' THEN '배정된 접근권한 위임'
        ELSE description
    END
WHERE function_code IN (
    'READ',
    'CREATE',
    'UPDATE',
    'DELETE',
    'IMPORT',
    'EXPORT',
    'CALCULATE',
    'APPROVE',
    'EFILE',
    'PRINT',
    'MASK_OFF',
    'DELEGATE'
);

UPDATE form_relationships
SET rule_json = jsonb_set(
    rule_json,
    '{description}',
    to_jsonb((CASE rule_json->>'description'
        WHEN 'carry forward taxable income' THEN '과세표준 이월'
        WHEN 'carry forward accounting income' THEN '회계상 소득 이월'
        WHEN 'carry forward donation amount' THEN '기부금 금액 이월'
        ELSE rule_json->>'description'
    END)::TEXT),
    TRUE
)
WHERE rule_json->>'description' IN (
    'carry forward taxable income',
    'carry forward accounting income',
    'carry forward donation amount'
);
