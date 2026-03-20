# Status Source Shape Audit

## Role In The Doc Stack

This document records the upstream status-source families the app depends on, the payload shapes we currently consume, and the known shape quirks that affect normalization and UI trust.

Use it alongside `docs/status-normalization-spec.md`.

## Audit Questions

- Which source family powers each tracked provider?
- Which endpoints do we hit for summary, incidents, components, and maintenance?
- Which detail classes are inline, enrichment-only, summary-only, or unsupported?
- Which live-shape quirks must the adapter preserve explicitly?

## Provider Matrix

| Provider | Family | Host Platform | Endpoint Pattern | Detail Coverage | Notes |
| --- | --- | --- | --- | --- | --- |
| `openai` | `IncidentIoShim` | incident.io | `summary.json` + `incidents.json` | components inline, incidents enrichment, maintenance inline | second incidents call required for reliable incident detail |
| `anthropic` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `openrouter` | `OnlineOrNot` | OnlineOrNot | `status_pages/<slug>/summary` | incidents inline, components inline, maintenance inline | `status` may be string or object |
| `google` / `gemini` | `GoogleCloudJson` | Google Cloud | `products.json` + `incidents.json` | adapter currently summary-focused | current adapter does not preserve raw incident detail |
| `moonshot` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `github` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `cursor` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `perplexity` | `Instatus` | Instatus | `summary.json` + `v2/components.json` | incidents inline, maintenance inline, components enrichment | healthy summary payloads can be sparse |
| `deepseek` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `gitlab` | `StatusIo` | Status.io | `api.status.io/.../status/<id>` | incidents inline, components inline, maintenance inline | source status code `400` means partial disruption |
| `poe` | `IncidentIoShim` | incident.io | `summary.json` + `incidents.json` | components inline, incidents enrichment, maintenance inline | same multi-fetch constraint as OpenAI |
| `nano-gpt` | `BetterStack` | Better Stack | `index.json` | components inline, incidents inline when present, maintenance unsupported | resources use `public_name` |
| `nvidia` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `vercel` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `helicone` | `BetterStack` | Better Stack | `index.json` | components inline, incidents inline when present, maintenance unsupported | resources use `public_name` |
| `groq` | `IncidentIoShim` | incident.io | `summary.json` + `incidents.json` | components inline, incidents enrichment, maintenance inline | second incidents call required |
| `cohere` | `IncidentIoShim` | incident.io | `summary.json` + `incidents.json` | components inline, incidents enrichment, maintenance inline | second incidents call required |
| `cerebras` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `cloudflare` | `StatuspageV2` | Statuspage | `api/v2/summary.json` | incidents inline, components inline, maintenance inline | rich official feed |
| `together-ai` | `BetterStack` | Better Stack | `index.json` | components inline, incidents inline when present, maintenance unsupported | live resources use `public_name` |
| `huggingface` | `BetterStack` | Better Stack | `index.json` | components inline, incidents inline when present, maintenance unsupported | live resources use `public_name` |
| fallback providers | `ApiStatusCheck` | apistatuscheck.com | `api/status?api=<slug>` | summary only | no raw incidents, components, or maintenance |

## Source Family Contracts

### `StatuspageV2`

- Primary shape: `{ page, status, incidents, components, scheduled_maintenances }`
- Best fit for the canonical model because all major detail classes are inline.
- Empty arrays can mean `NoneReported` because the source explicitly returned the section.

### `IncidentIoShim`

- Primary summary shape resembles Statuspage, but incident detail is not reliable from the first call alone.
- The adapter must treat incidents as enrichment-backed detail.
- `summary.json` without the second `incidents.json` call should not be presented as complete incident coverage.

### `BetterStack`

- Top-level shape: `{ data, included }`
- Overall status lives at `/data/attributes/aggregate_state`
- Components come from `included[type=status_page_resource]`
- Live resource labels use `attributes.public_name`
- Scheduled maintenance is not exposed in the current adapter contract

### `OnlineOrNot`

- Top-level shape: `{ success, messages, errors, result }`
- `result.status` may be either a string or an object with a description
- Components, active incidents, and scheduled maintenance are inline arrays

### `StatusIo`

- Top-level shape: `{ result: { status_overall, status, incidents, maintenance } }`
- Components are nested under `result.status[*].containers`
- Live source code `400` means partial disruption and must map to degraded health

### `Instatus`

- Summary endpoint can be sparse when healthy
- Components are fetched from a second endpoint: `v2/components.json`
- Summary payload alone is not enough to claim complete component coverage

### `GoogleCloudJson`

- Products catalog comes from `products.json`
- Incident feed comes from `incidents.json`
- The current adapter derives provider health and summary from the incident feed but does not preserve raw incident rows in `ProviderStatus`
- That limitation must be modeled as adapter behavior, not source absence

### `ApiStatusCheck`

- Top-level shape: `{ api, links }`
- Summary-only fallback
- Never treat it as equivalent to official rich-source detail

## Known Live Drift

| Family | Drift | Expected Adapter Behavior |
| --- | --- | --- |
| `BetterStack` | component name is `public_name`, not `resource_name` | prefer `public_name`, fall back to `resource_name` |
| `StatusIo` | uses status code `400` for partial disruption | map `400` to degraded / `degraded_performance` |
| `IncidentIoShim` | requires a second incidents feed | mark incidents as enrichment-backed and record failures explicitly |
| `Instatus` | healthy summary payloads can omit component detail | do not infer component absence from summary-only payloads |
| `GoogleCloudJson` | current adapter is intentionally lossy | model raw incident detail as not preserved, not unsupported by the source |

## Parser Risk Checklist

- Do not treat empty vectors as proof that the source lacks detail unless the detail state is `NoneReported`.
- Do not treat summary-only fallback data as evidence that incidents/components are healthy.
- Do not treat source-domain status codes as transport errors.
- Preserve multi-endpoint fetch outcomes so `FetchFailed` and `NotAttempted` stay visible to the UI.

## Current Conclusions

- Source families are not shape-compatible enough to infer trust from vectors alone.
- The normalization layer must model detail availability explicitly.
- The UI should render helper-driven meanings like `none reported`, `unsupported`, or `failed to load`, rather than guessing from empty arrays.
