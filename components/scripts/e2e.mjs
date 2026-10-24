// Renders key frames of the fframes-kit gallery through its command line with one renderer
// and compares them with odiff to gallery/snapshots. A copy of the root scripts/e2e.mjs
// reduced to the gallery. Frames and diffs of the run go to target/e2e/<renderer>.
//   node scripts/e2e.mjs cpu|skia-cpu|gpu [--update]
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { compare } from "odiff-bin";

const [renderer, flag] = process.argv.slice(2);
const update = flag === "--update";
if (!["cpu", "skia-cpu", "gpu"].includes(renderer) || (flag && !update)) {
  console.error("usage: node scripts/e2e.mjs cpu|skia-cpu|gpu [--update]");
  process.exit(1);
}

// Renderers resample images and round colours differently: odiff's colour threshold 0.15
// absorbs that, and up to 0.5% of the pixels may still differ.
const THRESHOLD = 0.15;
const MAX_DIFF_PERCENT = 0.5;

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");
const exe = process.platform === "win32" ? ".exe" : "";
const out = join(target, "e2e", renderer);

// prettier-ignore
const examples = [
  { dir: "gallery", pkg: "components-gallery", bin: "components_gallery_bin", frames: "Sequence@50%,Series@50%,FadeIn@50%,Theme@50%,SquareToCircle@45%,Write@30%,Create@50%,Flash@40%,CurvedArrow@end,Callout@end,Matchers@end,Arcs@end,Angles@end,Polygrams@end,Uncreate@90%,Fade@50%,Slide@50%,Wipe@50%,CircleWipe@1.25s,Dissolve@50%,DreamyZoom@50%,FilmBurn@50%,SwapCards@50%,Trail@50%,LightLeak@50%,ColorEffects@50%,EdgeEffects@50%,NoiseEffects@50%,OverlayEffects@50%,TextureEffects@50%,HalftoneEffects@50%,GeometryEffects@50%,BlurEffects@50%,SceneOverlap@0.3s,FitText@50%,Letters@0.3s,TextReveal@0.5s,Typewriter@0.8s,Counter@40%,Paragraph@50%,Appear@50%,LowerThird@50%,Captions@50%,CaptionStack@50%,ChapterList@60%,Annotation@end,ProgressBar@50%,Check@end,BarChart@end,SpectrumBars@50%,Waveform@50%,DrawOn@0.7s,CornerMarks@end,Glow@50%,DropShadow@50%,PhotoFrame@50%,KenBurns@end,MathTex@50%,WriteFormula@50%,FormulaParts@1.1s,TransformMatchingTex@50%,ChemEq@end,TikzDiagram@end,NumberLine@end,NumberPlane@end,SinAndCosFunctionPlot@1.2s,SinAndCosFunctionPlot@end,GraphAreaPlot@end,Canvas@end,VGroup@end,Leader@end,Sim@1s,Matrix@end,Table@end,BraceLabel@end,ValueTracker@50%,FunctionPlot@end,RiemannSum@end,UnitCircle@50%,LinearTransform@end,Epicycles@50%,Projectile@end,Pendulum@50%,Circuit@50%,BohrAtom@50%,Molecule@end,Molecule3D@end,Orbital@end,ReactionEquation@end,BondFormation@end,EnergyProfile@end,TitrationCurve@end,StatesOfMatter@3s,Equilibrium@end,SortVisualizer@50%,BinarySearch@end,GraphView@end,CodeBlock@50%,DnaHelix@50%,PunnettSquare@end,PhyloTree@end,TagsSquareToCircle@45%,TagsFadeIn@50%" },
];

const run = (command, args, cwd) =>
  spawnSync(command, args, { cwd, stdio: ["ignore", "ignore", "inherit"] })
    .status === 0;

const packages = examples.flatMap(({ pkg }) => ["-p", pkg]);
const build = ["build", "--release", "--bins", ...packages];
if (spawnSync("cargo", build, { cwd: root, stdio: "inherit" }).status !== 0)
  process.exit(1);

rmSync(out, { recursive: true, force: true });
const failed = [];
const created = [];
for (const { dir, bin, frames, skia, args = [] } of examples) {
  if (skia && renderer === "cpu") continue;
  const frameDir = join(out, dir);
  const argv = [...args, "--renderer", renderer, "--scale", "0.5"];
  argv.push("frame", frames, "-o", frameDir);
  const binary = join(target, "release", bin + exe);
  if (!run(binary, argv, join(root, dir))) {
    failed.push(`${dir}: ${frames}`);
    continue;
  }

  for (const file of readdirSync(frameDir)) {
    const name = `${dir}/${file}`;
    const actual = join(frameDir, file);
    const snapshot = join(root, dir, "snapshots", file);
    if (!existsSync(snapshot)) {
      mkdirSync(dirname(snapshot), { recursive: true });
      copyFileSync(actual, snapshot);
      created.push(name);
      continue;
    }

    const diff = actual.replace(/\.png$/, ".diff.png");
    const result = await compare(snapshot, actual, diff, {
      antialiasing: true,
      threshold: THRESHOLD,
    });
    const percent = result.match ? 0 : (result.diffPercentage ?? 100);
    console.log(`${name}: ${percent.toFixed(3)}% differ`);
    if (percent <= MAX_DIFF_PERCENT) continue;
    if (update) copyFileSync(actual, snapshot);
    else failed.push(`${name}: see ${diff}`);
  }
}

if (created.length) {
  console.error(
    `new snapshots, look at them and commit them:\n${created.join("\n")}`
  );
  if (process.env.CI) process.exit(1);
}
if (failed.length) {
  console.error(`frames differ (${renderer}):\n${failed.join("\n")}`);
  process.exit(1);
}
