import { describe, expect, it } from "vitest";

import { DEFAULT_CAPTION_STYLE } from "./overlay-model";
import { captionContentHash, wrapCaptionLines } from "./caption-renderer";

describe("caption renderer", () => {
  it("wraps paragraphs and long tokens deterministically", () => {
    const measure = (value: string) => value.length * 10;
    expect(wrapCaptionLines("Forge the moment\nNOW", 90, measure)).toEqual([
      "Forge the",
      "moment",
      "NOW",
    ]);
    expect(wrapCaptionLines("ABCDEFGHIJK", 40, measure)).toEqual([
      "ABCD",
      "EFGH",
      "IJK",
    ]);
  });

  it("hashes every render-affecting caption property", async () => {
    const first = await captionContentHash({
      ...DEFAULT_CAPTION_STYLE,
      text: "Hook",
    });
    const second = await captionContentHash({
      ...DEFAULT_CAPTION_STYLE,
      text: "Hook",
      outlineWidthPx: 7,
    });

    expect(first).toMatch(/^[a-f0-9]{64}$/);
    expect(second).not.toBe(first);
  });
});
