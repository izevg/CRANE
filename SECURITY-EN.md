# Security Policy

[Русский](SECURITY.md) · **English**

CRANE sits in the path of every build, so we treat vulnerabilities in it as incidents. If you find one, please report it privately: that way users get the fix before attackers get the write-up.

## About the project and commitments

CRANE is pure OSS, forever: the project will never be sold. It has a single maintainer. There is no SLA, no guaranteed support period and no security team. Everything below is done on a best-effort basis, with no promises about timing.

## Supported versions

Security fixes go into the latest release. Backports to older versions are made when possible and are not guaranteed. Upgrades between versions: section 16.4.

## Reporting a vulnerability

**Do not disclose vulnerability details in public issues, pull requests, chats or social media.**

1. Send a private report through one of these channels:
   - the repository host's private vulnerability reporting (GitHub Private vulnerability reporting or a confidential issue on GitLab);
   - email to `security@<project domain>` (the address and PGP key will be published before the repository opens).
2. Please include:
   - the CRANE version and deployment method (Compose, Helm), relevant configuration;
   - steps to reproduce and, if you have one, a proof of concept;
   - what an attacker gains: read, write, package tampering, authorization bypass, denial of service;
   - how to reach you and how you would like to be credited (or that you prefer not to be).

## What happens next

The steps are these, with no promised deadlines: we acknowledge the report, assess severity (CVSS), fix, and publish an advisory.

Disclosure is coordinated: the advisory is published once a fix is available; by default no later than 90 days after the report unless we agree otherwise. A CVE ID is requested for each vulnerability. The advisory lists affected versions, the upgrade path, temporary mitigations and credit to the reporter.

## Scope

**In scope:**

- CRANE code: server, worker, analyzer, CLI, SPA;
- container images, the Helm chart, Compose files and CI templates from this repository;
- bypassing the access model: Cedar, tokens, separation of duties, the audit log;
- tampering with or corrupting packages, bypassing quarantine and promotion, dependency confusion through CRANE.

**Out of scope:**

- vulnerabilities in third-party components (PostgreSQL, Valkey, Keycloak, Trivy): report them upstream; report to us if CRANE uses a component insecurely;
- volumetric denial of service, social engineering, physical access;
- scanner output without a demonstrated impact;
- vulnerabilities in packages that CRANE users store: finding those is the job of the product's own vulnerability feature.

## Design stage

Architectural weaknesses can be discussed in the open: open an issue labeled `architecture`, see [CONTRIBUTING-EN.md](CONTRIBUTING-EN.md). The threat model and controls are described in [section 14](docs/product/14-security.md) (in Russian).

## Safe harbor

We will not pursue good-faith research as long as you:

- test only CRANE installations you own;
- do not access or modify other people's data;
- do not degrade service availability;
- give us reasonable time to fix the issue before publishing.

## Verifying releases

From v0.5.0 (when the repository goes public), releases are built in CI with SLSA provenance, cosign image signatures and an SBOM. CSAF advisories and VEX for each release are published when feasible. Verify the signature before deploying (section 14.3):

```bash
cosign verify <CRANE image> \
  --certificate-identity-regexp '<release pipeline identity>' \
  --certificate-oidc-issuer '<OIDC issuer>'
```

Exact values will be published with the first release.

## Cyber Resilience Act

CRANE is non-commercial free software and is not placed on the market in the course of a commercial activity (recital 18 of Regulation (EU) 2024/2847, the CRA). The project therefore does not assume manufacturer obligations. Whoever ships CRANE inside a commercial product evaluates their own obligations. This is not legal advice.
