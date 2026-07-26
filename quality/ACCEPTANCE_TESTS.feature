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
