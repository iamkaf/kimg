import { detectFormat } from "#kimg/index.js";
import { createVerifier } from "../helpers/verifier.js";
import { messageView } from "../helpers/views.js";

export const experimentalTests = [
  {
    expectation:
      "PSD stays visible on the page as experimental only. This card is intentionally excluded from stable pass/fail accounting.",
    experimental: true,
    section: "experimental",
    title: "PSD Import Placeholder",
    async run() {
      const verify = createVerifier();
      const header = new Uint8Array([0x38, 0x42, 0x50, 0x53, 0x00, 0x01]);
      const format = await detectFormat(header);
      verify.equal(format, "psd", "PSD header bytes should still detect as psd");

      return {
        assertions: verify.count,
        metrics: [
          ["detectFormat()", format],
          ["Status", "Experimental only; no stable fixture included"],
        ],
        views: [
          messageView(
            "Experimental note",
            "No PSD render fixture is shipped with the suite. This keeps the unstable parser visible without turning it into a release gate.",
          ),
        ],
      };
    },
  },
];
