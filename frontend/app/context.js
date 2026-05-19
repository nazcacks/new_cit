const KEY = "cit.work.context";

const emptyContext = {
  customerId: null,
  customerName: "",
  byId: null,
  fy: "",
  period: "",
  status: "",
  progress: 0,
  snapshot: null,
  lockMode: "OPEN",
};

export function loadContext() {
  try {
    return { ...emptyContext, ...JSON.parse(localStorage.getItem(KEY) || "{}") };
  } catch {
    return { ...emptyContext };
  }
}

export function saveContext(context) {
  const next = { ...emptyContext, ...context };
  localStorage.setItem(KEY, JSON.stringify(next));
  return next;
}

export function clearContext() {
  localStorage.removeItem(KEY);
  return { ...emptyContext };
}

export function hasWorkContext(context) {
  return Boolean(context?.customerId && context?.byId);
}

export function progressForStatus(status) {
  return {
    DRAFT: 20,
    IN_REVIEW: 75,
    APPROVED: 85,
    FILED: 100,
    AMENDED: 45,
  }[status] || 0;
}
