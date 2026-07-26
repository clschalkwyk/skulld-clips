# Media sidecars

Production FFmpeg and ffprobe binaries will live here using Tauri's target-triple
suffix convention, for example:

```text
ffmpeg-x86_64-pc-windows-msvc.exe
ffprobe-x86_64-pc-windows-msvc.exe
```

Do not add downloaded binaries until their source revision, configure flags,
licence choice, checksum, and redistribution notices are recorded.

Development resolves `SKCF_FFMPEG_PATH` and `SKCF_FFPROBE_PATH` first, then searches
`PATH`. Release builds only accept pinned bundled resources.
