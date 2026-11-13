---
id: INTEGRATIONS
status: accepted
owners: ["@k-dot-greyz"]
phase: 1
toolchain_floor: "1.100.0"
---

# Integrations

These are the edges the requirements talk about. None of them have a client in the tree.

| Edge | What phase 1 keeps | What it refuses |
| --- | --- | --- |
| Workspace host `gsuite.test` | Three principals: operator, radio, llm. One host. | A token from another host. A radio token on the model route. |
| Chat glass | A pointer card: block id, sha256, verdict. | Raw bytes, HTML, SVG, and any sentence treated as a command. |
| Drive inbox | A name for a later folder. A link is a string. | Fetching the file because the message contained an id. |
| Git installations | Three grants in `modules/profiles.json`. `gate::may_file` enforces them. | Cross-repo filing, `workflow`, `admin`, contents write. |
| Model route | Status table in FR-3. | A call without SSO-shaped session checks. A call that includes the plate. |
| Egress | The hull airlock. Destinations are an allowlist. Hull class wins. Dry dock sets `sent` to false. | Caller-supplied URLs, link-local, metadata hosts, plain http. |
| n8n | Absent. | A node, a credential, or a webhook "just for later". |

When an integration grows a client, the crate floor is recorded the day it is added. The client lives behind the profile that owns the verb. It does not get the process environment.
