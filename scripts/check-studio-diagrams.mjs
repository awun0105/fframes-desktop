// Geometry and narrative regressions: actual SVG coordinates, not style strings.
import { mkdirSync, writeFileSync } from "node:fs";
const out = "output/studio-diagrams";
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
const check = (name, passed, evidence) =>
  checks.push({ name, passed: !!passed, evidence });
const shot = async name => {
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(`${out}/${name}.png`, Buffer.from(data, "base64"));
};
const load = async () => {
  await send("Page.navigate", {
    url: "http://127.0.0.1:4174/studio/how-it-works.html?geometry",
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
const names = [
  "process",
  "capture",
  "review",
  "containment",
  "timing",
  "export",
];
for (const width of [1440, 768, 390, 320]) {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height: width === 1440 ? 1000 : 844,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
  for (const name of names) {
    const selector = `[data-explanation="${name}"]`;
    const top = await ev(
      `document.querySelector('${selector}').getBoundingClientRect().top+scrollY`
    );
    const state = () =>
      ev(
        `({path:getComputedStyle(document.querySelector('${selector} .ex-step-c')||document.querySelector('${selector} .ex-step-b')).strokeDashoffset,style:document.querySelector('${selector}').getAttribute('style')})`
      );
    await scroll(top - (await ev("innerHeight*.87")));
    const before = await state();
    const y = top - (await ev("innerHeight*.1"));
    await scroll(y);
    const middle = await state();
    check(
      `${name}: directed path advances at ${width}`,
      before.path !== middle.path,
      { before, middle }
    );
    await new Promise(r => setTimeout(r, 80));
    check(
      `${name}: stopped state holds at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await state())
    );
    await scroll(top + 600);
    await scroll(y);
    check(
      `${name}: reverse restores exact state at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await state())
    );
    check(
      `${name}: no page overflow at ${width}`,
      await ev(
        "document.documentElement.scrollWidth===document.documentElement.clientWidth"
      )
    );
    await scroll(top - 100);
    await shot(`${name}-${width}`);
  }
  const failures = await ev(
    `(()=>{const bad=[];for(const node of document.querySelectorAll('[data-ex-node], [data-support-node]')){const r=node.querySelector('rect').getBBox();for(const t of node.querySelectorAll('text')){const b=t.getBBox();if(b.x<r.x+5||b.x+b.width>r.x+r.width-5||b.y<r.y+5||b.y+b.height>r.y+r.height-5)bad.push({node:node.dataset.exNode||node.dataset.supportNode,text:t.textContent,box:{x:b.x,y:b.y,width:b.width,height:b.height},rect:{x:r.x,y:r.y,width:r.width,height:r.height}})}}return bad})()`
  );
  check(
    `All node labels fit their boxes at ${width}`,
    failures.length === 0,
    failures
  );
  for (let i = 0; i < 5; i++) {
    const y = await ev(
      `(()=>{const f=document.querySelector('.transaction-figure'),s=document.querySelectorAll('[data-gate-step]')[${i}];return s.getBoundingClientRect().top+scrollY-(innerWidth<=760?f.offsetHeight+90:120)})()`
    );
    await scroll(y);
    const distance = await ev(
      `(()=>{const c=document.querySelector('.travelling-candidate'),p=document.querySelectorAll('[data-plane]')[${i}].querySelector('circle');const a=new DOMPoint(305,436).matrixTransform(c.getScreenCTM());const b=new DOMPoint(p.cx.baseVal.value,p.cy.baseVal.value).matrixTransform(p.getScreenCTM());return Math.hypot(a.x-b.x,a.y-b.y)})()`
    );
    check(
      `Candidate tip aligns with gate ${i + 1} at ${width}`,
      distance < 1,
      distance
    );
  }
}
await send("Emulation.setDeviceMetricsOverride", {
  width: 1440,
  height: 1000,
  deviceScaleFactor: 1,
  mobile: false,
});
await frames();
await scroll(
  await ev(
    "document.querySelector('.readiness-figure').getBoundingClientRect().top+scrollY-110"
  )
);
const handoff = await ev(
  "(()=>{const a=new DOMPoint(152,82).matrixTransform(document.querySelector('.readiness-marker').getScreenCTM());const b=new DOMPoint(864,82).matrixTransform(document.querySelector('.handoff-marker').getScreenCTM());return Math.hypot(a.x-b.x,a.y-b.y)})()"
);
check(
  "Handoff arrow and final playhead share one position",
  handoff < 1,
  handoff
);
await shot("readiness-aligned");
for (const name of ["context", "broker", "recovery"]) {
  const top = await ev(
    `document.querySelector('.support-${name}').getBoundingClientRect().top+scrollY`
  );
  await scroll(top - 900);
  const before = await ev(
    `getComputedStyle(document.querySelector('.support-${name} .support-c')).transform`
  );
  await scroll(top - 100);
  const after = await ev(
    `getComputedStyle(document.querySelector('.support-${name} .support-c')).transform`
  );
  check(
    `${name}: connected nodes do not drift from fixed connectors`,
    before === "none" && after === "none",
    { before, after }
  );
  await shot(`anchored-${name}`);
}
await send("Emulation.setDeviceMetricsOverride", {
  width: 390,
  height: 844,
  deviceScaleFactor: 1,
  mobile: false,
});
await frames();
await ev(
  "document.querySelector('[data-explanation] .diagram-scroll').focus()"
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
  "New diagrams support keyboard scrolling and visible focus",
  await ev(
    "document.activeElement.matches('.diagram-scroll:focus-visible')&&document.activeElement.scrollLeft>0"
  )
);
const perf = await ev(
  `(async()=>{const tasks=[];const o=new PerformanceObserver(l=>tasks.push(...l.getEntries().map(e=>e.duration)));o.observe({type:'longtask',buffered:false});for(const f of document.querySelectorAll('[data-explanation]')){const top=f.getBoundingClientRect().top+scrollY;for(let n=0;n<45;n++){await new Promise(requestAnimationFrame);scrollTo(0,top-innerHeight*.9+(f.offsetHeight+innerHeight*.3)*n/44)}}await new Promise(requestAnimationFrame);o.disconnect();return {frames:270,longTasks:tasks}})()`
);
check(
  "New path animations have no long tasks over 50ms",
  perf.longTasks.length === 0,
  perf
);
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await frames();
check(
  "New diagrams are static with reduced motion",
  await ev(
    "[...document.querySelectorAll('[data-explanation]')].every(e=>!e.getAttribute('style')&&getComputedStyle(e.querySelector('.ex-path')).strokeDasharray==='none')"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: true });
await send("Page.reload");
await new Promise(r => setTimeout(r, 500));
check(
  "New diagrams remain complete without JavaScript",
  await ev(
    "document.querySelectorAll('[data-explanation] svg title').length===6&&[...document.querySelectorAll('.ex-path')].every(e=>getComputedStyle(e).strokeDasharray==='none')"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", { features: [] });
check("No browser exceptions", errors.length === 0, errors);
writeFileSync(
  `${out}/checks.json`,
  JSON.stringify({ checks, perf, passed: checks.every(c => c.passed) }, null, 2)
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
