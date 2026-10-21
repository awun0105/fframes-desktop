// Draws the line illustrations of the fframes Studio pages (landing/studio/) and writes each
// one into its page between `<!-- fig:name -->` and `<!-- /fig:name -->`. The prose stays in
// the HTML; run `node scripts/build-studio-figures.mjs` after changing a drawing here.
// Classes (f-line, t-blue, ...) are styled in landing/studio/studio.css.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const pagesDir = join(root, 'landing/studio');

const COS = Math.cos(Math.PI / 6);
const SIN = 0.5;
const r = (n) => Math.round(n * 10) / 10;
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Isometric projection: x runs down-right, y down-left, z up. */
const iso = (ox, oy) => (x, y, z = 0) => [r(ox + (x - y) * COS), r(oy + (x + y) * SIN - z)];
const poly = (pts, cls) => `<polygon points="${pts.map((p) => p.join(',')).join(' ')}" class="${cls}"/>`;
const line = (x1, y1, x2, y2, cls = 'f-line') => `<line x1="${r(x1)}" y1="${r(y1)}" x2="${r(x2)}" y2="${r(y2)}" class="${cls}"/>`;
const path = (d, cls = 'f-line') => `<path d="${d}" class="${cls}"/>`;
const text = (x, y, s, cls = 'f-label', anchor = 'start') =>
  `<text x="${r(x)}" y="${r(y)}" class="${cls}"${anchor === 'start' ? '' : ` text-anchor="${anchor}"`}>${esc(s)}</text>`;
const rect = (x, y, w, h, cls) => `<rect x="${r(x)}" y="${r(y)}" width="${r(w)}" height="${r(h)}" class="${cls}"/>`;

/**
 * Scroll animation: wraps markup in a group that animates FROM a start state to the drawing
 * as it stands, so the drawing is always the last frame. `kind` is one of the a-* animations
 * in studio.css; `s`..`e` is its window on the figure's 0..100 clock; `vars` sets --dx/--dy.
 * A drawn line or path gets pathLength="1" so the dash animation needs no measuring.
 */
function anim(kind, s, e, inner, vars = {}) {
  const style = Object.entries({ s, e, ...vars }).map(([k, v]) => `--${k}:${r(v)}`).join(';');
  const body = kind === 'draw' ? inner.replace(/^<(path|line) /, '<$1 pathLength="1" ') : inner;
  return `<g class="a a-${kind}" style="${style}">${body}</g>`;
}

/** A playhead that is not part of the drawing: it sweeps on [s, e] from dx, then fades out. */
const playhead = (x, y1, y2, [s, e], dx, out) => anim('out', out, out + 6, anim('move', s, e, line(x, y1, x, y2, 'playhead'), { dx }));

/** A slab of width w (x), depth d (y) and thickness t whose bottom sits at z. */
function slab(P, x, y, z, w, d, t, tone, extra = '') {
  const top = [P(x, y, z + t), P(x + w, y, z + t), P(x + w, y + d, z + t), P(x, y + d, z + t)];
  const right = [P(x + w, y, z), P(x + w, y + d, z), P(x + w, y + d, z + t), P(x + w, y, z + t)];
  const left = [P(x, y + d, z), P(x + w, y + d, z), P(x + w, y + d, z + t), P(x, y + d, z + t)];
  return `<g${extra ? ` class="${extra}"` : ''}>${poly(left, `s-${tone} f-line`)}${poly(right, `s-${tone} f-line`)}${poly(top, `t-${tone} f-line`)}</g>`;
}

/** Leader from a point to a label, horizontal, with the label left or right of it. */
function leader(x, y, toX, label, sub, side) {
  const tx = side === 'right' ? toX + 8 : toX - 8;
  const anchor = side === 'right' ? 'start' : 'end';
  return line(x, y, toX, y, 'f-lead') + text(tx, y + 4, label, 'f-label', anchor) + (sub ? text(tx, y + 18, sub, 'f-small', anchor) : '');
}

function svg(name, w, h, title, desc, body, minWidth) {
  const style = minWidth ? ` style="--min:${minWidth}px"` : '';
  return `<svg viewBox="0 0 ${w} ${h}" role="img" aria-labelledby="${name}-t ${name}-d"${style}><title id="${name}-t">${esc(title)}</title><desc id="${name}-d">${esc(desc)}</desc>${body}</svg>`;
}

/* ---------- the Studio stack ---------- */

const LIBRARY = [
  { label: 'Your video', sub: 'Rust · Video trait', tone: 'yellow' },
  { label: 'svgr! tree', sub: 'SVG built per frame', tone: 'paper' },
  { label: 'Renderer', sub: 'CPU or Skia GPU', tone: 'grey' },
];
const STUDIO = [
  ...LIBRARY,
  { key: 'sdk', label: 'Managed SDK', sub: 'Rust · FFmpeg · Skia, pinned', tone: 'green' },
  { key: 'worker', label: 'Preview worker', sub: 'your video as a program', tone: 'paper' },
  { key: 'engine', label: 'Engine', sub: 'build · check · history', tone: 'paper' },
  { key: 'agent', label: 'Coding agent', sub: 'Claude Code · Codex · ACP', tone: 'red' },
  { key: 'window', label: 'Studio window', sub: 'preview · timeline · chat', tone: 'blue' },
];

/**
 * An exploded stack of layers, bottom first. `focus` dims every other layer. `motion(i)` may
 * return `[kind, s, e, vars]` to animate layer i, slab and label together.
 */
function stack(P, layers, { size = 150, t = 9, gap = 30, side = 'right', labelX, focus, muteLibrary = false, motion }) {
  const out = [];
  const zTop = (layers.length - 1) * (t + gap);
  for (const [cx, cy] of [[0, size], [size, size], [size, 0]]) {
    const [x1, y1] = P(cx, cy, t);
    const [, y2] = P(cx, cy, zTop + t);
    out.push(line(x1, y1, x1, y2, 'f-faint'));
  }
  layers.forEach((layer, i) => {
    const z = i * (t + gap);
    const dim = (focus && layer.key !== focus) || (muteLibrary && !layer.key) ? 'muted' : '';
    const [ax, ay] = side === 'right' ? P(size, 0, z + t) : P(0, size, z + t);
    const [lx] = [labelX];
    const body =
      slab(P, 0, 0, z, size, size, t, focus && layer.key === focus ? 'blue' : layer.tone, dim) +
      `<g${dim ? ' class="muted"' : ''}>${leader(ax + (side === 'right' ? 6 : -6), ay, lx, layer.label, layer.sub, side)}</g>`;
    const m = motion?.(i);
    out.push(m ? anim(m[0], m[1], m[2], body, m[3]) : body);
  });
  return out.join('');
}

function heroStacks() {
  const left = iso(330, 310);
  const right = iso(720, 310);
  const body =
    stack(left, LIBRARY, { side: 'left', labelX: 180 }) +
    // Studio's five layers settle onto the library's three from faint ghosts above
    stack(right, STUDIO, { side: 'right', labelX: 880, muteLibrary: true, motion: (i) => i >= 3 && ['ghost', (i - 3) * 16, (i - 3) * 16 + 24, { dy: -36 }] }) +
    text(330, 500, 'fframes library', 'f-title', 'middle') +
    text(720, 500, 'fframes Studio', 'f-title', 'middle');
  return svg('fig-hero', 1100, 520, 'The fframes library and the Studio stack',
    'Left: the library as three layers, your video, the svgr tree and the renderer. Right: the same three layers with five more on top, the managed SDK, the preview worker, the engine, the coding agent and the Studio window.', body, 900);
}

function focusStack(name, key, caption) {
  const P = iso(170, 235);
  const body = stack(P, STUDIO, { side: 'right', labelX: 330, focus: key, t: 7, gap: 22, size: 120 });
  return svg(name, 560, 380, `The Studio stack, ${caption}`, `The eight layers of Studio with the ${caption} layer highlighted.`, body);
}

/* ---------- selection ---------- */

function selection() {
  const P = iso(260, 150);
  const W = 220, D = 124, t = 4, gap = 46;
  // in motion: the planes start flat and separate, the pointer clicks the title, then the lead
  // line carries it to the card and the rows fill in
  const parts = [];
  parts.push(slab(P, 0, 0, 0, W, D, t, 'grey'));
  // shapes on the middle plane
  const z1 = t + gap + t;
  parts.push(anim('move', 0, 40,
    slab(P, 0, 0, t + gap, W, D, t, 'paper') +
    poly([P(30, 70, z1), P(90, 70, z1), P(90, 110, z1), P(30, 110, z1)], 't-green f-line') +
    poly([P(140, 20, z1), P(200, 20, z1), P(200, 60, z1), P(140, 60, z1)], 't-red f-line'), { dy: t + gap }));
  const z2 = 2 * (t + gap) + t;
  const title = [P(40, 30, z2), P(170, 30, z2), P(170, 52, z2), P(40, 52, z2)];
  parts.push(anim('move', 0, 40, slab(P, 0, 0, 2 * (t + gap), W, D, t, 'paper') + poly(title, 't-blue f-line k-blue'), { dy: 2 * (t + gap) }));
  const guides = [[0, D], [W, D], [W, 0]].map(([cx, cy]) => {
    const [x1, y1] = P(cx, cy, t);
    const [, y2] = P(cx, cy, z2);
    return line(x1, y1, x1, y2, 'f-faint');
  });
  parts.push(anim('fade', 25, 45, guides.join('')));
  // labels on the left
  const lab = (z, label, sub) => { const [x, y] = P(0, D, z); return leader(x - 6, y, 130, label, sub, 'left'); };
  parts.push(anim('fade', 25, 45, lab(t, 'Background', '') + lab(t + gap + t, 'Shapes', '') + lab(z2, 'Title', 'id intro.title')));
  // the pointer, on the title
  const [px, py] = P(105, 41, z2);
  parts.push(anim('pop', 42, 50, `<path d="M${px} ${py} l0 16 l4 -4 l3 7 l3 -1 l-3 -7 l6 0 z" class="t-dark f-line"/>`));
  // packet card
  const cx = 520, cy = 92;
  parts.push(anim('fade', 52, 62, rect(cx, cy, 250, 150, 't-paper f-line') + text(cx + 14, cy + 22, 'Task context', 'f-title')));
  const rows = [['element', 'intro.title'], ['frame', '126 @ 30 fps'], ['bounds', '160 220 1200 180'], ['source', 'src/lib.rs'], ['symbol', 'render_frame'], ['request', 'slide it in from the left']];
  rows.forEach(([k, v], i) => {
    parts.push(anim('move-fade', 60 + i * 6, 70 + i * 6, text(cx + 14, cy + 46 + i * 17, k, 'f-small') + text(cx + 86, cy + 46 + i * 17, v, 'f-code'), { dx: -8 }));
  });
  parts.push(anim('draw', 50, 70, path(`M${px + 14} ${py - 6} C ${px + 120} ${py - 40}, ${cx - 80} ${cy + 40}, ${cx} ${cy + 40}`, 'f-lead k-blue')));
  return svg('fig-select', 800, 350, 'A click on the preview becomes task context',
    'A frame exploded into three planes, background, shapes and a title. A pointer clicks the title, and a line leads to a card listing the element id, the frame, its bounds, the source file, the symbol and the request.', parts.join(''), 600);
}

/* ---------- the loop ---------- */

function loop() {
  const steps = [['Brief', 'or selection'], ['Ground', 'freeze context'], ['Edit', 'draft copy'], ['Build', 'pinned SDK'], ['Inspect', 'frames, audio'], ['Preview', 'at playhead'], ['Accept', 'atomic write'], ['Export', 'accepted only']];
  const x0 = 46, dx = 106, y = 70;
  // in motion the line draws on 0..70 and step i lights up as it passes, at 10 * i
  const parts = [anim('draw', 0, 70, line(x0, y, x0 + dx * 7, y))];
  steps.forEach(([a, b], i) => {
    const x = x0 + dx * i;
    const tone = i === 2 ? 't-red' : i >= 6 ? 't-blue' : 't-paper';
    parts.push(anim('pop', i * 10, i * 10 + 6, `<circle cx="${x}" cy="${y}" r="9" class="${tone} f-line"/>`));
    parts.push(anim('fade', i * 10, i * 10 + 8, text(x, y + 32, a, 'f-label', 'middle') + text(x, y + 47, b, 'f-small', 'middle')));
  });
  // where the agent stops vs where the video is ready
  parts.push(anim('move-fade', 22, 30, path(`M${x0 + dx * 2} ${y - 14} V ${y - 34}`, 'f-lead k-red') + text(x0 + dx * 2, y - 40, 'agent finished', 'f-label c-red', 'middle'), { dy: 8 }));
  parts.push(anim('move-fade', 52, 60, path(`M${x0 + dx * 5} ${y - 14} V ${y - 34}`, 'f-lead k-blue') + text(x0 + dx * 5, y - 40, 'video ready', 'f-label c-blue', 'middle'), { dy: 8 }));
  // the one repair turn: inspect fails, the arc goes back to edit before the line moves on
  parts.push(anim('draw', 42, 52, path(`M${x0 + dx * 4} ${y + 58} C ${x0 + dx * 4} ${y + 92}, ${x0 + dx * 2} ${y + 92}, ${x0 + dx * 2} ${y + 58}`, 'f-lead k-red')));
  parts.push(anim('fade', 48, 56, text(x0 + dx * 3, y + 104, 'fails: one repair turn', 'f-small', 'middle')));
  return svg('fig-loop', 836, 190, 'From a request to an exported video',
    'Eight steps on a line: brief, ground, edit, build, inspect, preview, accept, export. The agent is finished after edit; the video is ready only after preview. A failed inspection loops back to edit once.', parts.join(''), 640);
}

/* ---------- timing ---------- */

function timing() {
  const parts = [];
  const x0 = 40, y0 = 40, w = 460;
  // in motion a playhead sweeps the timeline on 10..70; `at` is when it reaches an x
  const at = (x) => 10 + 60 * ((x - x0) / w);
  const ruler = [line(x0, y0, x0 + w, y0)];
  for (let i = 0; i <= 23; i++) ruler.push(line(x0 + i * 20, y0, x0 + i * 20, y0 + (i % 5 === 0 ? 8 : 4), 'f-lead'));
  parts.push(anim('fade', 0, 12, ruler.join('')));
  const clips = [['Intro', 0, 130, 't-blue'], ['Main', 130, 170, 't-paper'], ['Main #2', 300, 90, 't-paper'], ['Outro', 390, 70, 't-paper']];
  for (const [label, x, cw, tone] of clips) {
    parts.push(anim('dim', at(x0 + x + 2), at(x0 + x + 2) + 6, rect(x0 + x + 2, y0 + 20, cw - 4, 34, `${tone} f-line`) + text(x0 + x + 10, y0 + 41, label, 'f-label')));
  }
  // split and trim marks
  parts.push(anim('pop', at(x0 + 300), at(x0 + 300) + 5, line(x0 + 300, y0 + 10, x0 + 300, y0 + 70, 'f-lead k-red') + text(x0 + 300, y0 + 84, 'split', 'f-small c-red', 'middle')));
  parts.push(anim('pop', at(x0 + 128), at(x0 + 128) + 5, path(`M${x0 + 128} ${y0 + 26} v22`, 'f-line k-blue') + text(x0 + 128, y0 + 84, 'trim', 'f-small c-blue', 'middle')));
  // what changes
  parts.push(anim('draw', 72, 80, path(`M${x0 + w + 10} ${y0 + 37} H 560`, 'f-lead')));
  parts.push(anim('dim', 78, 88, rect(560, 22, 170, 34, 't-blue f-line') + text(574, 43, 'timeline.json', 'f-label')));
  parts.push(anim('fade', 82, 90, text(745, 43, 'written', 'f-small')));
  parts.push(rect(560, 72, 170, 34, 't-grey f-line'));
  parts.push(text(574, 93, 'build', 'f-label'));
  parts.push(anim('fade', 88, 96, text(745, 93, 'untouched', 'f-small')));
  parts.push(playhead(x0 + w, y0 - 10, y0 + 64, [10, 70], -w, 90));
  return svg('fig-timing', 820, 140, 'Timing edits write timeline.json, not a new build',
    'A timeline with four clips, a split and a trimmed edge. An arrow points to timeline.json, which is written, and to the build, which is untouched.', parts.join(''), 640);
}

/* ---------- processes ---------- */

function processes() {
  const box = (x, y, w, h, label, sub, tone = 't-paper') => rect(x, y, w, h, `${tone} f-line`) + text(x + 12, y + 22, label) + (sub ? text(x + 12, y + 38, sub, 'f-small') : '');
  const p = [];
  p.push(`<rect x="20" y="24" width="470" height="380" class="f-faint"/>`);
  p.push(text(32, 44, 'fframes-studio process', 'f-title'));
  p.push(box(40, 64, 200, 52, 'UI thread', 'draws only', 't-blue'));
  p.push(box(270, 64, 200, 52, 'Engine thread', 'one job at a time'));
  p.push(box(40, 176, 200, 52, 'Agent workflow', 'task state machine'));
  p.push(box(270, 176, 200, 52, 'Build service', 'one build per key'));
  p.push(box(40, 288, 200, 52, 'Journal + store', 'history, recovery'));
  p.push(box(270, 288, 200, 52, 'Tool broker', 'read-only tools'));
  p.push(line(240, 90, 270, 90)); p.push(line(140, 116, 140, 176)); p.push(line(370, 116, 370, 176));
  p.push(line(240, 202, 270, 202)); p.push(line(140, 228, 140, 288));
  p.push(path('M200 228 V258 H370 V288'));
  // outside
  p.push(rect(560, 140, 300, 88, 't-dark f-line') + text(574, 164, 'Coding agent', 'f-on-dark') + text(574, 182, 'contained process', 'f-small-dark') + text(574, 198, 'edits the draft copy', 'f-small-dark'));
  p.push(rect(560, 30, 170, 52, 't-grey f-line') + text(572, 52, 'Draft copy') + text(572, 68, 'app data', 'f-small'));
  p.push(line(645, 140, 645, 82));
  p.push(path('M200 176 V150 H560', 'f-line k-red'));
  p.push(text(300, 144, 'claude · codex · acp', 'f-small c-red'));
  p.push(path('M470 314 H620 V228'));
  p.push(text(630, 264, 'mcp / cli', 'f-small'));
  p.push(rect(560, 330, 300, 88, 't-yellow f-line') + text(574, 354, 'Preview worker') + text(574, 372, 'your video, compiled', 'f-small') + text(574, 388, 'frames · timeline · audio', 'f-small'));
  p.push(path('M470 90 H530 V374 H560', 'f-line k-blue'));
  p.push(text(540, 440, 'preview protocol', 'f-small c-blue'));
  return svg('fig-proc', 880, 450, 'Studio’s processes',
    'Inside the Studio process: the UI thread, the engine thread, the agent workflow, the build service, the journal and the tool broker. Outside: the coding agent, which edits a draft copy and calls the tool broker, and the preview worker, your video compiled as its own program.', p.join(''), 640);
}

/* ---------- the edit transaction, as filters ---------- */

function filters() {
  const P = iso(330, 290);
  const size = 150, t = 5, gap = 40;
  const layers = [
    ['Capture twice', 'both copies must match', 'paper'],
    ['Build', 'pinned SDK, isolated copy', 'paper'],
    ['Inspect', 'range, edges, audio', 'paper'],
    ['Review', 'optional: keep some files', 'paper'],
    ['Publish', 'one atomic file write', 'blue'],
  ];
  const parts = [];
  const zTop = (layers.length - 1) * (t + gap);
  for (const [cx, cy] of [[0, size], [size, size], [size, 0]]) {
    const [x1, y1] = P(cx, cy, t);
    const [, y2] = P(cx, cy, zTop + t);
    parts.push(line(x1, y1, x1, y2, 'f-faint'));
  }
  // in motion the draft bar grows on 0..86 and plane i lights up as it reaches it
  layers.forEach(([label, sub, tone], i) => {
    const z = i * (t + gap);
    const [ax, ay] = P(size, 0, z + t);
    parts.push(anim('dim', 7 + i * 13, 13 + i * 13, slab(P, 0, 0, z, size, size, t, tone, 'see-through') + leader(ax + 6, ay, 520, label, sub, 'right')));
  });
  // the draft, travelling up through every plane
  const [bx, yStart] = P(size / 2, size / 2, -36);
  const [, yEnd] = P(size / 2, size / 2, zTop + 92);
  parts.push(anim('grow-y', 0, 86, `<rect x="${bx - 6}" y="${yEnd}" width="12" height="${r(yStart - yEnd)}" class="c-blue" opacity=".45"/>`));
  parts.push(anim('move-fade', 84, 96,
    `<path d="M${bx - 13} ${r(yEnd + 12)} L${bx} ${r(yEnd - 4)} L${bx + 13} ${r(yEnd + 12)}" class="f-line k-blue"/>` +
    text(bx, yEnd - 14, 'accepted revision', 'f-label c-blue', 'middle'), { dy: 10 }));
  const [, yBottom] = P(size, size, 0);
  parts.push(text(bx, yBottom + 24, 'agent draft', 'f-label', 'middle'));
  // the failure path, out of inspect, when the bar passes it
  const [fx, fy] = P(0, size, 2 * (t + gap) + t);
  parts.push(anim('draw', 36, 46, path(`M${fx - 6} ${fy} H 160`, 'f-lead k-red')));
  parts.push(anim('fade', 40, 50, text(152, fy + 4, 'fail', 'f-label c-red', 'end') + text(152, fy + 18, 'one repair turn', 'f-small', 'end')));
  return svg('fig-filters', 760, 480, 'Every edit passes five filters',
    'The agent’s draft travels up through five planes: capture twice, build, inspect, optional review and publish. It comes out on top as the accepted revision. A failure at inspect goes back for one repair turn.', parts.join(''), 600);
}

/* ---------- from a pixel to source ---------- */

function chain() {
  const cards = [
    ['Pixel', ['x 812  y 341', 'frame 126']],
    ['Object map', ['sent with the frame', 'topmost hit wins']],
    ['Editor key', ['intro / starter-title', 'headline / primary']],
    ['Source', ['src/lib.rs', 'render_frame']],
  ];
  const w = 168, gap = 30, y = 30;
  const p = [];
  cards.forEach(([title, lines], i) => {
    const x = 20 + i * (w + gap);
    p.push(rect(x, y, w, 84, `${i === 3 ? 't-blue' : 't-paper'} f-line`));
    p.push(text(x + 12, y + 22, title, 'f-title'));
    lines.forEach((l, j) => p.push(text(x + 12, y + 46 + j * 16, l, 'f-code')));
    if (i < 3) p.push(path(`M${x + w + 4} ${y + 42} H ${x + w + gap - 4} m-6 -4 l6 4 l-6 4`, 'f-line'));
  });
  return svg('fig-chain', 812, 140, 'From a pixel to the Rust that drew it',
    'Four cards in a row: the clicked pixel at frame 126, the object map the worker sends with the frame, the editor key the code registered, and the source file and function.', p.join(''), 640);
}

/* ---------- swapping previews ---------- */

function swap() {
  const p = [];
  const x0 = 140, x1 = 760, swapX = 560;
  p.push(text(20, 54, 'old build', 'f-label'));
  p.push(text(20, 114, 'new build', 'f-label'));
  p.push(rect(x0, 40, swapX - x0, 22, 't-paper f-line'));
  p.push(text(x0 + 10, 55, 'playing', 'f-small'));
  // in motion a playhead runs to the swap on 0..70 while the new build fills in under it
  const at = (px) => 70 * ((px - x0) / (swapX - x0));
  const phases = [['compile', 186, 't-grey'], ['stage', 112, 't-yellow'], ['ready', 62, 't-green']];
  let x = x0 + 60;
  for (const [label, w, tone] of phases) {
    p.push(anim('grow-x', at(x), at(x + w), rect(x, 100, w, 22, `${tone} f-line`)));
    p.push(anim('fade', at(x + w) - 2, at(x + w) + 4, text(x + 8, 115, label, 'f-small')));
    x += w;
  }
  p.push(anim('grow-x', 72, 90, rect(swapX, 100, x1 - swapX, 22, 't-blue f-line')));
  p.push(anim('fade', 88, 94, text(swapX + 10, 115, 'playing', 'f-small')));
  p.push(anim('fade', 68, 78,
    line(swapX, 24, swapX, 140, 'f-line k-blue') +
    text(swapX, 160, 'swap at the playhead', 'f-label c-blue', 'middle') +
    text(swapX, 175, 'worker · frame · audio together', 'f-small', 'middle')));
  p.push(playhead(swapX, 24, 140, [0, 70], x0 - swapX, 74));
  return svg('fig-swap', 800, 190, 'A new build replaces the old one only when it is ready',
    'Two lanes. The old build keeps playing while the new one compiles, stages and becomes ready. At that moment both swap at the playhead, worker, frame and audio together.', p.join(''), 640);
}

/* ---------- containment ---------- */

function fence() {
  const p = [];
  p.push(`<rect x="40" y="30" width="420" height="170" rx="6" class="f-line k-blue" stroke-dasharray="5 4"/>`);
  p.push(text(54, 52, 'job object · process group', 'f-label c-blue'));
  p.push(rect(70, 80, 140, 40, 't-dark f-line') + text(84, 104, 'agent', 'f-on-dark'));
  const kids = [[260, 70, 'node'], [260, 130, 'cargo'], [120, 150, 'rustc']];
  for (const [x, y, label] of kids) {
    p.push(rect(x, y, 110, 32, 't-paper f-line') + text(x + 12, y + 20, label));
  }
  p.push(path('M210 100 H260')); p.push(path('M210 110 C 235 110, 240 146, 260 146')); p.push(path('M140 120 V150'));
  p.push(rect(560, 90, 130, 32, 't-red f-line') + text(572, 110, 'escaped child'));
  p.push(path('M370 86 C 470 86, 480 106, 560 106', 'f-lead k-red'));
  p.push(text(625, 150, 'candidate refused', 'f-label c-red', 'middle'));
  return svg('fig-fence', 740, 220, 'Only a contained agent’s work is captured',
    'The agent and its child processes sit inside a dashed fence, a job object on Windows or a process group elsewhere. A child that escapes the fence makes Studio refuse the candidate.', p.join(''), 560);
}

const FIGURES = {
  hero: heroStacks,
  'stack-sdk': () => focusStack('fig-stack-sdk', 'sdk', 'managed SDK'),
  'stack-agent': () => focusStack('fig-stack-agent', 'agent', 'coding agent'),
  select: selection,
  loop,
  timing,
  proc: processes,
  filters,
  chain,
  swap,
  fence,
};

let missing = new Set(Object.keys(FIGURES));
for (const page of ['index.html', 'how-it-works.html']) {
  const file = join(pagesDir, page);
  const html = readFileSync(file, 'utf8');
  const next = html.replace(/<!-- fig:([a-z-]+) -->[\s\S]*?<!-- \/fig:\1 -->/g, (_, name) => {
    if (!FIGURES[name]) throw new Error(`${page}: no figure named ${name}`);
    missing.delete(name);
    return `<!-- fig:${name} -->${FIGURES[name]()}<!-- /fig:${name} -->`;
  });
  writeFileSync(file, next);
}
if (missing.size) console.warn(`figures not placed in any page: ${[...missing].join(', ')}`);
console.log('Wrote the Studio figures into landing/studio.');
