# YouTube Performance Integration

## Product decision

YouTube performance is an optional post-MVP workspace. It does not publish
videos, upload media, add an account requirement to editing, or make the core
workflow network-dependent.

Clip Forge cannot reliably infer which YouTube upload came from a local export.
The user must link a project to one recent upload from the connected channel or
paste a YouTube URL/video ID. A later publishing integration may record that ID
automatically, but title, filename, and upload-time guessing are not accepted.

## First performance scorecard

For each linked video, request:

- engaged views and views;
- estimated minutes watched;
- average view duration and average percentage viewed;
- likes, comments, and shares;
- subscribers gained, subscribers lost, and their net value;
- the same metrics grouped by day.

The aggregate query is authoritative for the scorecard. Daily rows are a trend
table and are not summed to recreate aggregate values because YouTube can limit
dimension rows and calculates averages at the report level.

Revenue, geography, demographics, traffic sources, search terms, retention
curves, realtime estimates, and cross-platform attribution are deferred.

## User workflow

1. Open **Channel performance**.
2. Connect a Google OAuth desktop client in the system browser.
3. Approve only `youtube.readonly` and `yt-analytics.readonly`.
4. Open the Clip Forge project that produced the uploaded video.
5. Select the upload from the channel uploads playlist or paste its URL.
6. Confirm the exact video link.
7. Refresh the project or all linked projects explicitly.
8. Review the cached scorecard and latest fourteen daily rows offline.

Newly published videos may have no analytics rows yet. This is a valid pending
state, not a failed sync.

## Native boundary

Rust owns:

- OAuth authorization-code flow with PKCE, state validation, a loopback
  callback, and a fixed three-minute timeout;
- system-browser launch for the Rust-constructed Google authorization URL;
- refresh-token storage in the operating-system credential store;
- access-token refresh and in-memory expiry;
- fixed Google OAuth, YouTube Data API, and YouTube Analytics API endpoints;
- bounded HTTP timeouts and two-MiB response bodies;
- channel ownership checks before a project/video link is accepted;
- atomic versioned local persistence for channel metadata, links, and snapshots;
- stable network, authorization, integration, and API errors.

Svelte receives no token, client secret, raw API URL, generic HTTP capability, or
arbitrary request parameters. It owns only connection controls, explicit
project/video selection, loading/empty/error states, scorecards, and tables.
While browser authorization is active, Svelte polls the native connection status
at a bounded interval and renders the native `awaitingBrowser`,
`exchangingToken`, and `loadingChannel` phases. Polling stops on success,
failure, panel teardown, or shortly after the native three-minute timeout.

## Persistence and privacy

The refresh token is stored under the service
`com.skulld.clipforge.youtube` in the operating-system credential store.
Access tokens remain in memory. OAuth client configuration comes from the
application build/runtime environment and is never written to a project.

The app-local `youtube-performance-v1.json` file contains:

- connected channel ID/title and uploads-playlist ID;
- project ID/name to YouTube video ID/title/publish-date links;
- last retrieved aggregate and daily performance.

It contains no source/output path, caption text, media, artwork, OAuth token, or
client secret. Disconnect removes the credential and local performance file.
Diagnostic bundles must exclude the file and all YouTube account/performance
data.

## Google API contract

Channel discovery:

```text
GET /youtube/v3/channels
part=snippet,contentDetails
mine=true
```

Recent uploads use
`contentDetails.relatedPlaylists.uploads` with
`playlistItems.list(part=snippet,contentDetails,maxResults=25)`. The integration
does not use `search.list`.

Link validation:

```text
GET /youtube/v3/videos
part=snippet
id={videoId}
```

The returned `snippet.channelId` must equal the connected channel ID.

Performance:

```text
GET /v2/reports
ids=channel=={channelId}
startDate={publishedDate}
endDate={yesterday}
metrics=engagedViews,views,estimatedMinutesWatched,averageViewDuration,
        averageViewPercentage,likes,comments,shares,subscribersGained,
        subscribersLost
filters=video=={videoId}
```

A second request adds `dimensions=day&sort=day`. Response columns are mapped by
the returned header names, never by an assumed position.

## Configuration and release gate

Internal development requires:

```text
SKCF_YOUTUBE_CLIENT_ID
SKCF_YOUTUBE_CLIENT_SECRET  # only when supplied by the desktop OAuth client
```

Both the YouTube Data API v3 and YouTube Analytics API must be enabled for the
Google Cloud project. The OAuth client type is **Desktop app** and loopback IP
redirects are used.

If Google reports that a service is disabled, the app identifies the API used
by the failing request: channel/video calls name YouTube Data API v3, while
performance-report calls name YouTube Analytics API.

Before public distribution:

- complete the OAuth consent-screen and sensitive-scope verification required
  for the intended user type;
- publish an accurate privacy policy and account-data deletion behavior;
- verify Windows/macOS credential-store behavior and packaged browser callback;
- review YouTube API Services Terms, quota, branding, and stored-data
  refresh/deletion obligations;
- run a real configured-channel smoke without recording channel IDs, video
  titles, metrics, tokens, or private paths in CI output.

An unconfigured build must show a compact unavailable state and keep the editor
fully functional. End-user copy must confirm that local editing and export are
unaffected. OAuth environment variables and API enablement instructions belong
behind a developer-setup disclosure, and the panel must not repeat the same
configuration problem as both an unavailable state and an error alert.
