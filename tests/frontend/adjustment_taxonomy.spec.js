const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");
const modules = fs.readFileSync("src/modules.rs", "utf8");
const tax = fs.readFileSync("src/tax.rs", "utf8");
const design = fs.readFileSync("법인세_세무조정계산서_시스템_설계서.md", "utf8");
const workflow = fs.readFileSync("업무흐름.md", "utf8");
const v18 = fs.readFileSync("추가구현_v1.8.md", "utf8");

const taxonomy = [
  { code: "B1", shortKo: "소득금액조정명세서", ko: "B1 소득금액조정명세서", en: "B1 Income adjustment statement", family: "income", endpoint: "adjustments/income", doc: "B-1. 소득금액조정명세서" },
  { code: "B2", shortKo: "기부금", ko: "B2 기부금", en: "B2 Donations", family: "transactions", endpoint: "adjustments/transactions/B2", doc: "B-2. 기부금" },
  { code: "B3", shortKo: "접대비", ko: "B3 접대비", en: "B3 Entertainment expense", family: "transactions", endpoint: "adjustments/transactions/B3", doc: "B-3. 기업업무추진비(접대비)" },
  { code: "B4", shortKo: "감가상각비", ko: "B4 감가상각비", en: "B4 Depreciation expense", family: "assets", endpoint: "adjustments/assets/B4", doc: "B-4. 감가상각비" },
  { code: "B5", shortKo: "퇴직급여충당금/퇴직연금", ko: "B5 퇴직급여충당금/퇴직연금", en: "B5 Retirement allowance reserve/pension", family: "assets", endpoint: "adjustments/assets/B5", doc: "B-5. 퇴직급여충당금/퇴직연금" },
  { code: "B6", shortKo: "대손충당금 및 대손금", ko: "B6 대손충당금 및 대손금", en: "B6 Bad debt reserve and bad debts", family: "assets", endpoint: "adjustments/assets/B6", doc: "B-6. 대손충당금 및 대손금" },
  { code: "B7", shortKo: "외화자산·부채 평가", ko: "B7 외화자산·부채 평가", en: "B7 Foreign currency asset/liability valuation", family: "evaluation", endpoint: "adjustments/evaluation/B7", doc: "B-7. 외화자산·부채 평가" },
  { code: "B8", shortKo: "재고자산·유가증권 평가", ko: "B8 재고자산·유가증권 평가", en: "B8 Inventory/securities valuation", family: "evaluation", endpoint: "adjustments/evaluation/B8", doc: "B-8. 재고자산·유가증권 평가" },
  { code: "B9", shortKo: "지급이자 손금불산입", ko: "B9 지급이자 손금불산입", en: "B9 Non-deductible interest expense", family: "transactions", endpoint: "adjustments/transactions/B9", doc: "B-9. 지급이자 손금불산입" },
  { code: "B10", shortKo: "업무용승용차 관련비용", ko: "B10 업무용승용차 관련비용", en: "B10 Business vehicle expenses", family: "assets", endpoint: "adjustments/assets/B10", doc: "B-10. 업무용승용차 관련비용" },
  { code: "B11", shortKo: "이월결손금", ko: "B11 이월결손금", en: "B11 Loss carryforward", family: "evaluation", endpoint: "adjustments/evaluation/B11", doc: "B-11. 이월결손금" },
  { code: "B12", shortKo: "세액공제·감면", ko: "B12 세액공제·감면", en: "B12 Tax credits/reductions", family: "tax", endpoint: "adjustments/tax/B12", doc: "B-12. 세액공제·감면" },
  { code: "B13", shortKo: "최저한세", ko: "B13 최저한세", en: "B13 Minimum tax", family: "tax", endpoint: "adjustments/tax/B13", doc: "B-13. 최저한세" },
  { code: "B14", shortKo: "가산세", ko: "B14 가산세", en: "B14 Additional tax", family: "tax", endpoint: "adjustments/tax/B14", doc: "B-14. 가산세" },
  { code: "B15", shortKo: "자본금과 적립금", ko: "B15 자본금과 적립금", en: "B15 Capital and reserves", family: "evaluation", endpoint: "adjustments/evaluation/B15", doc: "B-15. 자본금과 적립금" },
  { code: "B16", shortKo: "외국법인 세무조정", ko: "B16 외국법인 세무조정", en: "B16 Foreign corporation adjustment", family: "special", endpoint: "adjustments/special/B16", doc: "B-16. 외국법인 세무조정" },
  { code: "B17", shortKo: "연결납세", ko: "B17 연결납세", en: "B17 Consolidated tax", family: "special", endpoint: "adjustments/special/B17", doc: "B-17. 연결납세" },
];

function rustFunctionBody(source, name) {
  const start = source.indexOf(`fn ${name}`);
  assert(start >= 0, `${name} must exist`);
  const next = source.indexOf("\nfn ", start + 1);
  return source.slice(start, next > start ? next : undefined);
}

const normalizerByFamily = {
  assets: rustFunctionBody(tax, "normalize_asset_module"),
  transactions: rustFunctionBody(tax, "normalize_transaction_module"),
  evaluation: rustFunctionBody(tax, "normalize_evaluation_module"),
  tax: rustFunctionBody(tax, "normalize_tax_amount_module"),
  special: rustFunctionBody(tax, "normalize_special_module"),
};

for (const item of taxonomy) {
  const shortEn = item.en.replace(`${item.code} `, "");
  const docCode = item.code.replace("B", "B-");
  assert(design.includes(item.doc), `${item.code} design heading is missing`);
  assert(workflow.includes(item.code.replace("B", "B-")) || workflow.includes(item.shortKo), `${item.code} workflow reference is missing`);
  assert(v18.includes(`- ${docCode}: ${item.shortKo}`), `${item.code} v1.8 taxonomy entry is missing`);

  assert(
    screens.includes(`{ code: "${item.code}", ko: "${item.shortKo}", en: "${shortEn}", module: "${item.family}", api: "${item.endpoint}" }`),
    `${item.code} frontend adjustmentTaxonomy mismatch`
  );
  const specMatch = screens.match(new RegExp(`"ws/adj:${item.code}": leafSpec\\("GET", "([^"]+)"`));
  assert(specMatch, `${item.code} leaf spec is missing`);
  assert(specMatch[1].includes(`/${item.endpoint}`), `${item.code} leaf spec must use ${item.endpoint}`);

  assert(i18n.includes(`"route.ws.adj.${item.code}": "${item.ko}"`), `${item.code} Korean i18n label mismatch`);
  assert(i18n.includes(`"route.ws.adj.${item.code}": "${item.en}"`), `${item.code} English i18n label mismatch`);

  assert(modules.includes(`"${item.en}"`), `${item.code} menu English label mismatch`);
  assert(modules.includes(`"ws/adj:${item.code}" => "${item.ko}"`), `${item.code} menu Korean label mismatch`);

  if (item.family !== "income") {
    assert(normalizerByFamily[item.family].includes(`"${item.code}"`), `${item.code} backend normalizer must accept ${item.family}`);
  }
}

assert(!screens.includes("Business transfer difference"), "B10 must not use stale business transfer label");
assert(!screens.includes("B5\", \"인정이자\""), "B5 must not use stale deemed-interest frontend taxonomy");
assert(!i18n.includes("B10 Business transfer difference"), "B10 English i18n must not be stale");
assert(!modules.includes("B5 Deemed interest"), "B5 menu English label must not be stale");

console.log("frontend adjustment_taxonomy.spec.js passed");
