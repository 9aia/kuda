+++
id = "I-002"
title = "kuda sync regresses cards: acp leaks monologue into purpose, drops notes"
status = "open"
created = "2026-11-08"
priority = "high"
+++

Found during tamias-driver-kuda work (f0a4339). kuda sync with acp summarizer produced degraded cards for 8 stale modules: agent reasoning leaked into purpose field, notes: blocks emptied. Sync should preserve human-owned notes: regions and treat acp parse failures as fallback-to-template, not garbage-in-card. Also: installed ~/.local/bin/kuda is stale (predates records kind — drops docs/project cards); consider a version self-check.
