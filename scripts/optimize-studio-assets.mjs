// Keep the shared brand source intact; three decimal places are far below a
// display pixel at the Studio wordmark's 96px rendered width.
import { readFileSync, writeFileSync } from "node:fs";
const source = readFileSync("landing/brand/fframes-wordmark.svg", "utf8");
const optimized = source.replace(/-?\d+\.\d+/g, value =>
  String(Number(Number(value).toFixed(3)))
);
writeFileSync("landing/studio/wordmark.svg", optimized);
console.log(
  `Studio wordmark: ${Buffer.byteLength(source)} → ${Buffer.byteLength(optimized)} bytes`
);
