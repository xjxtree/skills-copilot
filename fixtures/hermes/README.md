# Hermes Fixtures

These files are executable parser and scanner inputs for
`docs/adapters/hermes-adapter-spec.md`.

- `active-home/.hermes/skills/nested/research-brief/SKILL.md` covers nested
  active-home discovery.
- `active-home/.hermes/skills/broken/malformed-metadata/SKILL.md` covers a
  broken record without aborting the scan.
- `.env`, `auth.json`, `cron/jobs.json`, and `logs/session.log` under the same
  Hermes home are negative-scope fixtures. Ordinary skill scanning must not
  parse them.

Only the documented Hermes skill roots are parser inputs. Generic project
roots, cron records, credentials, logs, and unrelated config are not skills.
