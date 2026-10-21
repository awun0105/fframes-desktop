// Studio's styles are small enough to deliver with the document. This removes
// stylesheet round trips and keeps first paint, no-JS and print fully styled.
// Edit the CSS source files, then run this script (also part of build:landing).
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

const studio = fileURLToPath(new URL("../landing/studio/", import.meta.url));
for (const [page, specific] of [
  ["index.html", "features.css"],
  ["how-it-works.html", "how-it-works.css"],
]) {
  const sources = ["home.css", specific, "readability.css"];
  const css = sources
    .map(source => readFileSync(join(studio, source), "utf8"))
    .join("\n")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter(Boolean)
    .join("\n");
  if (/<\/style/i.test(css))
    throw new Error("Unsafe closing tag in Studio CSS");
  const file = join(studio, page);
  let html = readFileSync(file, "utf8");
  html = html
    .replace(
      /\s*<!-- studio:bootstrap -->[\s\S]*?<!-- \/studio:bootstrap -->/g,
      ""
    )
    .replace(/\s*<!-- studio:styles -->[\s\S]*?<!-- \/studio:styles -->/g, "")
    .replace(
      /\s*<link\s+rel="stylesheet"\s+href="(?:home|features|how-it-works|readability)\.css(?:\?[^"]*)?"\s*\/?>/g,
      ""
    );
  // Establish the saved motion layout before first paint. The normal controller
  // still owns controls and scroll updates once the document has loaded.
  const motionClass =
    page === "index.html" ? "story-motion" : "internals-motion";
  const bootstrap = `\n    <!-- studio:bootstrap -->\n    <!-- prettier-ignore -->\n    <script data-studio-bootstrap>(()=>{const root=document.documentElement;root.classList.add("js-enabled");let choice=null;try{choice=localStorage.getItem("fframes-studio-scroll-motion")}catch{}if(choice==="on"||(choice!=="off"&&!matchMedia("(prefers-reduced-motion: reduce)").matches))root.classList.add("${motionClass}")})()</script>\n    <!-- /studio:bootstrap -->`;
  const block = `${bootstrap}\n    <!-- studio:styles -->\n    <!-- prettier-ignore -->\n    <style data-studio-styles>\n${css}\n</style>\n    <!-- /studio:styles -->`;
  const anchor = '<meta name="color-scheme" content="light" />';
  if (!html.includes(anchor))
    throw new Error(`Missing styles insertion point: ${page}`);
  html = html.replace(anchor, anchor + block);
  writeFileSync(file, html);
  console.log(
    `${page}: inline styles ${gzipSync(css).length.toLocaleString()} bytes gzipped; 0 stylesheet requests`
  );
}
