// Dependency-free Chrome DevTools checks. Start preview:landing and Chrome on port 9226.
import { writeFileSync, mkdirSync } from "node:fs";
const out = "output/studio-how-refinement";
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
const page = origin + "/studio/how-it-works.html";
const checks = [];
const check = (name, passed, evidence) =>
  checks.push({ name, passed: !!passed, evidence });
const size = async (width, height) => {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
};
const snapshot = () =>
  ev(
    `({figure:document.querySelector('.transaction-figure').getAttribute('style'),planes:[...document.querySelectorAll('[data-plane]')].map(p=>p.getAttribute('style')),label:document.querySelector('[data-gate-label]').textContent})`
  );
const goStep = async n => {
  const y = await ev(
    `(()=>{const f=document.querySelector('.transaction-figure'),s=document.querySelectorAll('[data-gate-step]')[${n}];return s.getBoundingClientRect().top+scrollY-(innerWidth<=760?f.offsetHeight+90:120)})()`
  );
  await scroll(y);
  return y;
};
const load = async tag => {
  await send("Page.navigate", { url: page + "?check=" + tag });
  await new Promise(r => setTimeout(r, 350));
  if (!scriptsOff) {
    await ev("document.fonts.ready");
    await frames();
  }
};
await send("Page.enable");
await send("Runtime.enable");
await send("Page.bringToFront");
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
await send("Performance.enable");
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await size(1440, 1000);
await load("initial");
// Test the OS default independently of a visitor's saved motion choice.
await ev(`localStorage.removeItem('fframes-studio-scroll-motion')`);
await load("default-motion");
check(
  "Motion initializes",
  await ev(`document.documentElement.classList.contains('internals-motion')`)
);
const doc = await ev(
  `({h1:document.querySelectorAll('h1').length,main:document.querySelectorAll('main').length,broken:[...document.images].filter(i=>!i.complete||!i.naturalWidth).map(i=>i.src),missing:[...document.querySelectorAll('a[href^="#"]')].filter(a=>!document.getElementById(a.hash.slice(1))).map(a=>a.hash),unlabelled:[...document.querySelectorAll('svg[role="img"]')].filter(s=>!s.getAttribute('aria-labelledby')?.split(' ').every(id=>document.getElementById(id))).length,placeholders:document.body.textContent.includes('GATES'),tools:document.querySelectorAll('.tool-list li').length})`
);
check(
  "Assets, landmarks, nine tools, diagrams and anchors",
  doc.h1 === 1 &&
    doc.main === 1 &&
    !doc.broken.length &&
    !doc.missing.length &&
    !doc.unlabelled &&
    !doc.placeholders &&
    doc.tools === 9,
  doc
);
await shot("desktop-hero");
const states = [];
for (let n = 0; n < 5; n++) {
  await goStep(n);
  states.push(await snapshot());
  await shot(`desktop-stage-${n + 1}`);
}
check(
  "Candidate changes at each of five scroll states",
  new Set(states.map(s => s.figure)).size === 5,
  states
);
check(
  "Only final state shows accepted result",
  states.slice(0, 4).every(s => !s.figure.includes("--accepted-alpha: 1;")) &&
    states[4].figure.includes("--accepted-alpha: 1;")
);
check(
  "All stages complete at acceptance",
  states[4].planes.every(p => p.includes("--passed: 1;"))
);
const lastBounds = await ev(
  `(()=>{const r=document.querySelector('.transaction-figure').getBoundingClientRect();return{top:r.top,bottom:r.bottom,height:innerHeight}})()`
);
check(
  "Final figure fits desktop viewport",
  lastBounds.top >= 76 && lastBounds.bottom <= lastBounds.height,
  lastBounds
);
const a = await goStep(1),
  b = await goStep(2);
await scroll((a + b) / 2);
const midway = await snapshot();
await new Promise(r => setTimeout(r, 200));
check(
  "Stopped scroll holds exact state",
  JSON.stringify(midway) === JSON.stringify(await snapshot())
);
await goStep(4);
await scroll((a + b) / 2);
check(
  "Reverse restores intermediate transforms exactly",
  JSON.stringify(midway) === JSON.stringify(await snapshot())
);
const readinessY = await ev(
  `document.querySelector('[data-readiness]').getBoundingClientRect().top+scrollY`
);
await scroll(readinessY - 700);
const readyBefore = await ev(
  `document.querySelector('[data-readiness]').getAttribute('style')`
);
await scroll(
  await ev(
    `document.querySelector('.readiness-figure').getBoundingClientRect().top+scrollY-110`
  )
);
const readyAfter = await ev(
  `document.querySelector('[data-readiness]').getAttribute('style')`
);
await shot("desktop-readiness");
check(
  "Readiness fills and advances individual lanes",
  readyBefore !== readyAfter && readyAfter.includes("--handoff: 1;"),
  { readyBefore, readyAfter }
);
await scroll(readinessY - 700);
check(
  "Readiness motion reverses",
  readyBefore ===
    (await ev(
      `document.querySelector('[data-readiness]').getAttribute('style')`
    ))
);
for (const [width, height] of [
  [1280, 800],
  [768, 1024],
  [390, 844],
  [320, 640],
]) {
  await size(width, height);
  await scroll(0);
  const layout = await ev(
    `({client:document.documentElement.clientWidth,width:innerWidth,scroll:document.documentElement.scrollWidth})`
  );
  check(
    `No page overflow at ${width}×${height}`,
    layout.client === layout.scroll,
    layout
  );
  await shot(`layout-${width}-hero`);
  await goStep(2);
  const state = await snapshot();
  await shot(`layout-${width}-inspect`);
  await goStep(4);
  await shot(`layout-${width}-accepted`);
  const box = await ev(
    `(()=>{const f=document.querySelector('.transaction-figure').getBoundingClientRect();return{top:f.top,bottom:f.bottom,height:innerHeight}})()`
  );
  check(
    `Accepted figure fits at ${width}px`,
    box.top >= 60 && box.bottom <= box.height,
    box
  );
  await goStep(2);
  check(
    `Exact reversal at ${width}px`,
    JSON.stringify(state) === JSON.stringify(await snapshot())
  );
  if (width === 390) {
    await scroll(
      await ev(
        `document.querySelector('#tools').getBoundingClientRect().top+scrollY-85`
      )
    );
    await shot("mobile-tools");
    await scroll(
      await ev(
        `document.querySelector('.guide-install').getBoundingClientRect().top+scrollY-85`
      )
    );
    await shot("mobile-install");
  }
}
await size(390, 844);
await ev(`document.querySelector('.diagram-scroll').focus()`);
const horizontalBefore = await ev(
  `document.querySelector('.diagram-scroll').scrollLeft`
);
for (let i = 0; i < 8; i++) {
  await send("Input.dispatchKeyEvent", {
    type: "keyDown",
    key: "ArrowRight",
    code: "ArrowRight",
    windowsVirtualKeyCode: 39,
  });
  await send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key: "ArrowRight",
    code: "ArrowRight",
    windowsVirtualKeyCode: 39,
  });
}
await new Promise(r => setTimeout(r, 200));
check(
  "Wide diagram scrolls with keyboard",
  (await ev(`document.querySelector('.diagram-scroll').scrollLeft`)) >
    horizontalBefore
);
await shot("mobile-process-map");
await size(1440, 1000);
await load("keyboard");
await key("Tab");
const skip = await ev(
  `({text:document.activeElement.textContent,ring:document.activeElement.matches(':focus-visible')})`
);
await key("Enter");
check(
  "Keyboard skip link works",
  skip.ring && (await ev(`location.hash==='#main'`)),
  skip
);
const focus = [];
for (let i = 0; i < 32; i++) {
  await key("Tab");
  focus.push(
    await ev(
      `({href:document.activeElement.getAttribute('href'),region:document.activeElement.className})`
    )
  );
}
check(
  "Tab reaches all five stage links and diagram regions",
  ["#capture", "#build", "#inspect", "#review", "#apply"].every(h =>
    focus.some(x => x.href === h)
  ) && focus.some(x => x.region === "diagram-scroll"),
  focus
);
for (const name of ["capture", "build", "inspect", "review", "apply"]) {
  await ev(`document.querySelector('.gate-nav a[href="#${name}"]').focus()`);
  await key("Enter");
  check(`Keyboard activates ${name}`, await ev(`location.hash==='#${name}'`));
}
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await frames();
await goStep(0);
const reducedBefore = await snapshot();
await goStep(4);
check(
  "Reduced motion remains static",
  JSON.stringify(reducedBefore) === JSON.stringify(await snapshot()) &&
    !(await ev(
      `document.documentElement.classList.contains('internals-motion')`
    ))
);
await scroll(0);
await shot("reduced-motion");
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await frames();
check(
  "Live preference change restores scrub",
  await ev(`document.documentElement.classList.contains('internals-motion')`)
);
console.log("Layout, motion and keyboard checks completed.");
scriptsOff = true;
await send("Emulation.setScriptExecutionDisabled", { value: true });
await load("nojs");
await scroll(0);
const staticDoc = await ev(
  `({motion:document.documentElement.classList.contains('internals-motion'),stages:document.querySelectorAll('[data-gate-step]').length,tools:document.querySelectorAll('.tool-list li').length,accepted:getComputedStyle(document.querySelector('.accepted-result')).opacity,install:document.querySelector('.guide-install a').getAttribute('href')})`
);
check(
  "No JavaScript preserves complete story and install destination",
  !staticDoc.motion &&
    staticDoc.stages === 5 &&
    staticDoc.tools === 9 &&
    staticDoc.accepted === "1" &&
    staticDoc.install === "index.html#install",
  staticDoc
);
await shot("no-js-desktop");
await size(390, 844);
await scroll(0);
await shot("no-js-mobile");
await ev(`document.querySelector('a[href="#transaction"]').focus()`);
await key("Enter");
check("No JS native anchor works", await ev(`location.hash==='#transaction'`));
await send("Emulation.setScriptExecutionDisabled", { value: false });
scriptsOff = false;
await load("performance");
await size(1440, 1000);
// Trace an actual frame-by-frame scrub. Durations describe this local Chrome run only.
const metricsBefore = await send("Performance.getMetrics");
await send("Tracing.start", {
  categories: "devtools.timeline,blink.user_timing",
  transferMode: "ReturnAsStream",
});
const perf = await ev(
  `(async()=>{const intervals=[],longtasks=[];let previous=performance.now();const observer=new PerformanceObserver(l=>longtasks.push(...l.getEntries().map(e=>e.duration)));observer.observe({type:'longtask',buffered:false});document.documentElement.style.scrollBehavior='auto';const start=document.querySelector('#capture').getBoundingClientRect().top+scrollY-120,end=document.querySelector('#apply').getBoundingClientRect().top+scrollY-120;for(let n=0;n<120;n++){await new Promise(requestAnimationFrame);const now=performance.now();intervals.push(now-previous);previous=now;scrollTo(0,start+(end-start)*(n/119))}observer.disconnect();return {intervals,longtasks}})()`
);
const tracingDone = new Promise(r => events.set("Tracing.tracingComplete", r));
await send("Tracing.end");
const { stream } = await tracingDone;
let trace = "";
for (;;) {
  const r = await send("IO.read", { handle: stream });
  trace += r.data;
  if (r.eof) break;
}
await send("IO.close", { handle: stream });
writeFileSync(`${out}/scroll-trace.json`, trace);
const metricsAfter = await send("Performance.getMetrics");
const metric = name =>
  metricsAfter.metrics.find(x => x.name === name).value -
  metricsBefore.metrics.find(x => x.name === name).value;
const traceEvents = JSON.parse(trace).traceEvents;
const duration = name =>
  traceEvents
    .filter(e => e.name === name && e.ph === "X")
    .reduce((sum, e) => sum + (e.dur || 0) / 1000, 0);
const sorted = perf.intervals.slice(1).sort((a, b) => a - b);
const performanceResult = {
  frames: 120,
  rafMedianMs: sorted[Math.floor(sorted.length * 0.5)],
  rafP95Ms: sorted[Math.floor(sorted.length * 0.95)],
  longTasks: perf.longtasks,
  scriptMs: metric("ScriptDuration") * 1000,
  layoutMs: duration("Layout"),
  paintMs: duration("Paint"),
  styleMs: duration("UpdateLayoutTree"),
  note: "Local headless Chrome; CPU trace and rAF cadence, not a real-device GPU or battery benchmark.",
};
check(
  "Scrub introduces no long tasks over 50ms",
  !perf.longtasks.some(t => t > 50),
  performanceResult
);
check("No uncaught browser exceptions", errors.length === 0, errors);
const localLinks = [
  "/studio/how-it-works.html",
  "/studio/index.html",
  "/index.html",
  "/brand/fframes-mark.svg",
];
for (const link of localLinks) {
  const r = await fetch(origin + link);
  check(`Local destination ${link}`, r.ok, r.status);
}
await ev(`document.querySelector('.guide-install a').focus()`);
await key("Enter");
await new Promise(r => setTimeout(r, 350));
check(
  "Install control opens the existing homepage install section",
  await ev(
    `location.pathname==='/studio/index.html' && location.hash==='#install' && !!document.querySelector('#install')`
  )
);
const report = {
  checkedAt: new Date().toISOString(),
  browser: await send("Browser.getVersion"),
  checks,
  performance: performanceResult,
  passed: checks.every(c => c.passed),
  limitations: [
    "No native UI interaction replay: launched Studio exposes no window in this session.",
    "Original Studio recordings unavailable in checkout.",
    "Rust validation tests use controlled fixtures; no signed-in agent task run for this change.",
    "Safari, Firefox, real phones, assistive technology and unfamiliar-viewer comprehension remain untested.",
    "External release download and installer execution not performed; existing install URLs preserved.",
  ],
};
writeFileSync(`${out}/checks.json`, JSON.stringify(report, null, 2));
console.log(
  JSON.stringify(
    {
      passed: report.passed,
      checks: checks.length,
      failures: checks.filter(x => !x.passed),
      performance: performanceResult,
    },
    null,
    2
  )
);
ws.close();
if (!report.passed) process.exitCode = 1;
