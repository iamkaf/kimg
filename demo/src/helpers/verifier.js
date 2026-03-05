export function createVerifier() {
  return {
    count: 0,
    equal(actual, expected, message) {
      this.count += 1;
      if (actual !== expected) {
        throw new Error(`${message}. Expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}.`);
      }
    },
    ok(condition, message) {
      this.count += 1;
      if (!condition) {
        throw new Error(message);
      }
    },
  };
}
