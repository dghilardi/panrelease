# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.12.x  | Yes                |
| < 0.12  | No                 |

## Reporting a Vulnerability

We take the security of Panrelease seriously. If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT open a public GitHub issue** for security vulnerabilities
2. **Email the maintainers** directly with details of the vulnerability
3. Include the following information:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Assessment**: We will assess the vulnerability and determine its impact within 1 week
- **Resolution**: We aim to release a fix within 2 weeks of confirming the vulnerability
- **Disclosure**: We will coordinate with you on the timing of public disclosure

### Scope

The following are in scope for security reports:

- Command injection via configuration files or CLI arguments
- Path traversal vulnerabilities
- Arbitrary file write/overwrite outside project boundaries
- Credential or secret exposure
- Supply chain concerns in dependencies

### Out of Scope

- Issues in upstream dependencies (please report those to the respective projects)
- Denial of service through large input files
- Issues requiring physical access to the machine

## Security Best Practices

When using Panrelease:

- Always review `.panproject.toml` files before running panrelease in unfamiliar repositories
- Keep Panrelease updated to the latest version
- Use GPG signing (`force_sign = true`) for release commits in production projects
- Review hook commands in configuration files, as they execute arbitrary shell commands

## Dependency Security

We regularly update dependencies to incorporate security patches. If you notice a vulnerable dependency, please let us know by opening an issue (for non-sensitive dependency updates) or via the private reporting process described above.
