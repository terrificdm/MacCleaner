// Renders src/assets/logo.svg to src-tauri/icons/source.png (1024x1024).
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync } from "node:fs";

const svg = readFileSync(new URL("../src/assets/logo.svg", import.meta.url), "utf8");
const png = new Resvg(svg, { fitTo: { mode: "width", value: 1024 } }).render().asPng();
writeFileSync(new URL("../src-tauri/icons/source.png", import.meta.url), png);
console.log(`source.png: ${png.length} bytes`);
