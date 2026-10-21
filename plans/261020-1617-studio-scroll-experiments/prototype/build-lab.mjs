// Builds the scroll lab page from the real Studio figures: it copies every
// `<!-- fig:name -->` block out of landing/studio/*.html into index.html next to this file.
// Rerun after `node scripts/build-studio-figures.mjs` so the lab shows the current drawings.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const pagesDir = join(here, '../../../landing/studio');

const figures = {};
for (const page of ['index.html', 'how-it-works.html']) {
  const html = readFileSync(join(pagesDir, page), 'utf8');
  for (const [, name, svg] of html.matchAll(/<!-- fig:([a-z-]+) -->([\s\S]*?)<!-- \/fig:\1 -->/g)) figures[name] = svg;
}

const EXPERIMENTS = [
  ['stack-sdk', 'Overview · 01', 'Insert the layer', 'The stack opens from a closed block, then the managed SDK slides into its slot from outside.'],
  ['select', 'Overview · 02', 'Explode, click, hand over', 'The frame separates into its planes, the pointer clicks the title, and the lead line carries it to the task context row by row.'],
  ['stack-agent', 'Overview · 03', 'Insert the layer', 'The same stack component again, now the coding agent slides in. One pattern for every focused stack.'],
  ['loop', 'Overview · 04', 'Run the loop', 'The line draws step by step. Inspect fails once and the repair arc goes back to edit before the line reaches preview.'],
  ['timing', 'Overview · 05', 'Scrub the cut', 'A playhead sweeps the timeline, clips light up as it passes, the split and trim snap in, then timeline.json fills blue while the build stays grey.'],
  ['proc', 'How it works · top', 'Wire the processes', 'Threads appear inside the Studio process, then the outside processes, then the wires to them, the preview protocol last.'],
  ['filters', 'How it works · 01', 'Rise through the filters', 'The draft bar grows up through the five planes and each plane lights as it passes. Inspect sends one repair turn out.'],
  ['chain', 'How it works · 02', 'Pixel to Rust', 'Cards and arrows arrive left to right, read in the order the lookup happens.'],
  ['swap', 'How it works · 04', 'Swap at the playhead', 'A playhead runs while the new build compiles, stages and gets ready underneath; the swap appears when they meet.'],
  ['fence', 'How it works · 05', 'The escape', 'The fence and its processes appear, then one child slides out through the fence and the candidate is refused.'],
];

const missing = ['hero', ...EXPERIMENTS.map(([n]) => n)].filter((n) => !figures[n]);
if (missing.length) throw new Error(`figures not found: ${missing.join(', ')}`);

const fig = (name, caption) =>
  `<figure class="fig" data-fig="${name}"><div class="scroll">${figures[name]}</div><figcaption>${caption}<span class="hint">Scroll sideways to see all of it</span></figcaption></figure>`;

const sections = EXPERIMENTS.map(([name, kicker, title, note]) => `
    <section class="block lab-item" aria-label="${title}">
      <p class="kicker">${kicker} &middot; fig:${name}</p>
      <h2>${title}</h2>
      <p class="lede">${note}</p>
      ${fig(name, title)}
    </section>`).join('\n');

const page = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Studio Scroll Lab</title>
<meta name="robots" content="noindex">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Instrument+Serif:ital@1&family=Geist+Mono:wght@400;500;600&display=swap" rel="stylesheet">
<link rel="stylesheet" href="../../../landing/studio/studio.css">
<link rel="stylesheet" href="scroll-lab.css">
</head>
<body>
  <div class="light-leak" aria-hidden="true"></div>
  <div class="grid-lines" aria-hidden="true"><div></div></div>
  <header class="topbar">
    <div class="wrap">
      <span class="wordmark"><img src="../../../landing/brand/fframes-wordmark.svg" alt="" width="95" height="24" /><span>Scroll lab</span></span>
      <nav class="links" aria-label="Lab"><a class="btn" href="../../../landing/studio/index.html">Live page</a></nav>
    </div>
  </header>

  <main class="wrap">
    <section class="masthead">
      <p class="eyebrow">fframes Studio &middot; scroll experiments</p>
      <h1>The library renders it. <em>Studio runs the loop.</em></h1>
      <p class="standfirst">Every figure below is the generated SVG from the live pages, untouched. The animation hooks are added at runtime by scroll-lab.js; the static drawing is always the final frame.</p>
    </section>

    <div class="hero-stage">${fig('hero', 'Same three layers, five more on top')}</div>
${sections}
  </main>

  <div class="lab-bar" role="toolbar" aria-label="Experiment controls">
    <span class="lab-group" data-control="mode"><button type="button" data-value="scrub">Scrub</button><button type="button" data-value="play">Play once</button></span>
    <span class="lab-group" data-control="hero"><button type="button" data-value="assemble">Hero: assemble</button><button type="button" data-value="pin">Hero: pinned</button></span>
    <span class="lab-group" data-control="motion"><button type="button" data-value="on">Motion</button><button type="button" data-value="off">Static</button></span>
    <button type="button" class="lab-replay">Replay</button>
  </div>
  <script src="scroll-lab.js"></script>
</body>
</html>
`;

writeFileSync(join(here, 'index.html'), page);
console.log(`Wrote the scroll lab with ${EXPERIMENTS.length + 1} figures.`);
