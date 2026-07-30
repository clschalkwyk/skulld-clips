export type UUID = string;
export type Milliseconds = number;

export interface NormalizedRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SourceFingerprint {
  sizeBytes: number;
  modifiedAtMs: number;
  firstChunkSha256: string;
  lastChunkSha256: string | null;
}

export interface VideoProbe {
  streamIndex: number;
  codec: string;
  rawWidth: number;
  rawHeight: number;
  displayWidth: number;
  displayHeight: number;
  rotationDegrees: 0 | 90 | 180 | 270;
  avgFrameRate: number | null;
  realFrameRate: number | null;
  pixelFormat: string | null;
  sampleAspectRatio: string | null;
}

export interface AudioProbe {
  streamIndex: number;
  codec: string;
  sampleRate: number | null;
  channels: number | null;
  channelLayout: string | null;
}

export interface MediaProbe {
  durationMs: Milliseconds;
  containerName: string;
  fileSizeBytes: number;
  video: VideoProbe;
  hasAudio: boolean;
  audio: AudioProbe | null;
  warnings: string[];
}

export type ClipEventKind = "completion" | "death" | "bossEncounter";

export interface ClipCandidate {
  id: UUID;
  kind: ClipEventKind;
  eventMs: Milliseconds;
  detectedStartMs: Milliseconds;
  detectedEndMs: Milliseconds;
  suggestedInMs: Milliseconds;
  suggestedOutMs: Milliseconds;
  confidence: number;
  evidence: string[];
}

export type YouTubePostFormat = "short" | "video";

export type YouTubePostMomentType =
  | ClipEventKind
  | "buildShowcase"
  | "gameplayHighlight"
  | "guide";

export interface YouTubePostBrief {
  game: string;
  format: YouTubePostFormat;
  momentType: YouTubePostMomentType;
  contentSummary: string;
  primarySearchPhrase: string;
  supportingKeywords: string;
  callToAction: string;
}

export interface YouTubeTitleOption {
  id: "searchFirst" | "hookFirst" | "momentFirst";
  label: string;
  title: string;
}

export interface YouTubePostDraft {
  titleOptions: YouTubeTitleOption[];
  title: string;
  description: string;
  hashtags: string[];
}

export interface YouTubePostChecks {
  titleCharacters: number;
  descriptionCharacters: number;
  titleWithinLimit: boolean;
  descriptionWithinLimit: boolean;
  searchPhraseInTitle: boolean;
  searchPhraseInOpeningDescription: boolean;
  hashtagCount: number;
}

export type AiPostProvider = "openai" | "openrouter";
export type YouTubePostGenerationSource = "local" | AiPostProvider;

export interface AiProviderCredentialStatus {
  provider: AiPostProvider;
  configured: boolean;
}

export interface AiModelOption {
  provider: AiPostProvider;
  id: string;
  name: string;
  contextLength: number | null;
}

export type ClipAnalysisEvent =
  | {
      event: "clip-analysis://progress";
      jobId: UUID;
      progress: number;
      analyzedMs: Milliseconds;
      totalMs: Milliseconds;
    }
  | {
      event: "clip-analysis://completed";
      jobId: UUID;
      candidates: ClipCandidate[];
    }
  | {
      event: "clip-analysis://failed";
      jobId: UUID;
      error: AppError;
    }
  | {
      event: "clip-analysis://cancelled";
      jobId: UUID;
    };

export interface AssetRef {
  relativePath: string;
  sha256: string;
  width: number;
  height: number;
  mimeType: string;
  originalFilename: string | null;
}

export interface StingAssetRef extends AssetRef {
  durationMs: Milliseconds;
  hasAudio: boolean;
  preview: StingPreviewRef;
}

export interface StingPreviewRef {
  relativePath: string;
  sha256: string;
  width: number;
  height: number;
  frameWidth: 192;
  frameHeight: 192;
  columns: number;
  rows: number;
  frameCount: number;
  framesPerSecond: 12;
}

export interface OverlayBase {
  id: UUID;
  name: string;
  position: NormalizedRect;
  opacity: number;
  startMs: Milliseconds;
  endMs: Milliseconds;
  zIndex: number;
}

export interface ImageOverlay extends OverlayBase {
  type: "image";
  asset: AssetRef;
}

export interface CaptionStyle {
  text: string;
  fontFamily: string;
  fontSizePx: number;
  fontWeight: number;
  align: "left" | "center" | "right";
  lineHeight: number;
  maxWidthPx: number;
  fill: string;
  outlineWidthPx: number;
  outlineColor: string;
  backgroundEnabled: boolean;
  backgroundColor: string;
  paddingPx: number;
}

export interface CaptionOverlay extends OverlayBase {
  type: "caption";
  caption: CaptionStyle;
  generatedAsset: AssetRef;
}

export interface StingOverlay extends OverlayBase {
  type: "sting";
  asset: StingAssetRef;
  preset: "toasty-right";
  includeAudio: boolean;
  playbackRate?: 1 | 2 | 3;
  repeat?: boolean;
}

export type Overlay = ImageOverlay | CaptionOverlay | StingOverlay;

export interface ExportSettings {
  presetId: "vertical-generic" | "youtube-shorts" | "instagram-reels" | "tiktok";
  qualityMode: "draft" | "balanced" | "high";
  frameRateMode: "source-capped-60" | "30" | "60";
  videoCodec: "h264";
  pixelFormat: "yuv420p";
  audioCodec: "aac";
  audioBitrateKbps: 128 | 160 | 192 | 256;
}

export interface ProjectV1 {
  schemaVersion: 1;
  id: UUID;
  name: string;
  createdAt: string;
  updatedAt: string;
  source: {
    path: string;
    filename: string;
    fingerprint: SourceFingerprint;
    probe: MediaProbe;
  };
  timeline: { inMs: Milliseconds; outMs: Milliseconds };
  canvas: { width: 1080; height: 1920; background: "crop-fill" };
  crop: NormalizedRect;
  overlays: Overlay[];
  exportDefaults: ExportSettings;
}

export type SourceStatus = "ok" | "missing" | "changed";

export interface CreateProjectResult {
  projectPath: string;
  project: ProjectV1;
}

export interface LoadProjectResult {
  projectPath: string;
  project: ProjectV1;
  sourceStatus: SourceStatus;
  migrationApplied: boolean;
}

export interface SaveProjectResult {
  savedAt: string;
  projectSha256: string;
}

export interface RelinkSourceResult {
  project: ProjectV1;
  fingerprintMatched: boolean;
}

export interface RecentProject {
  projectPath: string;
  name: string;
  sourceFilename: string;
  lastOpenedAt: string;
  sourceStatus: SourceStatus;
  durationMs: Milliseconds;
}

export type AppErrorCode =
  | "E_INVALID_ARGUMENT"
  | "E_MEDIA_UNSUPPORTED"
  | "E_SOURCE_MISSING"
  | "E_SOURCE_CHANGED"
  | "E_PROJECT_SCHEMA"
  | "E_ASSET_MISSING"
  | "E_DESTINATION_DENIED"
  | "E_OUTPUT_EXISTS"
  | "E_DISK_SPACE"
  | "E_FFPROBE_FAILED"
  | "E_FFMPEG_FAILED"
  | "E_EXPORT_ACTIVE"
  | "E_EXPORT_NOT_FOUND"
  | "E_EXPORT_CANCELLED"
  | "E_ANALYSIS_ACTIVE"
  | "E_ANALYSIS_NOT_FOUND"
  | "E_ANALYSIS_FAILED"
  | "E_INTEGRATION_UNAVAILABLE"
  | "E_AUTH_REQUIRED"
  | "E_NETWORK"
  | "E_YOUTUBE_API"
  | "E_AI_PROVIDER_AUTH"
  | "E_AI_PROVIDER_API"
  | "E_IO"
  | "E_INTERNAL";

export interface AppError {
  code: AppErrorCode;
  message: string;
  safeDetail: string | null;
  retryable: boolean;
}

export interface ExportRequest {
  projectPath: string;
  projectSnapshot: ProjectV1;
  destinationPath: string;
  overwrite: boolean;
  settings: ExportSettings;
}

export interface ExportValidation {
  valid: boolean;
  errors: AppError[];
  warnings: string[];
  estimatedBytes: number | null;
  freeBytes: number | null;
}

export interface YouTubeChannel {
  channelId: string;
  title: string;
}

export type YouTubeConnectionPhase =
  | "disconnected"
  | "awaitingBrowser"
  | "exchangingToken"
  | "loadingChannel"
  | "connected"
  | "failed";

export interface YouTubeConnectionStatus {
  configured: boolean;
  authenticated: boolean;
  connectionPhase: YouTubeConnectionPhase;
  channel: YouTubeChannel | null;
  lastSyncedAt: string | null;
}

export interface YouTubeVideoCandidate {
  videoId: string;
  title: string;
  publishedAt: string;
}

export interface YouTubePerformanceMetrics {
  engagedViews: number;
  views: number;
  estimatedMinutesWatched: number;
  averageViewDurationSeconds: number;
  averageViewPercentage: number;
  likes: number;
  comments: number;
  shares: number;
  subscribersGained: number;
  subscribersLost: number;
}

export interface YouTubeDailyPerformance {
  date: string;
  metrics: YouTubePerformanceMetrics;
}

export interface YouTubePerformanceSnapshot {
  startDate: string;
  endDate: string;
  syncedAt: string;
  metrics: YouTubePerformanceMetrics;
  daily: YouTubeDailyPerformance[];
}

export interface YouTubeProjectPerformance {
  projectId: UUID;
  projectName: string;
  videoId: string;
  videoTitle: string;
  publishedAt: string;
  linkedAt: string;
  performance: YouTubePerformanceSnapshot | null;
}

export type ExportEvent =
  | {
      event: "export://progress";
      jobId: UUID;
      phase: "preparing-assets" | "encoding" | "verifying";
      progress: number;
      encodedMs: number;
      totalMs: number;
      fps: number | null;
      speed: number | null;
      outputBytes: number | null;
    }
  | {
      event: "export://completed";
      jobId: UUID;
      outputPath: string;
      durationMs: number;
      sizeBytes: number;
    }
  | { event: "export://failed"; jobId: UUID; error: AppError }
  | { event: "export://cancelled"; jobId: UUID };
