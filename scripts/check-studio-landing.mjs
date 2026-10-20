// Dependency-free Chrome DevTools checks. Start preview:landing and Chrome on port 9226.
import { writeFileSync, mkdirSync } from "node:fs";
const out = "output/studio-refinement";
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
const state = () =>
  ev(
    `({style:document.querySelector('.workflow-figure').getAttribute('style'),label:document.querySelector('[data-state-label]').textContent,titleBefore:getComputedStyle(document.querySelector('.title-before')).opacity,titleAfter:getComputedStyle(document.querySelector('.title-after')).opacity,build:getComputedStyle(document.querySelector('.gate-build-label')).opacity,inspect:getComputedStyle(document.querySelector('.gate-inspect-label')).opacity})`
  );
const checks = [];
const check = (name, passed, evidence) =>
  checks.push({ name, passed: !!passed, evidence });
const goStep = async n => {
  const y = await ev(
    `(()=>{const f=document.querySelector('.workflow-figure'),s=document.querySelectorAll('[data-step]')[${n}];return s.getBoundingClientRect().top+scrollY-(innerWidth<=760?f.offsetHeight+90:120)})()`
  );
  await scroll(y);
  return y;
};
const size = async (width, height) => {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
};
await send("Page.enable");
await send("Page.bringToFront");
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
await send("Runtime.enable");
await send("Performance.enable");
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await size(1440, 1000);
await send("Page.navigate", { url: origin + "/studio/" });
await new Promise(r => setTimeout(r, 700));
await ev("document.fonts.ready");
await frames();
// Test the OS default independently of a visitor's saved motion choice.
await ev(`localStorage.removeItem('fframes-studio-scroll-motion')`);
await send("Page.reload");
await new Promise(r => setTimeout(r, 350));
await frames();
check(
  "Scripts initialize",
  await ev(`document.documentElement.classList.contains('story-motion')`)
);
const assets = await ev(
  `({broken:[...document.images].filter(i=>!i.complete||!i.naturalWidth).map(i=>i.src),badAnchors:[...document.querySelectorAll('a[href^="#"]')].filter(a=>!document.getElementById(a.hash.slice(1))).map(a=>a.hash),h1:document.querySelectorAll('h1').length,illustrationLabel:document.querySelector('.illustration-label').textContent})`
);
check(
  "Assets, landmarks and local anchors",
  !assets.broken.length && !assets.badAnchors.length && assets.h1 === 1,
  assets
);
await shot("desktop-hero");
const states = [];
for (let n = 0; n < 4; n++) {
  await goStep(n);
  states.push(await state());
  await shot(`desktop-state-${n + 1}`);
}
const finalBounds = await ev(
  `(()=>{const f=document.querySelector('.workflow-figure').getBoundingClientRect();return {top:f.top,bottom:f.bottom,height:innerHeight}})()`
);
check(
  "Final desktop state remains fully visible",
  finalBounds.top >= 76 && finalBounds.bottom <= finalBounds.height,
  finalBounds
);
check(
  "Preview stays old until acceptance",
  states
    .slice(0, 3)
    .every(s => s.titleBefore === "1" && s.titleAfter === "0") &&
    states[3].titleBefore === "0" &&
    states[3].titleAfter === "1",
  states
);
check(
  "Build and inspect precede updated preview",
  states[1].build === "0" &&
    states[2].build === "1" &&
    states[3].inspect === "1"
);
const start = await goStep(1),
  end = await goStep(2);
await scroll((start + end) / 2);
const middle = await state();
await new Promise(r => setTimeout(r, 250));
check(
  "Stopping scroll holds the state",
  JSON.stringify(middle) === JSON.stringify(await state())
);
await goStep(3);
await scroll((start + end) / 2);
check(
  "Reverse scroll restores the exact intermediate state",
  JSON.stringify(middle) === JSON.stringify(await state())
);
await goStep(1);
check(
  "Reverse restores the draft state",
  JSON.stringify(states[1]) === JSON.stringify(await state())
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
    `({width:innerWidth,client:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth,overflow:[...document.querySelectorAll('header *,main *,footer *')].filter(e=>{const r=e.getBoundingClientRect();return r.width>0&&(r.right>document.documentElement.clientWidth+1||r.left<-1)}).map(e=>e.tagName+'.'+(e.className.baseVal??e.className)).slice(0,12)})`
  );
  check(
    `Layout ${width}×${height} has no horizontal overflow`,
    layout.scroll === layout.client,
    layout
  );
  await shot(`layout-${width}-hero`);
  await goStep(1);
  await shot(`layout-${width}-edit`);
  const first = await state();
  await goStep(3);
  await shot(`layout-${width}-preview`);
  await goStep(1);
  check(
    `Scroll reverses at ${width}px`,
    JSON.stringify(first) === JSON.stringify(await state())
  );
  const bounds = await ev(
    `(()=>{const f=document.querySelector('.workflow-figure').getBoundingClientRect(),nav=document.querySelector('.step-nav').getBoundingClientRect();return {top:f.top,bottom:f.bottom,navBottom:nav.bottom,height:innerHeight}})()`
  );
  check(
    `Figure controls fit ${width}px viewport`,
    bounds.top >= 60 && bounds.navBottom < height,
    bounds
  );
  if (width === 390) {
    await scroll(
      await ev(
        `document.querySelector('#install').getBoundingClientRect().top+scrollY-80`
      )
    );
    await shot("mobile-install");
  }
}
await size(1440, 1000);
await scroll(0);
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await frames();
const reducedA = await state();
await goStep(3);
const reducedB = await state();
check(
  "Reduced motion is static and fully explained",
  !(await ev(`document.documentElement.classList.contains('story-motion')`)) &&
    JSON.stringify(reducedA) === JSON.stringify(reducedB)
);
await scroll(0);
await shot("reduced-motion");
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await frames();
check(
  "Live preference change re-enables motion",
  await ev(`document.documentElement.classList.contains('story-motion')`)
);
console.log("Layout and motion checks complete.");
// Real keyboard events, including the skip link and every workflow anchor.
await send("Page.navigate", { url: origin + "/studio/?check=keyboard" });
await new Promise(r => setTimeout(r, 350));
await frames();
await key("Tab");
const skip = await ev(
  `({text:document.activeElement.textContent,focus:document.activeElement.matches(':focus-visible')})`
);
await key("Enter");
check(
  "Keyboard skip link works and has a focus ring",
  skip.focus && (await ev(`location.hash==='#main'`)),
  skip
);
const focusStops = [];
for (let n = 0; n < 38; n++) {
  await key("Tab");
  focusStops.push(
    await ev(
      `({tag:document.activeElement.tagName,text:document.activeElement.textContent.trim().slice(0,40),href:document.activeElement.getAttribute('href'),visible:document.activeElement.getBoundingClientRect().width>0,outline:getComputedStyle(document.activeElement).outlineStyle})`
    )
  );
}
check(
  "Keyboard reaches all four states and copy controls",
  focusStops.filter(x => x.tag === "BUTTON").length >= 2 &&
    ["#select", "#edit", "#check", "#preview"].every(h =>
      focusStops.some(x => x.href === h)
    ),
  focusStops
);
for (const [n, name] of ["select", "edit", "check", "preview"].entries()) {
  await ev(`document.querySelector('.step-nav a[href="#${name}"]').focus()`);
  await key("Enter");
  check(`Keyboard activates ${name}`, await ev(`location.hash==='#${name}'`));
}
await send("Browser.grantPermissions", {
  origin,
  permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"],
});
await send("Page.bringToFront");
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
for (let n = 0; n < 2; n++) {
  await ev(`document.querySelectorAll('.copy')[${n}].focus()`);
  await key("Enter");
  await new Promise(r => setTimeout(r, 100));
  const copy = await ev(
    `(async()=>({clipboard:await navigator.clipboard.readText(),expected:document.querySelectorAll('.command pre')[${n}].textContent.replace(/^PS> /,'').replace(/^\\$ /,''),feedback:document.querySelectorAll('.card')[${n}].querySelector('[role="status"]')?.textContent,active:document.activeElement.outerHTML.slice(0,120)}))()`
  );
  check(
    `Install copy ${n + 1} copies exact command`,
    copy.clipboard === copy.expected && copy.feedback === "Command copied.",
    copy
  );
}
await ev(
  `navigator.clipboard.writeText=async()=>{throw new Error('test denial')};document.querySelector('.copy').click()`
);
await frames();
check(
  "Clipboard denial gives visible guidance",
  await ev(
    `document.querySelector('.copy-feedback').textContent.includes('Select the command')`
  )
);
console.log("Keyboard and clipboard checks complete.");
// No JS: reload really disables page scripts; native anchors remain usable.
scriptsOff = true;
await send("Emulation.setScriptExecutionDisabled", { value: true });
await send("Page.navigate", { url: origin + "/studio/?check=nojs" });
await new Promise(r => setTimeout(r, 350));
await scroll(0);
const nojs = await ev(
  `({motion:document.documentElement.classList.contains('story-motion'),steps:document.querySelectorAll('[data-step]').length,title:getComputedStyle(document.querySelector('.title-before')).opacity,copyHidden:getComputedStyle(document.querySelector('.copy')).display,commands:[...document.querySelectorAll('.command code')].map(c=>c.textContent)})`
);
check(
  "No JS retains full story and install commands; hides inactive copy buttons",
  !nojs.motion &&
    nojs.steps === 4 &&
    nojs.copyHidden === "none" &&
    nojs.commands.length === 2,
  nojs
);
await shot("no-js-desktop");
await size(390, 844);
await scroll(0);
await shot("no-js-mobile");
await ev(`document.querySelector('a[href="#install"]').focus()`);
await key("Enter");
check("No JS install anchor works", await ev(`location.hash==='#install'`));
await send("Emulation.setScriptExecutionDisabled", { value: false });
scriptsOff = false;
await send("Page.reload");
await new Promise(r => setTimeout(r, 250));
await size(1440, 1000);
await send("Page.navigate", { url: origin + "/studio/" });
await new Promise(r => setTimeout(r, 250));
await frames();
console.log("Fallback checks complete.");
// Trace an actual frame-by-frame scrub. Durations describe this local Chrome run only.
const metricsBefore = await send("Performance.getMetrics");
await send("Tracing.start", {
  categories: "devtools.timeline,blink.user_timing",
  transferMode: "ReturnAsStream",
});
const perf = await ev(
  `(async()=>{const intervals=[],longtasks=[];let previous=performance.now();const observer=new PerformanceObserver(l=>longtasks.push(...l.getEntries().map(e=>e.duration)));observer.observe({type:'longtask',buffered:false});document.documentElement.style.scrollBehavior='auto';const start=document.querySelector('#select').getBoundingClientRect().top+scrollY-120,end=document.querySelector('#preview').getBoundingClientRect().top+scrollY-120;for(let n=0;n<120;n++){await new Promise(requestAnimationFrame);const now=performance.now();intervals.push(now-previous);previous=now;scrollTo(0,start+(end-start)*(n/119))}observer.disconnect();return {intervals,longtasks}})()`
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
  "/index.html",
  "/brand/fframes-mark.svg",
];
for (const link of localLinks) {
  const r = await fetch(origin + link);
  check(`Local destination ${link}`, r.ok, r.status);
}
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
