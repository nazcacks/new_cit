# Examples

Use these examples after starting the stack:

```powershell
docker compose up --build -d postgres api
```

- `flow.ps1`: end-to-end API flow.
- `adjustment_request.json`: sample tax adjustment request body.
- `dead_letter_job.json`: payload that intentionally moves to the DLQ.
