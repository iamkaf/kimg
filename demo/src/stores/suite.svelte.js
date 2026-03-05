import { writable, derived } from "svelte/store";

export const tests = writable([]);
export const diagnostics = writable([]);
export const simd = writable("Checking");
export const runtimeStatusText = writable("Booting");

export const suiteCounts = derived(tests, ($tests) => ({
  total: $tests.length,
  pass: $tests.filter((t) => t.status === "pass").length,
  fail: $tests.filter((t) => t.status === "fail").length,
  experimental: $tests.filter((t) => t.status === "experimental").length,
  running: $tests.filter((t) => t.status === "running").length,
  pending: $tests.filter((t) => t.status === "pending").length,
}));

export const suiteStatus = derived(tests, ($tests) => {
  if ($tests.length === 0) return "idle";
  if ($tests.some((t) => t.status === "running" || t.status === "pending")) return "running";
  if ($tests.some((t) => t.status === "fail")) return "failed";
  return "done";
});

export const sectionCounts = derived(tests, ($tests) => {
  const counts = {};
  for (const test of $tests) {
    if (!counts[test.section]) {
      counts[test.section] = { pass: 0, fail: 0, experimental: 0, pending: 0, running: 0, total: 0 };
    }
    counts[test.section].total++;
    counts[test.section][test.status] = (counts[test.section][test.status] || 0) + 1;
  }
  return counts;
});

export function recordDiagnostic(level, message) {
  diagnostics.update((d) => [...d, { level, message, time: Date.now() }]);
}

export function updateTest(id, patch) {
  tests.update(($tests) => $tests.map((t) => (t.id === id ? { ...t, ...patch } : t)));
}

export function initTests(descriptors) {
  tests.set(
    descriptors.map((t, i) => ({
      id: i,
      title: t.title,
      section: t.section,
      expectation: t.expectation,
      experimental: t.experimental ?? false,
      featured: t.featured ?? false,
      fullSpan: t.fullSpan ?? false,
      previewMin: t.previewMin ?? null,
      status: "pending",
      result: null,
      error: null,
      elapsed: null,
    })),
  );
}
