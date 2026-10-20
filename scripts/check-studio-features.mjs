// Product-wide homepage explanations: reversible SVG motion and static fallbacks.
import { mkdirSync, writeFileSync } from "node:fs";
const out = "output/studio-features";
mkdirSync(out, { recursive: true });
const targets = await (await fetch("http://127.0.0.1:9226/json/list")).json();
const ws = new WebSocket(
  targets.find(t => t.type === "page").webSocketDebuggerUrl
);
await new Promise(r => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map(),
  errors = [],
  checks = [];
ws.addEventListener("message", ({ data }) => {
  const m = JSON.parse(data);
  if (m.id) {
    const p = pending.get(m.id);
    pending.delete(m.id);
    m.error ? p.reject(m.error) : p.resolve(m.result);
  } else if (m.method === "Runtime.exceptionThrown")
    errors.push(m.params.exceptionDetails);
});
const send = (method, params = {}) =>
  new Promise((resolve, reject) => {
    const n = ++id;
    pending.set(n, { resolve, reject });
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
const frames = () =>
  ev(
    "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r))))"
  );
const scroll = async y => {
  await ev(
    `document.documentElement.style.scrollBehavior='auto';scrollTo(0,${y})`
  );
  await frames();
};
const shot = async name => {
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(`${out}/${name}.png`, Buffer.from(data, "base64"));
};
const check = (name, passed, evidence) =>
  checks.push({ name, passed: !!passed, evidence });
const load = async () => {
  await send("Page.navigate", {
    url: "http://127.0.0.1:4174/studio/?product-check",
  });
  await new Promise(r => setTimeout(r, 500));
  await ev("document.fonts.ready");
  await frames();
};
await send("Page.enable");
await send("Runtime.enable");
await send("Page.bringToFront");
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await load();
await ev("localStorage.removeItem('fframes-studio-scroll-motion')");
await load();
const parts = {
  project: ".pf-output",
  timing: ".pf-clip-c",
  sdk: ".pf-tool-two",
  export: ".pf-render-bar",
};
check(
  "Four distinct product explanations",
  await ev("document.querySelectorAll('[data-product-figure]').length===4")
);
for (const width of [1440, 768, 390, 320]) {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height: width === 1440 ? 1000 : 844,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
  for (const [name, part] of Object.entries(parts)) {
    const selector = `[data-product-figure="${name}"]`;
    const position = await ev(
      `document.querySelector('${selector}').getBoundingClientRect().top+scrollY`
    );
    const snapshot = () =>
      ev(
        `(()=>{const el=document.querySelector('${selector}'),s=getComputedStyle(el.querySelector('${part}'));return {style:el.getAttribute('style'),transform:s.transform,opacity:s.opacity}})()`
      );
    await scroll(position - (await ev("innerHeight*.86")));
    const before = await snapshot();
    const y = position - (await ev("innerHeight*.15"));
    await scroll(y);
    const middle = await snapshot();
    check(
      `${name}: actual element changes at ${width}`,
      before.transform !== middle.transform ||
        before.opacity !== middle.opacity,
      { before, middle }
    );
    await new Promise(r => setTimeout(r, 100));
    check(
      `${name}: stops exactly at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await snapshot())
    );
    await scroll(position + 600);
    await scroll(y);
    check(
      `${name}: reverses exactly at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await snapshot())
    );
    check(
      `${name}: no page overflow at ${width}`,
      await ev(
        "document.documentElement.scrollWidth===document.documentElement.clientWidth"
      )
    );
    await scroll(position - 100);
    await shot(`${name}-${width}`);
  }
}
await ev(
  "document.querySelector('[data-product-figure] .product-figure-scroll').focus()"
);
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
for (const type of ["keyDown", "keyUp"])
  await send("Input.dispatchKeyEvent", {
    type,
    key: "ArrowRight",
    code: "ArrowRight",
    windowsVirtualKeyCode: 39,
  });
await new Promise(r => setTimeout(r, 150));
check(
  "Figures support keyboard scrolling and visible focus",
  await ev(
    "document.activeElement.matches('.product-figure-scroll:focus-visible')&&document.activeElement.scrollLeft>0"
  )
);
const perf =
  await ev(`(async()=>{const tasks=[];const o=new PerformanceObserver(l=>tasks.push(...l.getEntries().map(e=>e.duration)));o.observe({type:'longtask',buffered:false});
for(const f of document.querySelectorAll('[data-product-figure]')){const top=f.getBoundingClientRect().top+scrollY;for(let n=0;n<45;n++){await new Promise(requestAnimationFrame);scrollTo(0,top-innerHeight*.9+(f.offsetHeight+innerHeight*.3)*n/44)}}await new Promise(requestAnimationFrame);o.disconnect();return {frames:180,longTasks:tasks}})()`);
check(
  "Four product figures scrub without long tasks over 50ms",
  perf.longTasks.length === 0,
  perf
);
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await frames();
check(
  "Reduced motion shows static complete figures",
  await ev(
    "!document.documentElement.classList.contains('story-motion')&&[...document.querySelectorAll('[data-product-figure]')].every(e=>!e.hasAttribute('style'))"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: true });
await send("Page.reload");
await new Promise(r => setTimeout(r, 500));
check(
  "Four figures remain readable without JavaScript",
  await ev(
    "document.querySelectorAll('[data-product-figure] svg title').length===4&&getComputedStyle(document.querySelector('.pf-render-bar')).transform==='none'"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", { features: [] });
check("No browser exceptions", errors.length === 0, errors);
writeFileSync(
  `${out}/checks.json`,
  JSON.stringify({ checks, passed: checks.every(c => c.passed), perf }, null, 2)
);
console.log(
  JSON.stringify(
    { checks: checks.length, failures: checks.filter(c => !c.passed), perf },
    null,
    2
  )
);
ws.close();
if (checks.some(c => !c.passed)) process.exitCode = 1;
