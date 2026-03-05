<script>
  import { onMount } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import TestSection from "./lib/TestSection.svelte";
  import TestCard from "./lib/TestCard.svelte";
  import {
    tests,
    suiteStatus,
    simd,
    runtimeStatusText,
    initTests,
    updateTest,
    recordDiagnostic,
  } from "./stores/suite.svelte.js";
  import { SECTION_ORDER } from "./constants.js";
  import { buildContext, resolveDemoPreloadInput, toErrorMessage, stringifyArgs } from "./helpers/context.js";
  import { createTests } from "./tests/index.js";
  import preload, { simdSupported } from "#kimg/index.js";

  let runSequence = 0;

  const sectionTests = $derived(
    SECTION_ORDER.map((key) => ({
      key,
      items: $tests.filter((t) => t.section === key),
    })).filter((s) => s.items.length > 0),
  );

  onMount(() => {
    installDiagnostics();
    void runSuite();
  });

  async function runSuite() {
    const runId = ++runSequence;
    runtimeStatusText.set("Initializing");
    simd.set("Checking");

    const descriptors = createTests();
    initTests(descriptors);

    try {
      await preload(resolveDemoPreloadInput());
      if (runId !== runSequence) return;

      const context = await buildContext();
      if (runId !== runSequence) return;

      runtimeStatusText.set(`Ready · ${context.fixture.width}×${context.fixture.height} teapot`);
      simd.set(context.runtime.simd ? "Available" : "Scalar");

      for (let i = 0; i < descriptors.length; i++) {
        if (runId !== runSequence) return;
        const test = descriptors[i];
        updateTest(i, { status: "running" });
        const startedAt = performance.now();

        try {
          const result = await test.run(context);
          const elapsed = performance.now() - startedAt;
          updateTest(i, {
            status: test.experimental ? "experimental" : "pass",
            result,
            elapsed,
          });
        } catch (err) {
          const elapsed = performance.now() - startedAt;
          const msg = toErrorMessage(err);
          updateTest(i, { status: "fail", error: msg, elapsed });
          recordDiagnostic("error", `[${test.title}] ${msg}`);
        }
      }

      const fail = $tests.filter((t) => t.status === "fail").length;
      runtimeStatusText.set(fail === 0 ? "Completed without failures" : "Completed with failures");
    } catch (err) {
      runtimeStatusText.set("Fatal error");
      recordDiagnostic("error", `[fatal] ${toErrorMessage(err)}`);
    }
  }

  function installDiagnostics() {
    const origError = console.error.bind(console);
    const origWarn = console.warn.bind(console);

    console.error = (...args) => {
      recordDiagnostic("error", stringifyArgs(args));
      origError(...args);
    };
    console.warn = (...args) => {
      recordDiagnostic("warn", stringifyArgs(args));
      origWarn(...args);
    };

    window.addEventListener("error", (e) => {
      recordDiagnostic("error", e.message || "Unknown window error");
    });
    window.addEventListener("unhandledrejection", (e) => {
      recordDiagnostic("error", toErrorMessage(e.reason));
    });
  }
</script>

<svelte:head>
  <title>kimg Visual Test Suite</title>
</svelte:head>

<div class="app-layout">
  <Sidebar onRerun={runSuite} />

  <main class="main-content">
    <div class="content-inner">
      {#each sectionTests as { key, items }}
        <TestSection sectionKey={key}>
          {#each items as test (test.id)}
            <TestCard {test} />
          {/each}
        </TestSection>
      {/each}

      {#if $tests.length === 0}
        <div class="empty-state">
          <p class="empty-title">Initializing test suite…</p>
          <p class="empty-sub">Loading WASM runtime and fixtures.</p>
        </div>
      {/if}
    </div>
  </main>
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }
  .main-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .content-inner {
    padding: 24px 28px 64px;
    max-width: 1600px;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 80px 24px;
    gap: 8px;
    color: var(--text-muted);
  }
  .empty-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }
  .empty-sub { font-size: 12px; }
</style>
