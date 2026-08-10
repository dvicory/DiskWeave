# Encrypted OMP session evidence

`omp-session-evidence-aes256.zip` contains the raw OMP parent session,
subagent/artifact transcripts present at capture time, referenced OMP blobs,
a rendered transcript, runtime-version metadata, and internal SHA-256 hashes.

The ZIP uses AES-256 encryption.

The decryption password is intentionally not stored in this repository.
It is stored separately by the repository owner.

This artifact is post-hoc evaluation evidence. It must never be supplied to
an evaluated agent before the run being scored.

The raw contents may contain credentials, private paths, prompts, tool output,
or other sensitive data. Do not decrypt in an untrusted environment.
