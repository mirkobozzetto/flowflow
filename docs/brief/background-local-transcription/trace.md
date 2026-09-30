# Trace: background-local-transcription

Branch `feat/background-local-transcription` from `origin/dev`.

## 2026-10-01

- Code for T02, T03, T04 committed (3d97443): `BGContinuedProcessingTask`
  submitted on enqueue and retry of a local job, progress per chunk, CPU
  engine in the background, GPU back at the next chunk in the foreground.
- Neural Engine path not built: needs Core ML encoder models and the
  Background Inference entitlement. CPU first; T01 decides if it is enough.
- Mac smoke check (disposable, deleted): GPU 2.2 s, CPU 7.9 s per 60 s
  chunk on this Mac; GPU -> CPU -> GPU -> stop mid-CPU -> GPU, transcript
  complete and in order.
- Fixed on the way: whisper-rs 0.16 abort callback reads a capturing
  closure as the wrong type; only capture-free functions are passed.
- Installed on iPhone 16 (`make all`). Waiting: device test by Mirko
  (T01 speed numbers, T02-T05). Nothing pushed.
