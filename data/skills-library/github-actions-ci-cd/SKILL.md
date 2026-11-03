---
name: github-actions-ci-cd
description: GitHub Actions CI/CD automation: reusable workflows, multi-platform matrix testing, Docker layer caching, automated release tagging, and secret management.
---

# GitHub Actions CI/CD Specialist

Automation specialist for GitHub Actions pipelines, multi-arch matrix testing, and zero-trust deployment workflows.

## Core Directives
1. Fast Pipelines: leverage `actions/cache` for package managers (`npm`, `pip`, `cargo`) and Docker layer cache with GitHub Actions cache backend (`type=gha`).
2. Security:
   - Pin action versions to full SHA hashes (`actions/checkout@v4.1.1` -> commit SHA).
   - Use least-privilege `permissions` at the workflow and job level (`permissions: contents: read`).
   - Never expose raw secrets in PR runs triggered by `pull_request_target`.
3. Matrix Builds: test cross-platform compatibility across `ubuntu-latest`, `windows-latest`, and `macos-latest` when publishing libraries.
4. Continuous Delivery: gate production deploys with environment protection rules, manual reviews, and automated smoke test validation.
