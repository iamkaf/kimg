import { chromium } from "playwright-core";

const demoUrl = process.env.KIMG_DEMO_URL;
const chromeBin = process.env.CHROME_BIN;

if (!demoUrl) {
  throw new Error("KIMG_DEMO_URL is required");
}

if (!chromeBin) {
  throw new Error("CHROME_BIN is required");
}

const launchArgs = [
  "--disable-dev-shm-usage",
  "--disable-gpu",
  "--run-all-compositor-stages-before-draw",
];

if (process.getuid?.() === 0 || process.env.CI === "true" || process.env.GITHUB_ACTIONS === "true") {
  launchArgs.push("--no-sandbox");
}

const browser = await chromium.launch({
  args: launchArgs,
  executablePath: chromeBin,
  headless: true,
});

try {
  const page = await browser.newPage();
  await page.goto(demoUrl, { waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.waitForFunction(
    () => {
      const status = document.body?.dataset?.suiteStatus;
      return status === "completed" || status === "failed" || status === "fatal";
    },
    { timeout: 30_000 },
  );

  const state = await page.evaluate(() => {
    const diagnostics = Array.from(document.querySelectorAll("#diagnostic-list li")).map((item) =>
      item.textContent?.trim() ?? "",
    );

    return {
      cards: Number(document.body.dataset.suiteCount ?? "0"),
      diagnostics,
      diagnosticCount: Number(document.body.dataset.suiteDiagnostics ?? "0"),
      experimental: Number(document.body.dataset.suiteExperimental ?? "0"),
      fail: Number(document.body.dataset.suiteFail ?? "0"),
      pass: Number(document.body.dataset.suitePass ?? "0"),
      runtimeStatus: document.getElementById("runtime-status")?.textContent?.trim() ?? "",
      status: document.body.dataset.suiteStatus ?? "",
    };
  });

  console.log(
    `demo-status: status=${state.status} cards=${state.cards} pass=${state.pass} fail=${state.fail} experimental=${state.experimental} diagnostics=${state.diagnosticCount}`,
  );

  if (state.status !== "completed") {
    if (state.runtimeStatus) {
      console.log(`demo-runtime-status: ${state.runtimeStatus}`);
    }
    for (const diagnostic of state.diagnostics.slice(0, 5)) {
      console.log(`demo-diagnostic: ${diagnostic}`);
    }
    process.exitCode = 1;
    throw new Error(`demo did not complete cleanly (status=${state.status})`);
  }

  if (state.cards < 20) {
    throw new Error(`demo rendered too few cards (${state.cards})`);
  }

  if (state.fail !== 0) {
    throw new Error(`demo reported failing cards (${state.fail})`);
  }

  if (state.diagnosticCount !== 0) {
    for (const diagnostic of state.diagnostics.slice(0, 5)) {
      console.log(`demo-diagnostic: ${diagnostic}`);
    }
    throw new Error(`demo captured diagnostics (${state.diagnosticCount})`);
  }

  if (state.pass <= 0) {
    throw new Error("demo reported zero passing cards");
  }

  if (state.pass + state.fail + state.experimental !== state.cards) {
    throw new Error(
      `demo counters do not add up (pass=${state.pass}, fail=${state.fail}, experimental=${state.experimental}, cards=${state.cards})`,
    );
  }

  console.log("==> Demo smoke test passed.");
} finally {
  await browser.close();
}
