// Export a social card using the approved logo, without changing its artwork.
// Requires Chrome with --remote-debugging-port=9226 (or CHROME_DEBUG_URL).
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const logo = readFileSync(
  `${root}/landing/studio/brand/fframes-studio-logo-blue-v1.png`
).toString("base64");
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#fafcff"/>
  <rect width="1200" height="8" fill="#2864f5"/>
  <circle cx="1200" cy="0" r="350" fill="#edf3ff"/>
  <circle cx="0" cy="630" r="240" fill="#eeeaff"/>
  <image href="data:image/png;base64,${logo}" x="50" y="60" width="1100" height="367"/>
  <g text-anchor="middle" font-family="Arial, sans-serif">
    <text x="600" y="440" font-size="48" letter-spacing="-1.5" fill="#15294b">A video studio.</text>
    <text x="600" y="500" font-size="48" letter-spacing="-1.5" fill="#2864f5">Built around your code.</text>
    <text x="600" y="565" font-size="22" fill="#586b85">Your code. Your agent. Your studio.</text>
  </g>
</svg>`;
mkdirSync(`${root}/output`, { recursive: true });
writeFileSync(`${root}/output/studio-social-preview.svg`, svg);
const debug = process.env.CHROME_DEBUG_URL || "http://127.0.0.1:9226";
const target = await (
  await fetch(`${debug}/json/new?about:blank`, { method: "PUT" })
).json();
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(resolve =>
  ws.addEventListener("open", resolve, { once: true })
);
let id = 0;
const pending = new Map();
ws.addEventListener("message", ({ data }) => {
  const message = JSON.parse(data);
  if (!message.id) return;
  const request = pending.get(message.id);
  pending.delete(message.id);
  message.error
    ? request.reject(message.error)
    : request.resolve(message.result);
});
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    pending.set(++id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
try {
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1200,
    height: 630,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await send("Page.navigate", {
    url: `data:text/html;base64,${Buffer.from(`<body style="margin:0"><img width="1200" height="630" style="display:block" src="data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}"></body>`).toString("base64")}`,
  });
  const ready = await send("Runtime.evaluate", {
    expression:
      "Promise.all([...document.images].map(i=>i.decode())).then(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))).then(()=>document.images.length===1&&document.images[0].naturalWidth===1200)",
    awaitPromise: true,
    returnByValue: true,
  });
  if (ready.exceptionDetails || !ready.result.value)
    throw new Error("Card assets did not decode");
  const { data } = await send("Page.captureScreenshot", {
    format: "png",
    clip: { x: 0, y: 0, width: 1200, height: 630, scale: 1 },
  });
  const output = `${root}/landing/studio/brand/studio-social-blue-v1.png`;
  writeFileSync(output, Buffer.from(data, "base64"));
  console.log(
    `Exported ${output}: 1200×630, ${Buffer.from(data, "base64").length} bytes`
  );
} finally {
  ws.close();
  await fetch(`${debug}/json/close/${target.id}`);
}
