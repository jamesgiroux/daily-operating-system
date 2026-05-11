#!/usr/bin/env node

const [, , laneArg] = process.argv;
const lane = laneArg && laneArg !== "--" ? laneArg : "stage8b";

console.error(
  `${lane} is a Stage 8b entry point. It is intentionally blocked until DOS-503 lands the Evaluation Evidence Contract and DOS-505 opens Stage 8b.`,
);
process.exit(2);
