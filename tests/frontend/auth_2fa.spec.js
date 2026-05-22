const fs = require("fs");
const assert = require("assert");

const auth = fs.readFileSync("src/auth.rs", "utf8");
const api = fs.readFileSync("src/api.rs", "utf8");
const app = fs.readFileSync("frontend/app.js", "utf8");
const apiJs = fs.readFileSync("frontend/app/api.js", "utf8");
const index = fs.readFileSync("frontend/index.html", "utf8");
const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");

assert(
  auth.includes("enforce_2fa_for_user(pool, user.user_id, user.use_2fa, request.otp.as_deref()).await?;"),
  "login must enforce per-user 2FA with the submitted OTP before creating a session"
);
assert(
  auth.indexOf("enforce_2fa_for_user(pool") < auth.indexOf("INSERT INTO auth_sessions"),
  "2FA must be enforced before session insertion"
);
assert(api.includes('message.contains("2fa")'), "login API must return 401-style auth errors for 2FA failures");

assert(index.includes('id="loginOtpWrap"'), "login form must include an OTP field wrapper");
assert(index.includes('id="loginOtp"'), "login form must include an OTP input");
assert(index.includes('data-i18n="auth.otp"'), "OTP label must be localized");

assert(apiJs.includes("response.status === 401 && !options.skipUnauthorized"), "API helper must support login challenge 401 handling");
assert(app.includes("let loginOtpChallenge = false;"), "frontend must track OTP challenge state");
assert(app.includes("skipUnauthorized: true"), "login request must not reset the login screen on OTP challenge");
assert(app.includes("...(loginOtpChallenge || otp ? { otp } : {})"), "login request must submit OTP only when present/challenged");
assert(app.includes('message.includes("2fa otp is required")'), "login must branch for missing OTP");
assert(app.includes('message.includes("invalid 2fa otp")'), "login must branch for invalid OTP");
assert(app.includes("function enableLoginOtpChallenge(message)"), "login must reveal OTP challenge UI");
assert(app.includes("function resetLoginOtpChallenge()"), "login must reset OTP challenge UI after success/logout");

for (const key of [
  "auth.otp",
  "auth.otpHelp",
  "auth.otpRequired",
  "auth.otpInvalid",
  "auth.otpEnrollmentRequired",
  "auth.ipBlocked",
  "auth.accountLocked",
  "auth.passwordExpired",
]) {
  assert(i18n.includes(`"${key}"`), `${key} must exist in i18n.js`);
}

console.log("frontend auth_2fa.spec.js passed");
