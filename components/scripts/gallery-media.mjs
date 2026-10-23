// Writes the gallery's generated media: a 3 s tone pattern (tones.wav) and a small
// gradient photo (photo.png). Usage: node scripts/gallery-media.mjs gallery/media
import fs from "node:fs";
import path from "node:path";
import zlib from "node:zlib";

const dir = process.argv[2];

// 3 s mono 16-bit 22050 Hz: four notes, 16 harmonics each, with a beat, so the spectrum moves.
const rate = 22050;
const seconds = 3;
const n = rate * seconds;
const pcm = Buffer.alloc(n * 2);
const notes = [220, 330, 440, 660];
for (let i = 0; i < n; i++) {
  const t = i / rate;
  const f = notes[Math.floor(t / 0.75) % notes.length];
  const beat = Math.exp(-((t % 0.375) * 8));
  let s = 0;
  for (let k = 1; k <= 16; k++) s += Math.sin(2 * Math.PI * f * k * t + k) / k ** 0.8;
  s *= 0.25;
  const v = Math.max(-1, Math.min(1, s * (0.3 + 0.7 * beat) * 0.8));
  pcm.writeInt16LE(Math.round(v * 32767), i * 2);
}
const header = Buffer.alloc(44);
header.write("RIFF", 0);
header.writeUInt32LE(36 + pcm.length, 4);
header.write("WAVE", 8);
header.write("fmt ", 12);
header.writeUInt32LE(16, 16);
header.writeUInt16LE(1, 20);
header.writeUInt16LE(1, 22);
header.writeUInt32LE(rate, 24);
header.writeUInt32LE(rate * 2, 28);
header.writeUInt16LE(2, 32);
header.writeUInt16LE(16, 34);
header.write("data", 36);
header.writeUInt32LE(pcm.length, 40);
fs.writeFileSync(path.join(dir, "tones.wav"), Buffer.concat([header, pcm]));

// 480x320 RGB: a sky gradient with a sun and two hills, enough to see a push-in.
const w = 480;
const h = 320;
const raw = Buffer.alloc((w * 3 + 1) * h);
for (let y = 0; y < h; y++) {
  raw[y * (w * 3 + 1)] = 0;
  for (let x = 0; x < w; x++) {
    let r = 40 + (y / h) * 180;
    let g = 70 + (y / h) * 90;
    let b = 160 - (y / h) * 60;
    if ((x - 330) ** 2 + (y - 110) ** 2 < 40 ** 2) [r, g, b] = [255, 214, 120];
    const hill1 = 230 + 30 * Math.sin(x / 70);
    const hill2 = 260 + 20 * Math.sin(x / 45 + 1);
    if (y > hill1) [r, g, b] = [40, 110, 70];
    if (y > hill2) [r, g, b] = [25, 80, 50];
    const o = y * (w * 3 + 1) + 1 + x * 3;
    raw[o] = r;
    raw[o + 1] = g;
    raw[o + 2] = b;
  }
}
const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (buf) => {
  let c = 0xffffffff;
  for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const sum = Buffer.alloc(4);
  sum.writeUInt32BE(crc(body));
  return Buffer.concat([len, body, sum]);
};
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(w, 0);
ihdr.writeUInt32BE(h, 4);
ihdr[8] = 8;
ihdr[9] = 2;
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);
fs.writeFileSync(path.join(dir, "photo.png"), png);
