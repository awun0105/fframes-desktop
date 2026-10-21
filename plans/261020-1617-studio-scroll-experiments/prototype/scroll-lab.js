// Scroll lab: tags the generated Studio figures with animation hooks at runtime, so the live
// SVGs stay untouched. In production scripts/build-studio-figures.mjs would emit the same
// classes and custom properties directly. See scroll-lab.css for the engine.
const NS = 'http://www.w3.org/2000/svg';
const $$ = (root, sel) => [...root.querySelectorAll(sel)];
const num = (el, attr) => parseFloat(el.getAttribute(attr));
const params = new URLSearchParams(location.search);
const root = document.documentElement;

/** Give an element an animation kind and its window on the figure's 0..100 clock. */
function anim(el, kind, s, e, vars = {}) {
  el.classList.add('a', `a-${kind}`);
  el.style.setProperty('--s', s);
  el.style.setProperty('--e', e);
  for (const [k, v] of Object.entries(vars)) el.style.setProperty(`--${k}`, v);
  if (kind === 'draw') el.setAttribute('pathLength', '1');
  return el;
}

/** Wrap elements in a new <g> placed where the first one was. */
function wrap(els) {
  const g = document.createElementNS(NS, 'g');
  els[0].before(g);
  g.append(...els);
  return g;
}

/** A rect and the text drawn inside it, as one group. */
function box(svg, rect) {
  const [x, y, w, h] = ['x', 'y', 'width', 'height'].map((a) => num(rect, a));
  const texts = $$(svg, 'text').filter((t) => num(t, 'x') >= x && num(t, 'x') <= x + w && num(t, 'y') >= y && num(t, 'y') <= y + h);
  return wrap([rect, ...texts]);
}

const textOf = (svg, s) => $$(svg, 'text').find((t) => t.textContent === s);
const pathOf = (svg, start) => $$(svg, 'path').find((p) => p.getAttribute('d').startsWith(start));

/** A playhead that is not in the drawing: it sweeps on its window, then fades out on `out`. */
function playhead(svg, x, y1, y2, [s, e], dx, out) {
  const line = document.createElementNS(NS, 'line');
  for (const [k, v] of Object.entries({ x1: x, x2: x, y1, y2 })) line.setAttribute(k, v);
  line.setAttribute('class', 'lab-playhead');
  svg.append(line);
  anim(line, 'move', s, e, { dx });
  anim(wrap([line]), 'out', out, out + 6);
}

/** Isometric stacks: faint guide lines start a stack, then each layer is a slab <g> plus its leader <g>. */
function stacks(svg) {
  const out = [];
  let cur = null;
  const kids = [...svg.children];
  for (let k = 0; k < kids.length; k++) {
    const el = kids[k];
    if (el.matches('line.f-faint')) {
      if (!cur || cur.layers.length) out.push((cur = { faint: [], layers: [] }));
      cur.faint.push(el);
    } else if (el.tagName === 'g' && cur) {
      const layer = wrap([el, kids[++k]]);
      layer.focus = !el.classList.contains('muted');
      cur.layers.push(layer);
    }
  }
  return out;
}

const FIGURES = {
  hero(svg) {
    const [lib, studio] = stacks(svg);
    if (pinnedHero) {
      // both stacks open from closed blocks, then Studio's five layers settle one by one.
      // The hero is on screen at load, so nothing starts empty: the bottom label is dimmed and
      // the five layers are ghosts; only the labels that would sit on top of it are hidden
      for (const st of [lib, studio]) {
        st.layers.slice(0, 3).forEach((layer, i) => {
          anim(layer, 'move', 0, 30, { dy: i * 39 });
          anim(layer.lastElementChild, i === 0 ? 'dim' : 'fade', 20, 35);
        });
      }
      lib.faint.forEach((l) => anim(l, 'fade', 0, 30));
      studio.faint.forEach((l) => anim(l, 'fade', 30, 60));
      studio.layers.slice(3).forEach((layer, i) => anim(layer, 'ghost', 35 + i * 12, 49 + i * 12, { dy: -40 }));
    } else {
      // the library stays put; Studio's layers start as ghosts above and settle as you scroll
      studio.layers.slice(3).forEach((layer, i) => anim(layer, 'ghost', i * 16, i * 16 + 24, { dy: -36 }));
    }
  },

  'stack-sdk': focusStack,
  'stack-agent': focusStack,

  select(svg) {
    const kids = [...svg.children];
    const slabs = kids.filter((e) => e.tagName === 'g');
    const polys = kids.filter((e) => e.tagName === 'polygon');
    anim(wrap([slabs[1], polys[0], polys[1]]), 'move', 0, 40, { dy: 50 });
    anim(wrap([slabs[2], polys[2]]), 'move', 0, 40, { dy: 100 });
    for (const el of [...$$(svg, 'line.f-faint'), ...$$(svg, 'line.f-lead'), ...$$(svg, 'text[x="122"]')]) anim(el, 'fade', 25, 45);
    anim(svg.querySelector('path.t-dark'), 'pop', 42, 50);
    anim(svg.querySelector('path.k-blue'), 'draw', 50, 70);
    anim(wrap([svg.querySelector('rect'), textOf(svg, 'Task context')]), 'fade', 52, 62);
    for (let i = 0; i < 6; i++) {
      const row = $$(svg, `text[y="${138 + i * 17}"]`);
      anim(wrap(row), 'move-fade', 60 + i * 6, 70 + i * 6, { dx: -8 });
    }
  },

  loop(svg) {
    anim(svg.querySelector('line'), 'draw', 0, 70);
    $$(svg, 'circle').forEach((c, i) => {
      anim(c, 'pop', i * 10, i * 10 + 6);
      for (const t of $$(svg, `text[x="${c.getAttribute('cx')}"]`).filter((t) => num(t, 'y') > 90)) anim(t, 'fade', i * 10, i * 10 + 8);
    });
    anim(wrap([pathOf(svg, 'M258 56'), textOf(svg, 'agent finished')]), 'move-fade', 22, 30, { dy: 8 });
    // inspect (step 4, at 40) fails once: the repair arc goes back before the line moves on
    anim(pathOf(svg, 'M470 128'), 'draw', 42, 52);
    anim(textOf(svg, 'fails: one repair turn'), 'fade', 48, 56);
    anim(wrap([pathOf(svg, 'M576 56'), textOf(svg, 'video ready')]), 'move-fade', 52, 60, { dy: 8 });
  },

  timing(svg) {
    // ruler first, then a playhead sweeps x 40..500 on the clock 10..70
    const at = (x) => 10 + 60 * ((x - 40) / 460);
    anim(wrap($$(svg, 'line').filter((l) => num(l, 'y1') === 40)), 'fade', 0, 12);
    for (const rect of $$(svg, 'rect').filter((r) => num(r, 'y') === 60)) anim(box(svg, rect), 'dim', at(num(rect, 'x')), at(num(rect, 'x')) + 6);
    anim(wrap([svg.querySelector('line.k-red'), textOf(svg, 'split')]), 'pop', at(340), at(340) + 5);
    anim(wrap([svg.querySelector('path.k-blue'), textOf(svg, 'trim')]), 'pop', at(168), at(168) + 5);
    anim(pathOf(svg, 'M510 77'), 'draw', 72, 80);
    anim(svg.querySelector('rect[x="560"][y="22"]'), 'fill', 78, 88);
    anim(textOf(svg, 'written'), 'fade', 82, 90);
    anim(textOf(svg, 'untouched'), 'fade', 88, 96);
    playhead(svg, 500, 30, 104, [10, 70], -460, 90);
  },

  proc(svg) {
    anim(wrap([svg.querySelector('rect.f-faint'), textOf(svg, 'fframes-studio process')]), 'fade', 0, 10);
    $$(svg, 'rect').filter((r) => [40, 270].includes(num(r, 'x'))).forEach((r, i) => anim(box(svg, r), 'move-fade', 4 + i * 5, 14 + i * 5, { dy: 8 }));
    for (const l of $$(svg, 'line').filter((l) => num(l, 'x1') !== 645)) anim(l, 'draw', 34, 48);
    anim(pathOf(svg, 'M200 228'), 'draw', 34, 48);
    [['rect[x="560"][y="30"]', 48], ['rect.t-dark', 54], ['rect.t-yellow', 60]].forEach(([sel, s]) => anim(box(svg, svg.querySelector(sel)), 'fade', s, s + 10));
    anim(svg.querySelector('line[x1="645"]'), 'draw', 58, 66);
    anim(pathOf(svg, 'M200 176'), 'draw', 62, 74);
    anim(textOf(svg, 'claude · codex · acp'), 'fade', 70, 78);
    anim(pathOf(svg, 'M470 314'), 'draw', 70, 80);
    anim(textOf(svg, 'mcp / cli'), 'fade', 76, 84);
    anim(pathOf(svg, 'M470 90'), 'draw', 80, 94);
    anim(textOf(svg, 'preview protocol'), 'fade', 88, 98);
  },

  filters(svg) {
    // the bar grows 0..86; plane i is reached at about 11 + 13i
    $$(svg, 'g.see-through').forEach((g, i) => {
      const lead = g.nextElementSibling;
      anim(wrap([g, lead, lead.nextElementSibling, lead.nextElementSibling.nextElementSibling]), 'dim', 7 + i * 13, 13 + i * 13);
    });
    anim(svg.querySelector('rect.c-blue'), 'grow-y', 0, 86);
    anim(wrap([svg.querySelector('path.k-blue'), textOf(svg, 'accepted revision')]), 'move-fade', 84, 96, { dy: 10 });
    anim(svg.querySelector('path.k-red'), 'draw', 36, 46);
    anim(wrap([textOf(svg, 'fail'), textOf(svg, 'one repair turn')]), 'fade', 40, 50);
  },

  chain(svg) {
    $$(svg, 'rect').forEach((r, i) => anim(box(svg, r), 'move-fade', i * 22, i * 22 + 14, { dx: -16 }));
    $$(svg, 'path').forEach((p, i) => anim(p, 'draw', i * 22 + 12, i * 22 + 22));
  },

  swap(svg) {
    // a playhead runs x 140..560 on the clock 0..70 while the new build fills in under it
    const at = (x) => 70 * ((x - 140) / 420);
    for (const rect of $$(svg, 'rect[y="100"]')) {
      const x = num(rect, 'x'), w = num(rect, 'width');
      const [s, e] = x >= 560 ? [72, 90] : [at(x), at(x + w)];
      const label = box(svg, rect);
      anim(rect, 'grow-x', s, e);
      anim(label.lastElementChild, 'fade', e - 2, e + 4);
    }
    anim(wrap([svg.querySelector('line.k-blue'), textOf(svg, 'swap at the playhead'), textOf(svg, 'worker · frame · audio together')]), 'fade', 68, 78);
    playhead(svg, 560, 24, 140, [0, 70], -420, 74);
  },

  fence(svg) {
    anim(wrap([svg.querySelector('rect[rx]'), textOf(svg, 'job object · process group')]), 'fade', 0, 14);
    anim(box(svg, svg.querySelector('rect.t-dark')), 'pop', 8, 20);
    $$(svg, 'rect.t-paper').forEach((r, i) => anim(box(svg, r), 'pop', 22 + i * 8, 30 + i * 8));
    for (const p of $$(svg, 'path[class="f-line"]')) anim(p, 'draw', 20, 40);
    anim(box(svg, svg.querySelector('rect.t-red')), 'move', 48, 74, { dx: -200 });
    anim(svg.querySelector('path.k-red'), 'draw', 72, 84);
    anim(textOf(svg, 'candidate refused'), 'move-fade', 84, 96, { dy: 6 });
  },
};

/** The eight-layer stack opens from a closed block, then the focused layer slides into its slot. */
function focusStack(svg) {
  const [st] = stacks(svg);
  st.faint.forEach((l) => anim(l, 'fade', 0, 40));
  st.layers.forEach((layer, i) => {
    anim(layer, 'move', 0, 45, { dy: i * 29 });
    anim(layer.lastElementChild, 'fade', 30, 50);
    if (layer.focus) anim(layer.firstElementChild, 'move-fade', 45, 85, { dx: -62, dy: -36 });
  });
}

// ---------- controls ----------
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const scrubbable = CSS.supports('animation-timeline: view()');
const state = {
  mode: scrubbable ? params.get('mode') ?? 'scrub' : 'play',
  hero: params.get('hero') ?? 'assemble',
  motion: params.get('motion') ?? (reduced ? 'off' : 'on'),
};
// the hero only pins where the CSS pins it; narrower screens get the assemble variant
const pinnedHero = state.hero === 'pin' && matchMedia('(min-width: 900px)').matches;

function apply() {
  root.classList.toggle('mode-scrub', state.mode === 'scrub');
  root.classList.toggle('mode-play', state.mode === 'play');
  root.classList.toggle('hero-pin', state.hero === 'pin');
  root.classList.toggle('lab-motion', state.motion === 'on');
  for (const group of document.querySelectorAll('.lab-group')) {
    for (const button of group.querySelectorAll('button')) {
      button.setAttribute('aria-pressed', String(state[group.dataset.control] === button.dataset.value));
      if (group.dataset.control === 'mode' && button.dataset.value === 'scrub' && !scrubbable) button.disabled = true;
    }
  }
}
apply();

for (const svg of document.querySelectorAll('.fig[data-fig] svg')) {
  const fig = svg.closest('.fig');
  fig.insertAdjacentHTML('afterbegin', '<span class="fig-clock" aria-hidden="true"></span>');
  // ?tag=off leaves the SVGs exactly as generated, to compare final frames against
  // figures that ship with hooks from the generator are not tagged again
  if (params.get('tag') !== 'off' && !svg.querySelector('.a')) FIGURES[fig.dataset.fig]?.(svg);
}

for (const group of document.querySelectorAll('.lab-group')) {
  group.addEventListener('click', (event) => {
    const button = event.target.closest('button');
    if (!button) return;
    state[group.dataset.control] = button.dataset.value;
    const url = new URL(location.href);
    url.searchParams.set(group.dataset.control, button.dataset.value);
    // the hero variant tags different elements, so it needs a fresh page
    if (group.dataset.control === 'hero') return location.assign(url);
    history.replaceState(null, '', url);
    apply();
  });
}

// play once: start a figure when a third of it is on screen
const observer = new IntersectionObserver((entries) => {
  for (const entry of entries) if (entry.isIntersecting) entry.target.classList.add('is-visible');
}, { threshold: 0.35 });
for (const fig of document.querySelectorAll('.fig[data-fig]')) observer.observe(fig);

document.querySelector('.lab-replay').addEventListener('click', () => {
  for (const fig of document.querySelectorAll('.fig.is-visible')) {
    fig.classList.remove('is-visible');
    void fig.offsetWidth;
    fig.classList.add('is-visible');
  }
});
