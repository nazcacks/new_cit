$BaseUrl = "http://localhost:8080"
$TenantCode = "demo"

Invoke-RestMethod "$BaseUrl/health"

Invoke-RestMethod -Method Post "$BaseUrl/api/tenants" `
  -ContentType "application/json" `
  -Body (@{
    tenant_code = $TenantCode
    tenant_name = "Demo Tax Firm"
    biz_reg_no = "1234567890"
    contract_start = "2026-01-01"
    max_users = 20
  } | ConvertTo-Json)

$customer = Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/customers" `
  -ContentType "application/json" `
  -Body (@{
    customer_code = "CUST001"
    customer_name = "서울테크 주식회사"
    biz_reg_no = "2208112345"
    corp_reg_no = "1101111234567"
    industry_code = "62010"
    is_sme = $true
  } | ConvertTo-Json)

$businessYear = Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/business-years" `
  -ContentType "application/json" `
  -Body (@{
    customer_id = $customer.customer_id
    year_label = 2026
    start_date = "2026-01-01"
    end_date = "2026-12-31"
  } | ConvertTo-Json)

Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/business-years/$($businessYear.by_id)/snapshot"

$adjustmentBody = Get-Content "$PSScriptRoot/adjustment_request.json" -Raw
Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/business-years/$($businessYear.by_id)/adjustments" `
  -ContentType "application/json" `
  -Body $adjustmentBody

Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/business-years/$($businessYear.by_id)/forms/FORM3"

$job = Invoke-RestMethod -Method Post "$BaseUrl/api/tenants/$TenantCode/business-years/$($businessYear.by_id)/efilings" `
  -ContentType "application/json" `
  -Body '{"max_attempts":3}'

Write-Host "Queued e-filing job $($job.job_id)"
