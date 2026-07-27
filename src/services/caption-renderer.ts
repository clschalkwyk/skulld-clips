import type { CaptionStyle } from "../../contracts/types";

const RENDERER_VERSION = 1;
const OUTPUT_WIDTH = 1080;
const OUTPUT_HEIGHT = 1920;

export interface RenderedCaption {
  contentHash: string;
  pngBytesBase64: string;
  width: number;
  height: number;
}

export async function captionContentHash(style: CaptionStyle): Promise<string> {
  const payload = JSON.stringify({
    rendererVersion: RENDERER_VERSION,
    text: style.text.normalize("NFC"),
    fontFamily: style.fontFamily,
    fontSizePx: style.fontSizePx,
    fontWeight: style.fontWeight,
    align: style.align,
    lineHeight: style.lineHeight,
    maxWidthPx: style.maxWidthPx,
    fill: style.fill.toLowerCase(),
    outlineWidthPx: style.outlineWidthPx,
    outlineColor: style.outlineColor.toLowerCase(),
    backgroundEnabled: style.backgroundEnabled,
    backgroundColor: style.backgroundColor.toLowerCase(),
    paddingPx: style.paddingPx,
  });
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(payload),
  );
  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export async function renderCaption(
  style: CaptionStyle,
): Promise<RenderedCaption> {
  await document.fonts.load(
    `${style.fontWeight} ${style.fontSizePx}px "${style.fontFamily}"`,
    style.text || " ",
  );
  const measurementCanvas = document.createElement("canvas");
  const measurement = requiredContext(measurementCanvas);
  configureTextContext(measurement, style);
  const lines = wrapCaptionLines(
    style.text.normalize("NFC"),
    style.maxWidthPx,
    (text) => measurement.measureText(text).width,
  );
  const lineHeightPx = style.fontSizePx * style.lineHeight;
  const textWidth = Math.max(
    1,
    ...lines.map((line) => measurement.measureText(line).width),
  );
  const inset = style.paddingPx + style.outlineWidthPx;
  const width = Math.min(OUTPUT_WIDTH, Math.max(1, Math.ceil(textWidth + inset * 2)));
  const height = Math.max(1, Math.ceil(lines.length * lineHeightPx + inset * 2));
  if (height > OUTPUT_HEIGHT) {
    throw new Error(
      "Caption text and spacing exceed the 1920 px output height. Shorten the text or reduce its size.",
    );
  }
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = requiredContext(canvas);
  configureTextContext(context, style);
  if (style.backgroundEnabled) {
    context.fillStyle = normalizedColor(style.backgroundColor, "#000000");
    context.fillRect(0, 0, width, height);
  }

  const textX =
    style.align === "left"
      ? inset
      : style.align === "right"
        ? width - inset
        : width / 2;
  context.textAlign = style.align;
  context.textBaseline = "top";
  context.lineJoin = "round";
  context.fillStyle = normalizedColor(style.fill, "#ffffff");
  context.strokeStyle = normalizedColor(style.outlineColor, "#000000");
  context.lineWidth = style.outlineWidthPx * 2;
  lines.forEach((line, index) => {
    const y = inset + index * lineHeightPx;
    if (style.outlineWidthPx > 0) {
      context.strokeText(line, textX, y);
    }
    context.fillText(line, textX, y);
  });

  const dataUrl = canvas.toDataURL("image/png");
  const marker = "base64,";
  const markerIndex = dataUrl.indexOf(marker);
  if (markerIndex < 0) {
    throw new Error("Caption canvas did not produce a PNG payload");
  }
  return {
    contentHash: await captionContentHash(style),
    pngBytesBase64: dataUrl.slice(markerIndex + marker.length),
    width,
    height,
  };
}

export function wrapCaptionLines(
  text: string,
  maximumWidth: number,
  measure: (text: string) => number,
): string[] {
  const paragraphs = text.split(/\r?\n/);
  const lines: string[] = [];
  for (const paragraph of paragraphs) {
    const words = paragraph.trim().split(/\s+/).filter(Boolean);
    if (words.length === 0) {
      lines.push("");
      continue;
    }
    let line = "";
    for (const word of words) {
      const candidate = line ? `${line} ${word}` : word;
      if (measure(candidate) <= maximumWidth) {
        line = candidate;
        continue;
      }
      if (line) {
        lines.push(line);
      }
      if (measure(word) <= maximumWidth) {
        line = word;
        continue;
      }
      const pieces = splitLongToken(word, maximumWidth, measure);
      lines.push(...pieces.slice(0, -1));
      line = pieces.at(-1) ?? "";
    }
    lines.push(line);
  }
  return lines.length > 0 ? lines : [""];
}

function splitLongToken(
  token: string,
  maximumWidth: number,
  measure: (text: string) => number,
): string[] {
  const pieces: string[] = [];
  let piece = "";
  for (const character of token) {
    if (piece && measure(piece + character) > maximumWidth) {
      pieces.push(piece);
      piece = character;
    } else {
      piece += character;
    }
  }
  if (piece) {
    pieces.push(piece);
  }
  return pieces;
}

function configureTextContext(
  context: CanvasRenderingContext2D,
  style: CaptionStyle,
): void {
  context.font = `${style.fontWeight} ${style.fontSizePx}px "${style.fontFamily}"`;
}

function requiredContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Caption canvas rendering is unavailable");
  }
  return context;
}

function normalizedColor(value: string, fallback: string): string {
  return /^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(value) ? value : fallback;
}
