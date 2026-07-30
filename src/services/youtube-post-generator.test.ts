import { describe, expect, it } from "vitest";

import type { YouTubePostBrief } from "../../contracts/types";
import {
  analyzeYouTubePost,
  createDefaultYouTubePostBrief,
  generateYouTubePost,
  validateYouTubePostBrief,
  YOUTUBE_DESCRIPTION_LIMIT,
  YOUTUBE_HASHTAG_LIMIT,
  YOUTUBE_TITLE_LIMIT,
} from "./youtube-post-generator";

const brief: YouTubePostBrief = {
  game: "Diablo IV",
  format: "short",
  momentType: "bossEncounter",
  contentSummary:
    "The Butcher ambushed my Whirlwind Barbarian and the fight came down to the final hit",
  primarySearchPhrase: "Diablo 4 Butcher fight",
  supportingKeywords: "Whirlwind Barbarian, Season 14, boss encounter",
  callToAction: "Subscribe for more Diablo IV build moments",
};

describe("YouTube post generator", () => {
  it("uses real project content before generic defaults", () => {
    expect(
      createDefaultYouTubePostBrief({
        projectName: "A useful project title",
        sourceFilename: "capture-2026.mp4",
        captionText: "The Butcher found the wrong Barbarian",
        detectedMomentKind: "bossEncounter",
        exportPreset: "youtube-shorts",
      }),
    ).toMatchObject({
      contentSummary: "The Butcher found the wrong Barbarian",
      momentType: "bossEncounter",
      primarySearchPhrase: "Diablo 4 boss fight",
      format: "short",
    });
  });

  it("does not turn machine-generated filenames into a content claim", () => {
    expect(
      createDefaultYouTubePostBrief({
        projectName:
          "2828028816-194602375-2c9ff081-08be-479b-aaac-1a6126d92e86",
        sourceFilename:
          "2828028816-194602375-2c9ff081-08be-479b-aaac-1a6126d92e86.mp4",
        captionText: null,
        detectedMomentKind: null,
        exportPreset: "vertical-generic",
      }).contentSummary,
    ).toBe("");
  });

  it("generates three bounded titles and a human-readable SEO description", () => {
    const draft = generateYouTubePost(brief);
    expect(draft.titleOptions).toHaveLength(3);
    expect(new Set(draft.titleOptions.map((option) => option.title)).size).toBe(
      3,
    );
    for (const option of draft.titleOptions) {
      expect([...option.title].length).toBeLessThanOrEqual(72);
      expect([...option.title].length).toBeLessThanOrEqual(YOUTUBE_TITLE_LIMIT);
      expect(option.title.toLocaleLowerCase()).toContain(
        brief.primarySearchPhrase.toLocaleLowerCase(),
      );
    }
    expect([...draft.description].length).toBeLessThanOrEqual(
      YOUTUBE_DESCRIPTION_LIMIT,
    );
    expect(draft.description.startsWith(brief.primarySearchPhrase)).toBe(true);
    expect(draft.hashtags.length).toBeLessThanOrEqual(YOUTUBE_HASHTAG_LIMIT);
    expect(draft.description).toContain("#BossFight");
    expect(draft.description).toContain("#Shorts");
  });

  it("reports metadata checks after the creator edits generated copy", () => {
    const draft = generateYouTubePost(brief);
    expect(
      analyzeYouTubePost(
        draft.title,
        draft.description,
        brief.primarySearchPhrase,
      ),
    ).toMatchObject({
      titleWithinLimit: true,
      descriptionWithinLimit: true,
      searchPhraseInTitle: true,
      searchPhraseInOpeningDescription: true,
      hashtagCount: 3,
    });
  });

  it("requires an honest content summary and primary search phrase", () => {
    expect(
      validateYouTubePostBrief({
        ...brief,
        contentSummary: "",
        primarySearchPhrase: "",
      }),
    ).toMatchObject({
      contentSummary: expect.any(String),
      primarySearchPhrase: expect.any(String),
    });
  });
});
