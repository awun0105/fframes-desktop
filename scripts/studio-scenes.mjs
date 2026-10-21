// Scroll scenes for the Studio "how it works" figures. Each figure has one progress
// value, --p (0..1), set by how-it-works.js; every animated part reads it inside its
// own window --a..--b. CSS defaults put --p at 1, so without JavaScript or with
// scroll motion off every scene shows its finished state.
const n = x => +x.toFixed(3);

export const win = (a, b) => `style="--a: ${n(a)}; --b: ${n(b)}"`;

// An off window (--c..--d) as well: the part shows inside a..b and hides inside c..d.
export const blink = (a, b, c, d) =>
  `style="--a: ${n(a)}; --b: ${n(b)}; --c: ${n(c)}; --d: ${n(d)}"`;

// kind: fade | rise | pop | lit | hide | draw (see .sc-* rules in how-it-works.css)
export const part = (kind, a, b, inner, cls = "") =>
  `<g class="sc sc-${kind} ${cls}" ${win(a, b)}>${inner}</g>`;

// Absolute M/L/H/V paths only: their corner points, for arrowheads and tokens.
function corners(d) {
  const points = [];
  let x = 0,
    y = 0;
  for (const [, cmd, args] of d.matchAll(/([MLHV])([^MLHV]*)/g)) {
    const v = args
      .trim()
      .split(/[\s,]+/)
      .map(Number);
    if (cmd === "H") x = v[0];
    else if (cmd === "V") y = v[0];
    else [x, y] = v;
    points.push([x, y]);
  }
  return points;
}

// A directed connector that draws itself inside a..b; its head appears as it lands.
export function edge(id, d, a, b, cls = "", headCls = "") {
  const points = corners(d);
  const [x1, y1] = points.at(-2),
    [x2, y2] = points.at(-1);
  const angle = n((Math.atan2(y2 - y1, x2 - x1) * 180) / Math.PI);
  return (
    `<path id="${id}" class="sc sc-draw sc-edge ${cls}" ${win(a, b)} d="${d}" pathLength="1"/>` +
    `<path class="sc sc-fade sc-head ${headCls}" ${win(b - 0.02, b)} d="m-8-4.5 8 4.5-8 4.5" transform="translate(${x2} ${y2}) rotate(${angle})"/>`
  );
}

// A marker that travels along a connector inside a..b (placed by how-it-works.js).
export const token = (along, a, b, inner, cls = "") =>
  `<g class="sc-token ${cls}" data-along="#${along}" data-a="${n(a)}" data-b="${n(b)}">${inner}</g>`;

export const chip = (tone = "pink") =>
  `<rect x="-7" y="-7" width="14" height="14" rx="3" class="sc-chip sc-chip-${tone}"/>`;

// Highlight ring for the part that is active inside a..b, gone again by c..d.
export const ring = (x, y, w, h, a, b, c, d, rx = 6) =>
  `<rect x="${x - 4}" y="${y - 4}" width="${w + 8}" height="${h + 8}" rx="${rx}" class="sc sc-blink sc-ring" ${blink(a, b, c, d)}/>`;

// Words of a caption in lines of at most `width` characters.
const wrap = (text, width) =>
  text.split(" ").reduce((lines, word) => {
    const line = lines.at(-1);
    if (line && line.length + word.length < width)
      lines[lines.length - 1] = `${line} ${word}`;
    else lines.push(word);
    return lines;
  }, []);

// One caption per scene step, all in one place; each replaces the previous one.
// With `width` (characters) a caption wraps onto further lines below y.
export function narration(x, y, steps, anchor = "start", width = 0) {
  return `<g class="sc-narration">${steps
    .map(([a, b, text], i) => {
      const last = i === steps.length - 1;
      const next = steps[i + 1]?.[0];
      const attrs = last
        ? `class="sc sc-fade" ${win(a, b)}`
        : `class="sc sc-blink" ${blink(a, b, next - 0.02, next)}`;
      const step = `<tspan class="sc-step">${String(i + 1).padStart(2, "0")}</tspan>`;
      const lines = width ? wrap(text, width - 3) : [text];
      return `<text x="${x}" y="${y}" text-anchor="${anchor}" ${attrs}>${step} ${lines[0]}${lines
        .slice(1)
        .map(
          line =>
            `<tspan x="${x + (anchor === "start" ? 22 : 0)}" dy="17">${line}</tspan>`
        )
        .join("")}</text>`;
    })
    .join("")}</g>`;
}
