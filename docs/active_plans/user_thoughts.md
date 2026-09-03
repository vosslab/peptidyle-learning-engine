> **Historical discovery input, not current instructions.** Current authority is
> [implementation_plan.md](implementation_plan.md),
> [release_completion_plan.md](active/release_completion_plan.md), and
> [HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md). The M0 result is concluded evidence.

my plan was a web frontend server/container and db server/container; I dunno what is best 2026 design; probably run
everything using cloud containers like AWS; probably a separate webwork renderer server like adapt uses; the ability to
launch more servers for high demand should be considered

what is the modern LAMP setup? Linux, yes but in a container; Apache, python server, lighttpd, or keep apache; I am used
to Apache, but that seems dated at this point; M -> mariadb, postgres; sqlite: needs to be secure for FERPA; P ->
typescript/framework/wasm/rust?

Key principles:

- Dream big. Build on the ambition already present.
- Fix the design, not the symptom.
- Long-term over short-term.
- Best software design is focused on adaptability.

Question Types need to be plugin-like, so we can support any future type as well.
