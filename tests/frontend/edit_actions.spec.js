const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

for (const selector of [
  "data-row-edit",
  "data-leaf-edit-form",
  "data-edit-error",
  "data-leaf-form",
  "data-step-edit",
  "data-chart-config-edit",
]) {
  assert(screens.includes(selector), `${selector} edit entrypoint must exist`);
}

for (const fn of [
  "handleLeafClick",
  "handleLeafSubmit",
  "openEditModal",
  "renderEditField",
  "readLeafFormValues",
  "updateLeafRow",
  "upsertLeafRow",
  "rerenderLeaf",
  "closeLeafModal",
]) {
  assert(screens.includes(`function ${fn}`), `${fn} must exist`);
}

assert(screens.includes('method: "PATCH"'), "record edits must PATCH persistent leaf records");
assert(screens.includes('method: "POST"'), "API row edits must POST through leaf action fallback");
assert(screens.includes("message.textContent = error.message"), "edit failures must keep the form open and show the error");
assert(screens.includes("form.querySelectorAll(\"[name]\")"), "edit forms must read named prefilled controls");
assert(screens.includes("row[column.key]"), "edit modal must prefill values from the selected row");
assert(screens.includes("state.rows[index] = row") || screens.includes("state.rows[index] ="), "saved edits must update the current row collection");

for (const representative of [
  "admin/cust:list",
  "admin/sec:menus",
  "ws/start:snapshot",
  "post/amend:unlock",
]) {
  assert(screens.includes(representative), `${representative} representative leaf must remain registered`);
}

console.log("frontend edit_actions.spec.js passed");
