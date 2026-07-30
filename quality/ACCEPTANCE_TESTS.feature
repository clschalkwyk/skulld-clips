Feature: Build a branded vertical gameplay clip

  Background:
    Given Skull’d Clip Forge is running offline
    And no export job is active

  Scenario: Happy path with caption and logo
    Given I import a supported landscape gameplay clip with audio
    When I set the in point to 5 seconds
    And I set the out point to 20 seconds
    And I position the locked 9:16 crop
    And I add a caption for the full range
    And I add a PNG logo for the full range
    And I export using Balanced quality
    Then an MP4 is created at the selected destination
    And its dimensions are exactly 1080 by 1920
    And it contains H.264-compatible video
    And its pixel format is yuv420p-compatible
    And it contains AAC-compatible audio
    And its duration is within one frame plus 20 milliseconds of 15 seconds
    And the caption and logo match their previewed positions

  Scenario: Silent source
    Given I import a supported clip with no audio
    When I export a valid selection
    Then export succeeds
    And a video stream exists
    And absence of audio is accepted

  Scenario: Constrained Skull'd stings
    Given I import a valid green-screen MP4 sting
    And I place a 1 times repeating sting inside the active range
    And I duplicate it as a 2 times sting that plays once
    And selecting either sting reveals a settled visible frame
    And I can anchor, nudge, resize, or drag either within the output bounds
    When I export using the Toasty-right preset with sting audio enabled
    Then both stings enter from the right at their selected source speeds
    And the repeating sting fills its selected duration
    And their green backgrounds are keyed out
    And their audio is mixed without clipping
    And no frozen sting frame remains after either end time

  Scenario: Preview uses the available editor space efficiently
    Given I opened a project with a verified source
    When the editor inspector is taller than the source preview
    Then the stage matches the display-oriented source aspect ratio
    And the stage fits inside the available canvas row
    And the inspector scrolls without pushing the timeline below the window

  Scenario: Canvas dock keeps overlay work close to the preview
    Given I opened a project with a sting inside the active range
    When I select the sting in the compact layer rail
    Then the preview uses the dominant editor width
    And the timeline starts after the layer rail below the preview and dock
    And the dock shows nine placement anchors and a size control
    And the dock shows the sting start offset and duration relative to clip in
    And the dock shows speed, once or repeat, and editable duration
    And the dock shows the current playhead and animation cycle count
    And I can choose full animation, two loops, or fill remaining
    When I choose Insert Sting Here at the current playhead
    Then a new sting instance starts there without moving the existing sting
    And an insertion within the final 500 milliseconds clamps safely inside the range
    When I set the sting start and end from the playhead
    Then its integer millisecond timing remains within the active range
    And exact placement and timing values remain available under advanced controls

  Scenario: Cancel
    Given a valid export is running
    When I cancel it
    Then the FFmpeg process tree terminates
    And the job becomes cancelled
    And no final file is reported
    And the partial file is removed
    And another export can start

  Scenario: Invalid trim
    Given I imported a clip
    When the in point is greater than or equal to the out point
    Then validation returns E_INVALID_ARGUMENT
    And FFmpeg is not started

  Scenario: Existing output
    Given a file exists at the output path
    When I validate without overwrite permission
    Then validation returns E_OUTPUT_EXISTS
    And the existing file is unchanged

  Scenario: Reopen project
    Given I saved a project with trim crop and overlays
    When I restart the application
    And I open the project
    Then all edits are restored
    And source status is ok

  Scenario: Relink moved source
    Given a project source moved
    When I open the project
    Then source status is missing
    When I choose the moved file with a matching fingerprint
    Then it is relinked
    And edits are unchanged

  Scenario: Changed source
    Given the source path points to different content
    When I open the project
    Then source status is changed
    And I must explicitly accept mismatch or choose the correct file

  Scenario: Special-character paths
    Given source and destination paths contain spaces apostrophes and Unicode
    When I export
    Then export succeeds
    And no shell interpretation occurs

  Scenario: Rotated source
    Given source orientation is stored as rotation metadata
    When I crop and export
    Then crop coordinates use display orientation
    And the output matches the preview orientation

  Scenario: Variable-frame-rate source
    Given a variable-frame-rate gameplay clip
    When I export at source-capped-60
    Then output uses a stable constant frame rate
    And audio drift remains inside tolerance

  Scenario: Offline operation
    Given the device has no network
    When I import edit save reopen and export
    Then all core operations succeed

  Scenario: Link a project to its YouTube performance
    Given the build has a configured Google OAuth desktop client
    And I opened the Clip Forge project that produced an uploaded video
    When I connect a YouTube channel with both read-only scopes
    Then the panel polls and shows browser, token exchange, and channel-loading status
    And polling stops when the channel connects or the attempt fails
    When I select that exact video from recent uploads
    Then the video is accepted only if it belongs to the connected channel
    And the project-to-video link is stored outside the project file
    When I refresh the linked video
    Then the scorecard shows the defined aggregate owner metrics
    And the daily table maps values using YouTube's returned column headers
    And a new video with no rows shows a pending state
    And a disabled performance service names the YouTube Analytics API rather than the Data API
    And no OAuth token is exposed to the frontend
    When I disconnect and confirm clearing data
    Then the OS credential and cached YouTube performance data are removed
    And offline import edit save reopen and export still work

  Scenario: Discover Diablo IV clip moments locally
    Given I opened a source containing completion, death and boss-health-bar moments
    When I start clip discovery
    Then FFmpeg samples the authorized source with the fixed Diablo IV profile
    And progress is visible and cancellable
    And completion and death treatments require persistent title-screen evidence
    And a boss encounter requires a persistent top-HUD health bar wider than normal
    And every candidate includes an event time, suggested range, confidence and evidence
    And no candidate changes the project until I choose Use suggested range
    When I review a candidate
    Then I can inspect the source at its event time without changing the timeline
    When I use its suggested range
    Then the validated in and out points are applied and autosaved
    And sampled frames are not retained or uploaded
