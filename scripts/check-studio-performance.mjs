// Repeatable lab samples against the local preview; not field Core Web Vitals.
import { mkdirSync, writeFileSync } from "node:fs";
const phase = process.argv[2] || "after";
const out = "output/studio-performance";
mkdirSync(out, { recursive: true });
const targets = await (await fetch("http://127.0.0.1:9226/json/list")).json();
const target = targets.find(t => t.type === "page");
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map();
ws.addEventListener("message", ({ data }) => {
  const m = JSON.parse(data);
  if (!m.id) return;
  const p = pending.get(m.id);
  pending.delete(m.id);
  m.error ? p.reject(m.error) : p.resolve(m.result);
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
const delay = ms => new Promise(r => setTimeout(r, ms));
await send("Page.enable");
await send("Page.bringToFront");
await send("Runtime.enable");
await send("Network.enable");
await send("Performance.enable");
await send("Emulation.setScriptExecutionDisabled", { value: false });
await send("Emulation.setEmulatedMedia", {
  features: [{ name: "prefers-reduced-motion", value: "no-preference" }],
});
await send("Network.setCacheDisabled", { cacheDisabled: true });
await send("Network.emulateNetworkConditions", {
  offline: false,
  latency: 150,
  downloadThroughput: 200000,
  uploadThroughput: 93750,
});
await send("Emulation.setCPUThrottlingRate", { rate: 4 });
const observer = await send("Page.addScriptToEvaluateOnNewDocument", {
  source: `
  localStorage.removeItem('fframes-studio-scroll-motion');
  window.lab={lcp:0,cls:0,longTasks:[]};
  new PerformanceObserver(l=>{for(const e of l.getEntries())lab.lcp=e.startTime}).observe({type:'largest-contentful-paint',buffered:true});
  new PerformanceObserver(l=>{for(const e of l.getEntries())if(!e.hadRecentInput)lab.cls+=e.value}).observe({type:'layout-shift',buffered:true});
  new PerformanceObserver(l=>{for(const e of l.getEntries())lab.longTasks.push(e.duration)}).observe({type:'longtask',buffered:true});
`,
});
const samples = [];
try {
  for (const width of [1440, 390]) {
    await send("Emulation.setDeviceMetricsOverride", {
      width,
      height: width === 390 ? 844 : 900,
      deviceScaleFactor: 1,
      mobile: width === 390,
    });
    for (const path of ["/studio/", "/studio/how-it-works.html"]) {
      await send("Network.clearBrowserCache");
      await send("Page.navigate", { url: "http://127.0.0.1:4174" + path });
      // Reject incomplete loads instead of reporting an unpainted page as fast.
      const deadline = Date.now() + 60000;
      while (
        !(await ev(
          `location.pathname === ${JSON.stringify(path)} && document.readyState === 'complete' && !!document.querySelector('[data-motion-controls]')`
        ))
      ) {
        if (Date.now() > deadline) throw Error(`Load timed out: ${path}`);
        await delay(250);
      }
      await ev("document.fonts.ready");
      await ev(
        "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))"
      );
      await delay(9000);
      const loading =
        await ev(`({ ...lab, fcp:performance.getEntriesByName('first-contentful-paint')[0]?.startTime,
        navigation:performance.getEntriesByType('navigation')[0].toJSON(),
        resources:performance.getEntriesByType('resource').map(r=>({name:r.name,duration:r.duration,bytes:r.transferSize,decoded:r.decodedBodySize})),
        fonts:document.fonts.status, overflow:document.documentElement.scrollWidth>innerWidth })`);
      if (!loading.fcp || !loading.lcp)
        throw Error(`No paint recorded: ${path}`);
      const before = (await send("Performance.getMetrics")).metrics;
      const scrolling = await ev(`new Promise(resolve=>{
        document.documentElement.style.scrollBehavior='auto';
        const max=document.documentElement.scrollHeight-innerHeight;
        const gaps=[];let previous=0,start=0;
        function tick(t){if(!start)start=t;if(previous)gaps.push(t-previous);previous=t;
          const p=Math.min(1,(t-start)/4000);scrollTo(0,max*(p<.5?p*2:(1-p)*2));
          if(p<1)requestAnimationFrame(tick);else resolve({frames:gaps.length,over50ms:gaps.filter(n=>n>50).length,maxGap:Math.max(...gaps)});
        }requestAnimationFrame(tick);
      })`);
      const after = (await send("Performance.getMetrics")).metrics;
      const metrics = {};
      for (const name of [
        "ScriptDuration",
        "LayoutDuration",
        "RecalcStyleDuration",
        "TaskDuration",
      ])
        metrics[name] =
          after.find(m => m.name === name).value -
          before.find(m => m.name === name).value;
      const { data } = await send("Page.captureScreenshot", { format: "png" });
      writeFileSync(
        `${out}/${phase}-${width}-${path.includes("how") ? "guide" : "home"}.png`,
        Buffer.from(data, "base64")
      );
      const sample = { width, path, loading, scrolling, scrollCPU: metrics };
      samples.push(sample);
      console.log(
        JSON.stringify({
          width,
          path,
          fcp: loading.fcp,
          lcp: loading.lcp,
          cls: loading.cls,
          requests: loading.resources.length,
          scrolling,
          scrollCPU: metrics,
        })
      );
    }
  }
  writeFileSync(`${out}/${phase}.json`, JSON.stringify(samples, null, 2));
} finally {
  await send("Page.removeScriptToEvaluateOnNewDocument", {
    identifier: observer.identifier,
  });
  await send("Network.emulateNetworkConditions", {
    offline: false,
    latency: 0,
    downloadThroughput: -1,
    uploadThroughput: -1,
  });
  await send("Network.setCacheDisabled", { cacheDisabled: false });
  await send("Emulation.setCPUThrottlingRate", { rate: 1 });
  await send("Emulation.setEmulatedMedia", { features: [] });
  ws.close();
}
