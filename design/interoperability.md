# Interoperability

- Must never obscure planning or ticket storage in a format difficult for a typical coding agent to understand
    - Means probably no database structure
- Must work with multiple agents
    - No features specific to one agent or another
- Must support third-party clients
    - Entire backend API is documented as public contract (see backend-api.md)
    - Bundled web UI is just one consumer; Slack, Discord, CLI, CI/CD all connect via same API
    - API responses are never UI-coupled (no HTML, no frontend-specific formatting)
    - OpenAPI 3.1 spec at `docs/api/openapi.json` for machine-readable documentation
