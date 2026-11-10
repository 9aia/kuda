+++
id = "I-001"
title = "myco emits priority=medium in frontmatter but CLI enum is low|med|high|crit — normalize emitter to med (kuda schema accepts both for now)"
status = "done"
created = "2026-11-05"
priority = "med"
+++

# myco emits priority=medium in frontmatter but CLI enum is low|med|high|crit — normalize emitter to med (kuda schema accepts both for now)

Status: **open** · Created: 2026-11-05

## Description

What is wrong and steps to reproduce.

## Checklist

- [ ] Action item

## Resolution

fixed upstream in myco 5c96eff — emitter writes pri: tokens (med/crit); lenient read keeps medium/critical parsing; check --fix normalized myco/kuda/sturn ledgers (33/3/5 files). myco.toml schema enum dropped medium so drift flags again
