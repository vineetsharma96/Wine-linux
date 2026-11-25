# Technical decisions

[Project decision records](research/TECHNICAL_DECISIONS.md)

## Stage 3.2

- **Built-in viewport.** RFB decodes into one CPU framebuffer. The Rust presenter copies a dirty generation directly into a checked ANativeWindow buffer. Canvas provides compatibility on the same transport. Android manages surface creation, destruction and producer coordination. [Architecture decision](research/DISPLAY_ARCHITECTURE_DECISION.md#stage-32-decision)

## Stage 3.1

- **APT source migration.** The guest package architecture and release gate a rewrite of recognized Ubuntu archive URLs. Ubuntu suites, components, signing options, and third-party mirrors remain under guest control. The first original is recoverable beside each changed file. [Guest APT research](research/ANDROID_LINUX_RESEARCH.md#stage-31-guest-package-sources)
- **Desktop lifecycle.** Rust scopes shutdown to the desktop launch that owns the PRoot child. Android activity changes may detach the viewport while the desktop remains available. [Session ownership](research/DISPLAY_ARCHITECTURE_DECISION.md#stage-31-session-ownership)
