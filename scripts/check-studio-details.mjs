// Focused checks for install emphasis and supplementary diagrams. Chrome CDP :9226.
import { mkdirSync, writeFileSync } from "node:fs";
const out = "output/studio-details";
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
const load = async path => {
  await send("Page.navigate", { url: "http://127.0.0.1:4174/studio/" + path });
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
await load("index.html");
await ev("localStorage.removeItem('fframes-studio-scroll-motion')");
await load("index.html");
for (const width of [1440, 768, 390, 320]) {
  const height = width === 1440 ? 1000 : width === 768 ? 1024 : 844;
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
  const sizes = await ev(
    `({diagram:document.querySelector('.edit-diagram').getBoundingClientRect().height,download:document.querySelector('.windows-download').href,overflow:document.documentElement.scrollWidth>innerWidth})`
  );
  check(
    `Homepage enlarged diagram and no overflow at ${width}`,
    sizes.diagram >= (width <= 390 ? 300 : 500) && !sizes.overflow,
    sizes
  );
  check(
    `Windows button directly targets installer at ${width}`,
    sizes.download.endsWith(
      "/releases/latest/download/fframes-studio-x86_64-pc-windows-msvc-setup.exe"
    ),
    sizes.download
  );
  await scroll(
    await ev(
      "document.querySelector('#install').getBoundingClientRect().top+scrollY-90"
    )
  );
  await shot(`install-${width}`);
}
await load("how-it-works.html");
for (const width of [1440, 390, 320]) {
  await send("Emulation.setDeviceMetricsOverride", {
    width,
    height: width === 1440 ? 1000 : 844,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await frames();
  for (const name of ["context", "broker", "recovery"]) {
    const selector = `.support-${name}`;
    const position = await ev(
      `document.querySelector('${selector}').getBoundingClientRect().top+scrollY`
    );
    await scroll(position - (await ev("innerHeight*.8")));
    const before = await ev(
      `document.querySelector('${selector}').getAttribute('style')`
    );
    const y = position - (await ev("innerHeight*.1"));
    await scroll(y);
    const snapshot = () =>
      ev(
        `({style:document.querySelector('${selector}').getAttribute('style'),part:getComputedStyle(document.querySelector('${selector} .support-c')).transform})`
      );
    const middle = await snapshot();
    check(`${name} elements move at ${width}`, before !== middle.style, middle);
    await new Promise(r => setTimeout(r, 100));
    check(
      `${name} holds when stopped at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await snapshot())
    );
    await scroll(position + 500);
    await scroll(y);
    check(
      `${name} reverses exactly at ${width}`,
      JSON.stringify(middle) === JSON.stringify(await snapshot())
    );
    await scroll(position - 100);
    await shot(`${name}-${width}`);
    check(
      `${name} introduces no page overflow at ${width}`,
      await ev(
        "document.documentElement.scrollWidth===document.documentElement.clientWidth"
      )
    );
  }
}
await ev("document.querySelector('.support-context .support-scroll').focus()");
await send("Emulation.setFocusEmulationEnabled", { enabled: true });
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
await new Promise(r => setTimeout(r, 150));
check(
  "New diagrams scroll with keyboard and show focus",
  await ev(
    "document.activeElement.matches('.support-scroll:focus-visible') && document.activeElement.scrollLeft>0"
  )
);
const performanceResult = await ev(`(async()=>{
  const longTasks=[];const observer=new PerformanceObserver(l=>longTasks.push(...l.getEntries().map(e=>e.duration)));
  observer.observe({type:'longtask',buffered:false});
  const ranges=[...document.querySelectorAll('[data-scroll-figure]')].map(f=>({top:f.getBoundingClientRect().top+scrollY,height:f.offsetHeight}));
  for(const {top,height} of ranges)for(let i=0;i<45;i++){
    await new Promise(requestAnimationFrame);scrollTo(0,top-innerHeight*.9+(height+innerHeight*.3)*i/44);
  }
  await new Promise(requestAnimationFrame);observer.disconnect();return {frames:135,longTasks};
})()`);
check(
  "Supplementary scroll animation produces no long tasks over 50ms",
  performanceResult.longTasks.length === 0,
  performanceResult
);
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "reduce" }],
});
await frames();
check(
  "Supplementary diagrams respect reduced motion",
  await ev(
    "[...document.querySelectorAll('[data-scroll-figure]')].every(f=>!f.hasAttribute('style')&&getComputedStyle(f.querySelector('.support-c')).transform==='none')"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: true });
await send("Page.reload");
await new Promise(r => setTimeout(r, 500));
check(
  "Supplementary diagrams remain readable without JS",
  await ev(
    "document.querySelectorAll('[data-scroll-figure] svg title').length===3 && [...document.querySelectorAll('[data-scroll-figure] .support-c')].every(e=>getComputedStyle(e).transform==='none')"
  )
);
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", { features: [] });
check("No browser exceptions", errors.length === 0, errors);
writeFileSync(
  `${out}/checks.json`,
  JSON.stringify({ checks, passed: checks.every(c => c.passed) }, null, 2)
);
console.log(
  JSON.stringify(
    { checks: checks.length, failures: checks.filter(c => !c.passed) },
    null,
    2
  )
);
ws.close();
if (checks.some(c => !c.passed)) process.exitCode = 1;
