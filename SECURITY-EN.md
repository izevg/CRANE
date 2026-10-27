# Security Policy

[Русский](SECURITY.md) · **English**

CRANE sits in the path of every build, so we treat vulnerabilities in it as incidents. If you find one, please report it privately: that way users get the fix before attackers get the write-up.

## Supported versions

| Version | Security fixes |
| --- | --- |
| Before the first release | Not applicable: there is no code yet, see "Design stage" below |
| Latest minor release | ✅ |
| Previous minor release | ✅ for 6 months after the next one ships |
| Older | ❌ please upgrade: upgrades skipping at most one minor version are supported (section 16.4) |

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

| Stage | Timeline |
| --- | --- |
| Acknowledgement | 3 business days |
| Initial assessment and severity (CVSS) | 7 business days |
| Fix for a critical issue | within 7 days |
| Fix for a high issue | within 30 days |
| Fix for a medium issue | within 90 days |
| Low | in the next scheduled release |

Disclosure is coordinated: we publish an advisory once a fix is available, and no later than 90 days after the report unless we agree otherwise. A CVE ID is requested for each vulnerability. The advisory lists affected versions, the upgrade path, temporary mitigations and credit to the reporter.

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

While there is no code, architectural weaknesses can be discussed in the open: open an issue labeled `architecture`, see [CONTRIBUTING-EN.md](CONTRIBUTING-EN.md). The threat model and controls are described in [section 14](docs/product/14-security.md) (in Russian).

## Safe harbor

We will not pursue good-faith research as long as you:

- test only CRANE installations you own;
- do not access or modify other people's data;
- do not degrade service availability;
- give us reasonable time to fix the issue before publishing.

## Verifying releases

Images are signed with cosign, and each one ships with an SBOM and build provenance (section 14.3). Verify the signature before deploying:

```bash
cosign verify <CRANE image> \
  --certificate-identity-regexp '<release pipeline identity>' \
  --certificate-oidc-issuer '<OIDC issuer>'
```

Exact values will be published with the first release.
