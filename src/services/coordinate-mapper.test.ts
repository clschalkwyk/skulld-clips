import { describe, expect, it } from "vitest";

import {
  fitSourceInsideStage,
  normalizedRectToSourcePixels,
  normalizedRectToStage,
  stagePointToNormalized,
} from "./coordinate-mapper";

describe("coordinate mapper", () => {
  it("fills a stage that matches the display-oriented source aspect ratio", () => {
    expect(
      fitSourceInsideStage(
        { width: 1920, height: 1080 },
        { width: 640, height: 360 },
      ),
    ).toEqual({
      x: 0,
      y: 0,
      width: 640,
      height: 360,
    });
  });

  it("letterboxes a landscape source and maps a normalized crop into the stage", () => {
    const source = { width: 1920, height: 1080 };
    const stage = { width: 500, height: 500 };
    const fitted = fitSourceInsideStage(source, stage);

    expect(fitted).toEqual({
      x: 0,
      y: 109.375,
      width: 500,
      height: 281.25,
    });
    expect(
      normalizedRectToStage(
        { x: 0.341797, y: 0, width: 0.316406, height: 1 },
        source,
        stage,
      ),
    ).toEqual({
      x: 170.8985,
      y: 109.375,
      width: 158.203,
      height: 281.25,
    });
  });

  it("maps clamped stage points back to display-oriented normalized coordinates", () => {
    const source = { width: 1080, height: 1920 };
    const stage = { width: 800, height: 400 };

    expect(stagePointToNormalized({ x: 400, y: 200 }, source, stage)).toEqual({
      x: 0.5,
      y: 0.5,
    });
    expect(stagePointToNormalized({ x: 0, y: 900 }, source, stage)).toEqual({
      x: 0,
      y: 1,
    });
  });

  it("uses display-oriented source dimensions for pixel conversion", () => {
    expect(
      normalizedRectToSourcePixels(
        { x: 0.1, y: 0.2, width: 0.5, height: 0.4 },
        { width: 1080, height: 1920 },
      ),
    ).toEqual({ x: 108, y: 384, width: 540, height: 768 });
  });
});
