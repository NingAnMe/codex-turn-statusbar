import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { deflateSync } from "node:zlib";

const root = new URL("..", import.meta.url).pathname;
const output = join(root, "src-tauri", "icons", "icon.png");
const icoOutput = join(root, "src-tauri", "icons", "icon.ico");

mkdirSync(dirname(output), { recursive: true });
const iconPng = png(128, 128);
writeFileSync(output, iconPng);
writeFileSync(icoOutput, ico(iconPng, 128, 128));
console.log(output);

function png(width, height) {
  const rows = [];
  for (let y = 0; y < height; y += 1) {
    const row = Buffer.alloc(1 + width * 4);
    row[0] = 0;
    for (let x = 0; x < width; x += 1) {
      const index = 1 + x * 4;
      const dx = x - width / 2;
      const dy = y - height / 2;
      const inCircle = Math.sqrt(dx * dx + dy * dy) <= 44;
      const inBubble =
        x >= 34 && x <= 94 && y >= 38 && y <= 78;
      const inTail =
        x >= 45 && x <= 58 && y >= 78 && y <= 94 && x - 45 <= 94 - y;
      const inCheckA =
        x >= 55 && x <= 64 && y >= 57 && y <= 66 && y - 57 >= x - 60;
      const inCheckB =
        x >= 63 && x <= 78 && y >= 48 && y <= 66 && y - 48 <= 78 - x;

      if (inBubble || inTail || inCircle) {
        row[index] = 52;
        row[index + 1] = 199;
        row[index + 2] = 89;
        row[index + 3] = 255;
      }

      if (inCheckA || inCheckB) {
        row[index] = 255;
        row[index + 1] = 255;
        row[index + 2] = 255;
        row[index + 3] = 255;
      }
    }
    rows.push(row);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", concatUInt32(width, height, Buffer.from([8, 6, 0, 0, 0]))),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function concatUInt32(width, height, rest) {
  const buffer = Buffer.alloc(8);
  buffer.writeUInt32BE(width, 0);
  buffer.writeUInt32BE(height, 4);
  return Buffer.concat([buffer, rest]);
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])), 0);
  return Buffer.concat([length, typeBuffer, data, crc]);
}

function ico(pngBuffer, width, height) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(1, 4);

  const entry = Buffer.alloc(16);
  entry[0] = width >= 256 ? 0 : width;
  entry[1] = height >= 256 ? 0 : height;
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4);
  entry.writeUInt16LE(32, 6);
  entry.writeUInt32LE(pngBuffer.length, 8);
  entry.writeUInt32LE(22, 12);

  return Buffer.concat([header, entry, pngBuffer]);
}

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      const mask = -(crc & 1);
      crc = (crc >>> 1) ^ (0xedb88320 & mask);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}
