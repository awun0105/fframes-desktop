// Dependency-free Chrome DevTools checks. Start preview:landing and Chrome on port 9226.
import { writeFileSync, mkdirSync } from "node:fs";
const out = "output/studio-motion-fix";
mkdirSync(out, { recursive: true });
const origin = "http://127.0.0.1:4174";
const targets = await (await fetch("http://127.0.0.1:9226/json/list")).json();
const target =
  targets.find(t => t.type === "page" && t.url.includes("4174")) ||
  targets.find(t => t.type === "page");
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map(),
  events = new Map(),
  errors = [];
ws.addEventListener("message", ({ data }) => {
  const m = JSON.parse(data);
  if (m.id) {
    const q = pending.get(m.id);
    pending.delete(m.id);
    m.error ? q.reject(m.error) : q.resolve(m.result);
  } else {
    events.get(m.method)?.(m.params);
    if (m.method === "Runtime.exceptionThrown")
      errors.push(m.params.exceptionDetails);
  }
});
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const n = ++id;
    pending.set(n, {
      resolve,
      reject: e => reject(new Error(method + ": " + JSON.stringify(e))),
    });
    ws.send(JSON.stringify({ id: n, method, params }));
  });
const ev = async expression => {
  const r = await send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (r.exceptionDetails) throw Error(JSON.stringify(r.exceptionDetails));
  return r.result.value;
};
let scriptsOff = false;
const frames = () =>
  scriptsOff
    ? new Promise(r => setTimeout(r, 80))
    : ev(
        "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r))))"
      );
const scroll = async y => {
  await ev(
    `document.documentElement.style.scrollBehavior='auto';scrollTo(0,${y})`
  );
  await frames();
};
const shot = async name => {
  const { data } = await send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: false,
  });
  writeFileSync(`${out}/${name}.png`, Buffer.from(data, "base64"));
};
const key = async (key, code = key) => {
  await send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key,
    code,
    text: key === "Enter" ? "\r" : undefined,
    windowsVirtualKeyCode: key === "Tab" ? 9 : 13,
  });
  await send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key,
    code,
    windowsVirtualKeyCode: key === "Tab" ? 9 : 13,
  });
  await frames();
};
const checks = [];
const check = (name, passed, evidence) =>
  checks.push({ name, passed: !!passed, evidence });
const storageKey = "fframes-studio-scroll-motion";
const pages = [
  {
    path: "/studio/",
    className: "story-motion",
    figure: ".workflow-figure",
    step: "#check",
    part: ".preview-plane",
  },
  {
    path: "/studio/how-it-works.html",
    className: "internals-motion",
    figure: ".transaction-figure",
    step: "#inspect",
    part: ".travelling-candidate",
  },
];
const load = async p => {
  await send("Page.navigate", {
    url: origin + p.path + "?motion-check=" + Date.now(),
  });
  await new Promise(r => setTimeout(r, 350));
  await ev("document.fonts.ready");
  await frames();
};
await send("Page.enable");
await send("Runtime.enable");
await send("Page.bringToFront");
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setDeviceMetricsOverride", {
  width: 1440,
  height: 900,
  deviceScaleFactor: 1,
  mobile: false,
});
await send("Emulation.setEmulatedMedia", { features: [] });
await load(pages[0]);
const nativePreference = await ev(
  `matchMedia('(prefers-reduced-motion: reduce)').matches`
);
await ev(`localStorage.removeItem('${storageKey}')`);
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
for (const [i, p] of pages.entries()) {
  await load(p);
  const initial = await ev(
    `({enabled:window.studioScrollMotion.enabled,classEnabled:document.documentElement.classList.contains('${p.className}'),visible:!document.querySelector('[data-motion-controls]').hidden,label:document.querySelector('[data-motion-controls] button').textContent,status:document.querySelector('[data-motion-controls] [role=status]').textContent})`
  );
  check(
    `${p.path}: reduced motion stays respected by default`,
    !initial.enabled &&
      !initial.classEnabled &&
      initial.visible &&
      initial.label === "Enable scroll motion",
    initial
  );
  await shot(`page-${i}-system-default`);
  await ev(`document.querySelector('[data-motion-controls] button').focus()`);
  await key("Enter");
  check(
    `${p.path}: keyboard opt-in overrides reduced motion and enables sticky layout`,
    await ev(
      `window.studioScrollMotion.enabled&&document.documentElement.classList.contains('${p.className}')&&getComputedStyle(document.querySelector('${p.figure}')).position==='sticky'&&document.querySelector('[data-motion-controls] button').getAttribute('aria-pressed')==='true'`
    )
  );
  const before = await ev(
    `getComputedStyle(document.querySelector('${p.part}')).transform`
  );
  const targetY = await ev(
    `document.querySelector('${p.step}').getBoundingClientRect().top+scrollY-120`
  );
  await scroll(targetY);
  const after = await ev(
    `getComputedStyle(document.querySelector('${p.part}')).transform`
  );
  check(
    `${p.path}: actual element moves with the OS still requesting reduced motion`,
    before !== after,
    { before, after }
  );
  await shot(`page-${i}-opted-in`);
  const style = await ev(
    `document.querySelector('${p.figure}').getAttribute('style')`
  );
  await new Promise(r => setTimeout(r, 180));
  check(
    `${p.path}: motion holds when scrolling stops`,
    style ===
      (await ev(`document.querySelector('${p.figure}').getAttribute('style')`))
  );
  await scroll(0);
  await scroll(targetY);
  check(
    `${p.path}: scroll reverses to the exact state`,
    style ===
      (await ev(`document.querySelector('${p.figure}').getAttribute('style')`))
  );
  await ev(`document.querySelector('[data-motion-controls] button').focus()`);
  await key("Enter");
  check(
    `${p.path}: pause resets to static overview`,
    await ev(
      `!window.studioScrollMotion.enabled&&!document.documentElement.classList.contains('${p.className}')&&document.querySelector('${p.figure}').getAttribute('style')===null`
    )
  );
  // Remove only this feature's saved preference before testing the next page's default.
  await ev(`localStorage.removeItem('${storageKey}')`);
}
await load(pages[1]);
await ev(`document.querySelector('[data-motion-controls] button').focus()`);
await key("Enter");
await load(pages[0]);
check(
  "Explicit opt-in persists across pages",
  await ev(
    `window.studioScrollMotion.enabled&&localStorage.getItem('${storageKey}')==='on'`
  )
);
await ev(`document.querySelector('[data-motion-controls] button').focus()`);
await key("Enter");
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await load(pages[1]);
check(
  "Explicit pause persists across pages and OS changes",
  await ev(
    `!window.studioScrollMotion.enabled&&localStorage.getItem('${storageKey}')==='off'`
  )
);
await ev(`localStorage.removeItem('${storageKey}')`);
await load(pages[0]);
await scroll(0);
const first = await ev(
  `getComputedStyle(document.querySelector('.preview-plane')).transform`
);
await scroll(400);
const early = await ev(
  `getComputedStyle(document.querySelector('.preview-plane')).transform`
);
check(
  "Overview layers now move within the first 400px of scrolling",
  first !== early,
  { first, early }
);
// Leave the local preview enabled by the same explicit opt-in visitors can choose.
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await load(pages[1]);
await ev(`document.querySelector('[data-motion-controls] button').focus()`);
await key("Enter");
await send("Emulation.setEmulatedMedia", { features: [] });
await load(pages[1]);
await scroll(0);
const report = {
  nativeReducedMotion: nativePreference,
  checkedAt: new Date().toISOString(),
  checks,
  passed: checks.every(c => c.passed),
  errors,
};
writeFileSync(`${out}/checks.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
ws.close();
if (!report.passed) process.exitCode = 1;
