import type {
  ClipEventKind,
  ExportSettings,
  YouTubePostBrief,
  YouTubePostChecks,
  YouTubePostDraft,
  YouTubePostMomentType,
  YouTubeTitleOption,
} from "../../contracts/types";

export const YOUTUBE_TITLE_LIMIT = 100;
export const YOUTUBE_DESCRIPTION_LIMIT = 5_000;
export const YOUTUBE_HASHTAG_LIMIT = 3;
const GENERATED_TITLE_TARGET = 72;

export type YouTubePostBriefErrors = Partial<
  Record<keyof YouTubePostBrief, string>
>;

interface DefaultYouTubePostBriefOptions {
  projectName: string;
  sourceFilename: string;
  captionText: string | null;
  detectedMomentKind: ClipEventKind | null;
  exportPreset: ExportSettings["presetId"];
}

const MOMENT_LABELS: Record<YouTubePostMomentType, string> = {
  completion: "Dungeon completion",
  death: "Death moment",
  bossEncounter: "Boss fight",
  buildShowcase: "Build showcase",
  gameplayHighlight: "Gameplay highlight",
  guide: "Quick guide",
};

const SEARCH_SUFFIXES: Record<YouTubePostMomentType, string> = {
  completion: "dungeon completion",
  death: "death clip",
  bossEncounter: "boss fight",
  buildShowcase: "build showcase",
  gameplayHighlight: "gameplay",
  guide: "guide",
};

export function createDefaultYouTubePostBrief({
  projectName,
  sourceFilename,
  captionText,
  detectedMomentKind,
  exportPreset,
}: DefaultYouTubePostBriefOptions): YouTubePostBrief {
  const game = "Diablo IV";
  const momentType = detectedMomentKind ?? "gameplayHighlight";
  return {
    game,
    format: exportPreset === "youtube-shorts" ? "short" : "video",
    momentType,
    contentSummary: firstUsefulContext(
      captionText ?? "",
      projectName,
      stripFilenameExtension(sourceFilename),
    ),
    primarySearchPhrase: defaultYouTubeSearchPhrase(game, momentType),
    supportingKeywords: "",
    callToAction:
      "Subscribe for more Diablo IV highlights, boss fights, and build moments.",
  };
}

export function defaultYouTubeSearchPhrase(
  game: string,
  momentType: YouTubePostMomentType,
): string {
  const searchableGame =
    cleanInline(game).toLocaleLowerCase() === "diablo iv"
      ? "Diablo 4"
      : cleanInline(game);
  return `${searchableGame} ${SEARCH_SUFFIXES[momentType]}`.trim();
}

export function validateYouTubePostBrief(
  brief: YouTubePostBrief,
): YouTubePostBriefErrors {
  const errors: YouTubePostBriefErrors = {};
  const game = cleanInline(brief.game);
  const contentSummary = cleanInline(brief.contentSummary);
  const primarySearchPhrase = cleanInline(brief.primarySearchPhrase);

  if (game.length < 2) {
    errors.game = "Name the game shown in the clip.";
  } else if (game.length > 60) {
    errors.game = "Keep the game name to 60 characters or fewer.";
  }
  if (contentSummary.length < 10) {
    errors.contentSummary =
      "Describe what actually happens in at least 10 characters.";
  } else if (contentSummary.length > 280) {
    errors.contentSummary = "Keep the content summary to 280 characters or fewer.";
  }
  if (primarySearchPhrase.length < 3) {
    errors.primarySearchPhrase =
      "Add the main phrase a viewer would search for.";
  } else if (primarySearchPhrase.length > 80) {
    errors.primarySearchPhrase =
      "Keep the primary search phrase to 80 characters or fewer.";
  }
  if (brief.supportingKeywords.length > 240) {
    errors.supportingKeywords =
      "Keep supporting keywords to 240 characters or fewer.";
  }
  if (brief.callToAction.length > 240) {
    errors.callToAction = "Keep the call to action to 240 characters or fewer.";
  }
  return errors;
}

export function generateYouTubePost(brief: YouTubePostBrief): YouTubePostDraft {
  const errors = validateYouTubePostBrief(brief);
  if (Object.keys(errors).length > 0) {
    throw new Error("The YouTube content brief is incomplete.");
  }

  const game = cleanInline(brief.game);
  const summary = stripTrailingPunctuation(cleanInline(brief.contentSummary));
  const primarySearchPhrase = stripTrailingPunctuation(
    cleanInline(brief.primarySearchPhrase),
  );
  const momentLabel = MOMENT_LABELS[brief.momentType];
  const titleOptions: YouTubeTitleOption[] = [
    {
      id: "searchFirst",
      label: "Search-first",
      title: fitWithPrefix(primarySearchPhrase, summary, ": "),
    },
    {
      id: "hookFirst",
      label: "Hook-first",
      title: fitWithSuffix(summary, primarySearchPhrase, " | "),
    },
    {
      id: "momentFirst",
      label: "Moment-first",
      title: fitWithSuffix(
        `${momentLabel}: ${summary}`,
        primarySearchPhrase,
        " | ",
      ),
    },
  ];
  const selectedTitle = titleOptions[0]?.title ?? "";
  const supportingKeywords = parseKeywords(brief.supportingKeywords);
  const hashtags = buildHashtags(
    game,
    brief.momentType,
    brief.format,
    primarySearchPhrase,
  );
  const callToAction =
    stripTrailingPunctuation(cleanInline(brief.callToAction)) ||
    `Subscribe for more ${game} gameplay`;
  const opening = `${primarySearchPhrase}: ${summary}.`;
  const formatLine =
    brief.format === "short"
      ? `This YouTube Short captures a ${momentLabel.toLocaleLowerCase()} in ${game}.`
      : `This video captures a ${momentLabel.toLocaleLowerCase()} in ${game}.`;
  const keywordLine =
    supportingKeywords.length > 0
      ? `The clip features ${joinNaturally(supportingKeywords)}.`
      : null;
  const description = [
    opening,
    formatLine,
    keywordLine,
    `${callToAction}.`,
    hashtags.join(" "),
  ]
    .filter((line): line is string => Boolean(line))
    .join("\n\n");

  return {
    titleOptions,
    title: selectedTitle,
    description,
    hashtags,
  };
}

export function analyzeYouTubePost(
  title: string,
  description: string,
  primarySearchPhrase: string,
): YouTubePostChecks {
  const normalizedPhrase = normalizeForMatch(primarySearchPhrase);
  return {
    titleCharacters: [...title].length,
    descriptionCharacters: [...description].length,
    titleWithinLimit: [...title].length <= YOUTUBE_TITLE_LIMIT,
    descriptionWithinLimit:
      [...description].length <= YOUTUBE_DESCRIPTION_LIMIT,
    searchPhraseInTitle: normalizeForMatch(title).includes(normalizedPhrase),
    searchPhraseInOpeningDescription: normalizeForMatch(
      description.slice(0, 240),
    ).includes(normalizedPhrase),
    hashtagCount: description.match(/(?:^|\s)#[\p{L}\p{N}_]+/gu)?.length ?? 0,
  };
}

function firstUsefulContext(...values: string[]): string {
  return (
    values
      .map(cleanInline)
      .find((value) => value.length >= 10 && !looksMachineGenerated(value)) ?? ""
  );
}

function looksMachineGenerated(value: string): boolean {
  return /^[\da-f]+(?:-[\da-f]+){3,}$/iu.test(value.replace(/\s+/g, ""));
}

function stripFilenameExtension(value: string): string {
  return value.replace(/\.[a-z0-9]{2,5}$/iu, "");
}

function cleanInline(value: string): string {
  return value.replace(/\p{Cc}+/gu, " ").replace(/\s+/gu, " ").trim();
}

function stripTrailingPunctuation(value: string): string {
  return value.replace(/[\s.!?,:;|—-]+$/gu, "").trim();
}

function truncateAtWord(value: string, maximum: number): string {
  if ([...value].length <= maximum) {
    return value;
  }
  const clipped = [...value].slice(0, Math.max(1, maximum - 1)).join("");
  const lastSpace = clipped.lastIndexOf(" ");
  const safe = lastSpace >= Math.floor(maximum * 0.55)
    ? clipped.slice(0, lastSpace)
    : clipped;
  return `${safe.trimEnd()}…`;
}

function fitWithPrefix(
  preservedPrefix: string,
  value: string,
  separator: string,
): string {
  const available =
    GENERATED_TITLE_TARGET -
    [...preservedPrefix].length -
    [...separator].length;
  if (available < 8) {
    return truncateAtWord(
      `${preservedPrefix}${separator}${value}`,
      GENERATED_TITLE_TARGET,
    );
  }
  return `${preservedPrefix}${separator}${truncateAtWord(value, available)}`;
}

function fitWithSuffix(
  value: string,
  preservedSuffix: string,
  separator: string,
): string {
  const available =
    GENERATED_TITLE_TARGET -
    [...preservedSuffix].length -
    [...separator].length;
  if (available < 8) {
    return truncateAtWord(
      `${value}${separator}${preservedSuffix}`,
      GENERATED_TITLE_TARGET,
    );
  }
  return `${truncateAtWord(value, available)}${separator}${preservedSuffix}`;
}

function parseKeywords(value: string): string[] {
  const seen = new Set<string>();
  return value
    .split(",")
    .map(cleanInline)
    .filter((keyword) => {
      const normalized = keyword.toLocaleLowerCase();
      if (!keyword || seen.has(normalized)) {
        return false;
      }
      seen.add(normalized);
      return true;
    })
    .slice(0, 5);
}

function joinNaturally(values: string[]): string {
  if (values.length <= 1) {
    return values[0] ?? "";
  }
  if (values.length === 2) {
    return `${values[0]} and ${values[1]}`;
  }
  return `${values.slice(0, -1).join(", ")}, and ${values.at(-1)}`;
}

function buildHashtags(
  game: string,
  momentType: YouTubePostMomentType,
  format: YouTubePostBrief["format"],
  primarySearchPhrase: string,
): string[] {
  const candidates = [
    hashtag(game),
    hashtag(MOMENT_LABELS[momentType]),
    format === "short" ? "#Shorts" : hashtag(primarySearchPhrase),
  ];
  const seen = new Set<string>();
  return candidates
    .filter((candidate) => candidate.length > 1)
    .filter((candidate) => {
      const normalized = candidate.toLocaleLowerCase();
      if (seen.has(normalized)) {
        return false;
      }
      seen.add(normalized);
      return true;
    })
    .slice(0, YOUTUBE_HASHTAG_LIMIT);
}

function hashtag(value: string): string {
  const words = cleanInline(value)
    .split(/[^\p{L}\p{N}]+/gu)
    .filter(Boolean);
  const body = words
    .map((word) => {
      const [first = "", ...rest] = [...word];
      return `${first.toLocaleUpperCase()}${rest.join("")}`;
    })
    .join("")
    .slice(0, 36);
  return `#${body}`;
}

function normalizeForMatch(value: string): string {
  return cleanInline(value).toLocaleLowerCase();
}
