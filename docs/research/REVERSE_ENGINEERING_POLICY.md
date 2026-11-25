# Permanent Reverse-Engineering Policy

## Scope & Evidence Hierarchy
Before implementing or redesigning any major subsystem, engineers and coding agents must investigate how established open-source projects solve the problem using **lawful, publicly available** sources in this priority order:
1. Official source code and build manifests.
2. Official technical documentation and release notes.
3. Public issue trackers, PRs, and architecture discussions.
4. Reproducible black-box experiments on our own hardware (e.g., Xiaomi Pad 7 `2410CRP4CI`) using `adb`, `dumpsys`, `logcat`, and package metadata.

All findings must clearly separate **Confirmed facts** (directly observed in source/device output) from **Strong hypotheses** and **Unknowns**.

## Legal & Security Boundaries
- Never decompile/steal proprietary code, bypass access controls, extract private keys/credentials, or access another app's private `/data/data/<pkg>` storage in production code.
- Always check upstream licenses (GPLv2/GPLv3, MIT, Apache-2.0, LGPL) and redistribution obligations before adopting or bundling any component.
